use std::net::ToSocketAddrs;

pub struct BufWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> BufWriter<'a> {
    #[inline]
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn write(&mut self, data: &[u8]) -> bool {
        if self.pos + data.len() > self.buf.len() {
            return false;
        }
        self.buf[self.pos..self.pos + data.len()].copy_from_slice(data);
        self.pos += data.len();
        true
    }

    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }
}

#[inline]
pub fn fmt_u64(mut val: u64, buf: &mut [u8; 20]) -> &[u8] {
    if val == 0 {
        buf[19] = b'0';
        return &buf[19..];
    }
    let mut pos = 20usize;
    while val > 0 {
        pos -= 1;
        buf[pos] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    &buf[pos..]
}

#[inline]
pub fn fmt_u32(mut val: u32, buf: &mut [u8; 20]) -> &[u8] {
    if val == 0 {
        buf[19] = b'0';
        return &buf[19..];
    }
    let mut pos = 20usize;
    while val > 0 {
        pos -= 1;
        buf[pos] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    &buf[pos..]
}

#[inline]
pub fn fmt_usize(mut val: usize, buf: &mut [u8; 20]) -> &[u8] {
    if val == 0 {
        buf[19] = b'0';
        return &buf[19..];
    }
    let mut pos = 20usize;
    while val > 0 {
        pos -= 1;
        buf[pos] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    &buf[pos..]
}

#[inline]
pub fn monotonic_ns() -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}

#[inline]
pub fn resolve_host(host: &str) -> Option<u32> {
    let addr_str = format!("{}:0", host);
    let mut addrs = addr_str.to_socket_addrs().ok()?;
    for addr in &mut addrs {
        if let core::net::SocketAddr::V4(v4) = addr {
            return Some(u32::from_ne_bytes(v4.ip().octets()));
        }
    }
    None
}
