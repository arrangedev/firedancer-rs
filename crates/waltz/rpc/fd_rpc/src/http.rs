use core::fmt;
use fd_rpc_sys as sys;

use crate::io::Connection;
use crate::utils::{self, BufWriter};

#[derive(Debug)]
pub enum HttpError {
    RequestTooLarge,
    ResponseMalformed,
    ResponseTooLarge,
    ResponseIncomplete,
    ConnectionClosed,
    BadStatus(u16),
    WriteFailed,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpError::RequestTooLarge => write!(f, "HTTP request too large for TX buffer"),
            HttpError::ResponseMalformed => write!(f, "malformed HTTP response"),
            HttpError::ResponseTooLarge => write!(f, "HTTP response too large for buffer"),
            HttpError::ResponseIncomplete => write!(f, "response timed out"),
            HttpError::ConnectionClosed => write!(f, "connection closed by peer"),
            HttpError::BadStatus(s) => write!(f, "HTTP status {}", s),
            HttpError::WriteFailed => write!(f, "failed to write HTTP request"),
        }
    }
}

impl core::error::Error for HttpError {}

pub struct HttpResponse<'a> {
    pub status: u16,
    pub body: &'a [u8],
}

pub fn write_request(
    conn: &mut Connection,
    method: &str,
    path: &str,
    host: &str,
    content_type: &str,
    body: &[u8],
) -> Result<(), HttpError> {
    let mut hdr_buf = [0u8; 512];
    let hdr_len = fmt_request_header(&mut hdr_buf, method, path, host, content_type, body.len())
        .ok_or(HttpError::RequestTooLarge)?;

    let rbuf = conn.rbuf_tx() as *mut sys::fd_h2_rbuf_t;
    let free = unsafe { sys::fd_h2_rbuf_free_sz(rbuf) } as usize;
    let total = hdr_len + body.len();
    if total > free {
        return Err(HttpError::RequestTooLarge);
    }

    conn.tx_push(&hdr_buf[..hdr_len]);
    if !body.is_empty() {
        conn.tx_push(body);
    }
    Ok(())
}

fn fmt_request_header(
    buf: &mut [u8],
    method: &str,
    path: &str,
    host: &str,
    content_type: &str,
    body_len: usize,
) -> Option<usize> {
    let mut w = BufWriter::new(buf);

    macro_rules! push {
        ($d:expr) => {
            if !w.write($d) {
                return None;
            }
        };
    }

    push!(method.as_bytes());
    push!(b" ");
    push!(path.as_bytes());
    push!(b" HTTP/1.1\r\nHost: ");
    push!(host.as_bytes());
    push!(b"\r\nContent-Type: ");
    push!(content_type.as_bytes());
    push!(b"\r\nContent-Length: ");

    let mut itoa_buf = [0u8; 20];
    push!(utils::fmt_usize(body_len, &mut itoa_buf));

    push!(b"\r\nConnection: keep-alive\r\n\r\n");

    Some(w.pos())
}

pub fn read_response<'a>(
    scratch: &'a mut [u8],
    conn: &mut Connection,
    timeout_ns: i64,
) -> Result<HttpResponse<'a>, HttpError> {
    let start = utils::monotonic_ns();
    let mut filled = 0usize;

    let header_info = loop {
        let r = conn.pump();
        if r.closed || r.error {
            return Err(HttpError::ConnectionClosed);
        }
        let n = conn.rx_pop(&mut scratch[filled..]);
        if n > 0 {
            let prev = filled;
            filled += n;
            match try_parse_headers(scratch, filled, prev) {
                HeaderResult::Complete(info) => break info,
                HeaderResult::Incomplete => {}
                HeaderResult::Error => return Err(HttpError::ResponseMalformed),
            }
            if filled >= scratch.len() {
                return Err(HttpError::ResponseTooLarge);
            }
        } else if timeout_ns >= 0 && (utils::monotonic_ns() - start) as i64 >= timeout_ns {
            return Err(HttpError::ResponseIncomplete);
        }
    };

    if let Some(cl) = header_info.content_length {
        let total = header_info.header_len + cl;
        while filled < total {
            if filled >= scratch.len() {
                return Err(HttpError::ResponseTooLarge);
            }
            let r = conn.pump();
            if r.closed || r.error {
                return Err(HttpError::ConnectionClosed);
            }
            let n = conn.rx_pop(&mut scratch[filled..]);
            if n > 0 {
                filled += n;
            } else if timeout_ns >= 0 && (utils::monotonic_ns() - start) as i64 >= timeout_ns {
                return Err(HttpError::ResponseIncomplete);
            }
        }
        Ok(HttpResponse {
            status: header_info.status,
            body: &scratch[header_info.header_len..total],
        })
    } else if header_info.is_chunked {
        read_chunked_body(scratch, filled, &header_info, conn, start, timeout_ns)
    } else {
        Ok(HttpResponse {
            status: header_info.status,
            body: &scratch[header_info.header_len..filled],
        })
    }
}

fn read_chunked_body<'a>(
    scratch: &'a mut [u8],
    filled: usize,
    info: &HeaderInfo,
    conn: &mut Connection,
    start: u64,
    timeout_ns: i64,
) -> Result<HttpResponse<'a>, HttpError> {
    let mut decoder: sys::phr_chunked_decoder = unsafe { core::mem::zeroed() };
    decoder.consume_trailer = 1;

    let body_start = info.header_len;
    let initial_raw = filled - body_start;
    let mut decoded_total: usize = 0;

    if initial_raw > 0 {
        let mut bufsz = initial_raw;
        let pret = unsafe {
            sys::phr_decode_chunked(
                &mut decoder,
                scratch.as_mut_ptr().add(body_start) as *mut libc::c_char,
                &mut bufsz,
            )
        };
        decoded_total = bufsz;
        if pret >= 0 {
            return Ok(HttpResponse {
                status: info.status,
                body: &scratch[body_start..body_start + decoded_total],
            });
        }
        if pret == -1 {
            return Err(HttpError::ResponseMalformed);
        }
    }

    loop {
        let r = conn.pump();
        if r.closed || r.error {
            return Err(HttpError::ConnectionClosed);
        }
        let write_at = body_start + decoded_total;
        let n = conn.rx_pop(&mut scratch[write_at..]);
        if n > 0 {
            let mut bufsz = n;
            let pret = unsafe {
                sys::phr_decode_chunked(
                    &mut decoder,
                    scratch.as_mut_ptr().add(write_at) as *mut libc::c_char,
                    &mut bufsz,
                )
            };
            decoded_total += bufsz;
            if pret >= 0 {
                return Ok(HttpResponse {
                    status: info.status,
                    body: &scratch[body_start..body_start + decoded_total],
                });
            }
            if pret == -1 {
                return Err(HttpError::ResponseMalformed);
            }
            if body_start + decoded_total >= scratch.len() {
                return Err(HttpError::ResponseTooLarge);
            }
        } else if timeout_ns >= 0 && (utils::monotonic_ns() - start) as i64 >= timeout_ns {
            return Err(HttpError::ResponseIncomplete);
        }
    }
}

struct HeaderInfo {
    header_len: usize,
    status: u16,
    content_length: Option<usize>,
    is_chunked: bool,
}

enum HeaderResult {
    Complete(HeaderInfo),
    Incomplete,
    Error,
}

fn try_parse_headers(buf: &[u8], filled: usize, last_len: usize) -> HeaderResult {
    let mut minor_version: libc::c_int = 0;
    let mut status: libc::c_int = 0;
    let mut msg: *const libc::c_char = core::ptr::null();
    let mut msg_len: usize = 0;
    let mut headers: [sys::phr_header; 32] = unsafe { core::mem::zeroed() };
    let mut num_headers: usize = 32;

    let rc = unsafe {
        sys::phr_parse_response(
            buf.as_ptr() as *const libc::c_char,
            filled,
            &mut minor_version,
            &mut status,
            &mut msg,
            &mut msg_len,
            headers.as_mut_ptr(),
            &mut num_headers,
            last_len,
        )
    };

    if rc == -2 {
        return HeaderResult::Incomplete;
    }
    if rc < 0 {
        return HeaderResult::Error;
    }

    let header_len = rc as usize;
    let mut content_length: Option<usize> = None;
    let mut is_chunked = false;

    for i in 0..num_headers {
        let h = &headers[i];
        if h.name.is_null() {
            continue;
        }
        let name = unsafe { core::slice::from_raw_parts(h.name as *const u8, h.name_len) };
        let val = unsafe { core::slice::from_raw_parts(h.value as *const u8, h.value_len) };
        if name.eq_ignore_ascii_case(b"content-length") {
            if let Ok(s) = core::str::from_utf8(val) {
                content_length = s.trim().parse().ok();
            }
        } else if name.eq_ignore_ascii_case(b"transfer-encoding") {
            if let Ok(s) = core::str::from_utf8(val) {
                is_chunked = s.trim().eq_ignore_ascii_case("chunked");
            }
        }
    }

    HeaderResult::Complete(HeaderInfo {
        header_len,
        status: status as u16,
        content_length,
        is_chunked,
    })
}
