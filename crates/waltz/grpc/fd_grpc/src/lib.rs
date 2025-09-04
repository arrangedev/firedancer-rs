//! Safe Rust API for Firedancer gRPC implementation
//!
//! This crate provides safe abstractions over the raw FFI bindings in `fd_grpc_sys`.
//! It implements a gRPC client for dispatching unary and server-streaming gRPC requests
//! over HTTP/2+TLS, designed for high-performance blockchain applications.
//!
//! ## Features
//!
//! - **Memory safe**: All unsafe operations are encapsulated with proper error handling
//! - **HTTP/2+TLS**: Built on top of HTTP/2 with OpenSSL TLS support
//! - **Protocol Buffers**: Integrated with nanopb for efficient message serialization
//! - **Connection management**: Automatic connection lifecycle and metrics
//! - **Streaming support**: Both unary and server-streaming requests
//! - **No std support**: Can be used in no_std environments with `alloc`
//!
//! ## Usage
//!
//! ### Basic Client Setup
//!
//! ```rust,no_run
//! use fd_grpc::{Client, DefaultCallbacks, Result};
//!
//! // Use default callbacks (or implement your own ClientCallbacks trait)
//! let callbacks = DefaultCallbacks;
//!
//! let mut client = Client::new(callbacks, 4096)?;
//! client.set_authority("api.example.com", 443)?;
//! client.set_version("my-app/1.0")?;
//! # Ok::<(), fd_grpc::Error>(())
//! ```
//!
//! ### Making Requests
//!
//! ```rust,no_run
//! # use fd_grpc::{Client, DefaultCallbacks, RequestContext};
//! # let callbacks = DefaultCallbacks;
//! # let mut client = Client::new(callbacks, 4096)?;
//! # let message_ptr = core::ptr::null();
//! # let descriptor = core::ptr::null();
//!
//! let stream_handle = client.request_start(
//!     "/package.Service/Method",
//!     RequestContext::new(42),
//!     descriptor,
//!     message_ptr,
//!     Some("auth_token"),
//! )?;
//!
//! // Set deadlines using the utility function
//! use core::time::Duration;
//! let header_deadline = fd_grpc::utils::duration_to_deadline_nanos(Duration::from_secs(5));
//! let response_deadline = fd_grpc::utils::duration_to_deadline_nanos(Duration::from_secs(30));
//! client.set_header_deadline(&stream_handle, header_deadline)?;
//! client.set_response_deadline(&stream_handle, response_deadline)?;
//! # Ok::<(), fd_grpc::Error>(())
//! ```

#![no_std]
extern crate alloc;

use alloc::{boxed::Box, string::String, vec::Vec};
use core::{fmt, mem::MaybeUninit, ptr, slice, time::Duration};
use fd_grpc_sys as sys;

/// Max size for a single gRPC message buffer
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024;
/// Max length for version strings
pub const MAX_VERSION_LEN: usize = sys::FD_GRPC_CLIENT_VERSION_LEN_MAX as usize;
/// Max number of concurrent streams
pub const MAX_STREAMS: usize = sys::FD_GRPC_CLIENT_MAX_STREAMS as usize;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Buffer is too small to hold the data
    BufferTooSmall,
    /// Invalid data format or content
    InvalidData(String),
    /// Connection is not established
    NotConnected,
    /// Request is blocked (e.g., handshake not complete)
    RequestBlocked,
    /// Invalid field or parameter
    InvalidField(String),
    /// Connection error
    ConnectionError(String),
    /// Stream error
    StreamError(String),
    /// Timeout occurred
    Timeout,
    /// Resource exhausted (e.g., too many streams)
    ResourceExhausted,
    /// Internal error
    InternalError(String),
    /// Null pointer encountered
    NullPointer,
    /// Invalid argument
    InvalidArgument(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BufferTooSmall => write!(f, "Buffer too small"),
            Error::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            Error::NotConnected => write!(f, "Connection not established"),
            Error::RequestBlocked => write!(f, "Request is blocked"),
            Error::InvalidField(msg) => write!(f, "Invalid field: {}", msg),
            Error::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            Error::StreamError(msg) => write!(f, "Stream error: {}", msg),
            Error::Timeout => write!(f, "Operation timed out"),
            Error::ResourceExhausted => write!(f, "Resource exhausted"),
            Error::InternalError(msg) => write!(f, "Internal error: {}", msg),
            Error::NullPointer => write!(f, "Null pointer"),
            Error::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    Ok = sys::FD_GRPC_STATUS_OK as isize,
    Cancelled = sys::FD_GRPC_STATUS_CANCELLED as isize,
    Unknown = sys::FD_GRPC_STATUS_UNKNOWN as isize,
    InvalidArgument = sys::FD_GRPC_STATUS_INVALID_ARGUMENT as isize,
    DeadlineExceeded = sys::FD_GRPC_STATUS_DEADLINE_EXCEEDED as isize,
    NotFound = sys::FD_GRPC_STATUS_NOT_FOUND as isize,
    AlreadyExists = sys::FD_GRPC_STATUS_ALREADY_EXISTS as isize,
    PermissionDenied = sys::FD_GRPC_STATUS_PERMISSION_DENIED as isize,
    ResourceExhausted = sys::FD_GRPC_STATUS_RESOURCE_EXHAUSTED as isize,
    FailedPrecondition = sys::FD_GRPC_STATUS_FAILED_PRECONDITION as isize,
    Aborted = sys::FD_GRPC_STATUS_ABORTED as isize,
    OutOfRange = sys::FD_GRPC_STATUS_OUT_OF_RANGE as isize,
    Unimplemented = sys::FD_GRPC_STATUS_UNIMPLEMENTED as isize,
    Internal = sys::FD_GRPC_STATUS_INTERNAL as isize,
    Unavailable = sys::FD_GRPC_STATUS_UNAVAILABLE as isize,
    DataLoss = sys::FD_GRPC_STATUS_DATA_LOSS as isize,
    Unauthenticated = sys::FD_GRPC_STATUS_UNAUTHENTICATED as isize,
}

impl StatusCode {
    pub fn from_raw(status: u32) -> Option<Self> {
        match status {
            sys::FD_GRPC_STATUS_OK => Some(Self::Ok),
            sys::FD_GRPC_STATUS_CANCELLED => Some(Self::Cancelled),
            sys::FD_GRPC_STATUS_UNKNOWN => Some(Self::Unknown),
            sys::FD_GRPC_STATUS_INVALID_ARGUMENT => Some(Self::InvalidArgument),
            sys::FD_GRPC_STATUS_DEADLINE_EXCEEDED => Some(Self::DeadlineExceeded),
            sys::FD_GRPC_STATUS_NOT_FOUND => Some(Self::NotFound),
            sys::FD_GRPC_STATUS_ALREADY_EXISTS => Some(Self::AlreadyExists),
            sys::FD_GRPC_STATUS_PERMISSION_DENIED => Some(Self::PermissionDenied),
            sys::FD_GRPC_STATUS_RESOURCE_EXHAUSTED => Some(Self::ResourceExhausted),
            sys::FD_GRPC_STATUS_FAILED_PRECONDITION => Some(Self::FailedPrecondition),
            sys::FD_GRPC_STATUS_ABORTED => Some(Self::Aborted),
            sys::FD_GRPC_STATUS_OUT_OF_RANGE => Some(Self::OutOfRange),
            sys::FD_GRPC_STATUS_UNIMPLEMENTED => Some(Self::Unimplemented),
            sys::FD_GRPC_STATUS_INTERNAL => Some(Self::Internal),
            sys::FD_GRPC_STATUS_UNAVAILABLE => Some(Self::Unavailable),
            sys::FD_GRPC_STATUS_DATA_LOSS => Some(Self::DataLoss),
            sys::FD_GRPC_STATUS_UNAUTHENTICATED => Some(Self::Unauthenticated),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Cancelled => "CANCELLED",
            Self::Unknown => "UNKNOWN",
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::DeadlineExceeded => "DEADLINE_EXCEEDED",
            Self::NotFound => "NOT_FOUND",
            Self::AlreadyExists => "ALREADY_EXISTS",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::ResourceExhausted => "RESOURCE_EXHAUSTED",
            Self::FailedPrecondition => "FAILED_PRECONDITION",
            Self::Aborted => "ABORTED",
            Self::OutOfRange => "OUT_OF_RANGE",
            Self::Unimplemented => "UNIMPLEMENTED",
            Self::Internal => "INTERNAL",
            Self::Unavailable => "UNAVAILABLE",
            Self::DataLoss => "DATA_LOSS",
            Self::Unauthenticated => "UNAUTHENTICATED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineKind {
    /// Deadline for receiving response headers
    Header = sys::FD_GRPC_DEADLINE_HEADER as isize,
    /// Deadline for end of response stream
    ResponseEnd = sys::FD_GRPC_DEADLINE_RX_END as isize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestContext {
    pub id: u64,
}

impl RequestContext {
    pub fn new(id: u64) -> Self {
        Self { id }
    }
}

#[derive(Debug, Clone)]
pub struct ResponseHeaders {
    /// HTTP/2 status code
    pub http_status: u32,
    /// Whether this is a gRPC protocol response
    pub is_grpc_proto: bool,
    /// gRPC status code
    pub grpc_status: Option<StatusCode>,
    /// gRPC status message
    pub grpc_message: Option<String>,
}

impl ResponseHeaders {
    pub(crate) fn from_raw(headers: &sys::fd_grpc_resp_hdrs_t) -> Self {
        let grpc_message = if headers.grpc_msg_len > 0 {
            let msg_bytes = unsafe {
                slice::from_raw_parts(
                    headers.grpc_msg.as_ptr() as *const u8,
                    headers.grpc_msg_len as usize,
                )
            };
            String::from_utf8_lossy(msg_bytes).into_owned().into()
        } else {
            None
        };

        Self {
            http_status: headers.h2_status,
            is_grpc_proto: headers.is_grpc_proto() != 0,
            grpc_status: if headers.grpc_status != 0 {
                StatusCode::from_raw(headers.grpc_status)
            } else {
                None
            },
            grpc_message,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClientMetrics {
    /// Number of times the client was polled for I/O
    pub wakeup_count: u64,
    /// Number of survivable stream errors
    pub stream_error_count: u64,
    /// Number of connection errors
    pub connection_error_count: u64,
    /// Number of transmitted data chunks
    pub stream_chunks_tx_count: u64,
    /// Number of transmitted bytes
    pub stream_chunks_tx_bytes: u64,
    /// Number of received data chunks
    pub stream_chunks_rx_count: u64,
    /// Number of received bytes
    pub stream_chunks_rx_bytes: u64,
    /// Number of requests sent
    pub requests_sent: u64,
    /// Number of active streams
    pub streams_active: i64,
    /// Cumulative RX wait time in ticks
    pub rx_wait_ticks_cumulative: i64,
    /// Cumulative TX wait time in ticks
    pub tx_wait_ticks_cumulative: i64,
}

impl ClientMetrics {
    fn from_raw(metrics: &sys::fd_grpc_client_metrics_t) -> Self {
        Self {
            wakeup_count: metrics.wakeup_cnt,
            stream_error_count: metrics.stream_err_cnt,
            connection_error_count: metrics.conn_err_cnt,
            stream_chunks_tx_count: metrics.stream_chunks_tx_cnt,
            stream_chunks_tx_bytes: metrics.stream_chunks_tx_bytes,
            stream_chunks_rx_count: metrics.stream_chunks_rx_cnt,
            stream_chunks_rx_bytes: metrics.stream_chunks_rx_bytes,
            requests_sent: metrics.requests_sent,
            streams_active: metrics.streams_active,
            rx_wait_ticks_cumulative: metrics.rx_wait_ticks_cum,
            tx_wait_ticks_cumulative: metrics.tx_wait_ticks_cum,
        }
    }
}

pub trait ClientCallbacks {
    /// Called when the HTTP/2 connection is established
    fn on_connection_established(&mut self) {}

    /// Called when the connection dies
    fn on_connection_dead(&mut self, h2_error: u32, closed_by: i32) {
        let _ = (h2_error, closed_by);
    }

    /// Called when a transmission is complete
    fn on_tx_complete(&mut self, request_ctx: RequestContext) {
        let _ = request_ctx;
    }

    /// Called when response headers are received
    fn on_rx_start(&mut self, request_ctx: RequestContext) {
        let _ = request_ctx;
    }

    /// Called when a message is received
    fn on_rx_message(&mut self, request_ctx: RequestContext, data: &[u8]) {
        let _ = (request_ctx, data);
    }

    /// Called when response stream ends
    fn on_rx_end(&mut self, request_ctx: RequestContext, headers: ResponseHeaders) {
        let _ = (request_ctx, headers);
    }

    /// Called when a request times out
    fn on_rx_timeout(&mut self, request_ctx: RequestContext, deadline_kind: DeadlineKind) {
        let _ = (request_ctx, deadline_kind);
    }

    /// Called when a ping acknowledgment is received
    fn on_ping_ack(&mut self) {}
}

#[derive(Debug, Default)]
pub struct DefaultCallbacks;

impl ClientCallbacks for DefaultCallbacks {}

#[derive(Debug)]
pub struct StreamHandle {
    stream: *mut sys::fd_grpc_h2_stream_t,
}

impl StreamHandle {
    fn new(stream: *mut sys::fd_grpc_h2_stream_t) -> Option<Self> {
        if stream.is_null() {
            None
        } else {
            Some(Self { stream })
        }
    }

    fn as_ptr(&self) -> *mut sys::fd_grpc_h2_stream_t {
        self.stream
    }
}

unsafe impl Send for StreamHandle {}
unsafe impl Sync for StreamHandle {}

struct CallbackContext<C: ClientCallbacks> {
    callbacks: C,
}

pub struct Client<C: ClientCallbacks> {
    client: *mut sys::fd_grpc_client_t,
    callback_ctx: *mut CallbackContext<C>,
    metrics: sys::fd_grpc_client_metrics_t,
    _memory: Vec<u8>,
}

impl<C: ClientCallbacks> Client<C> {
    pub fn new(callbacks: C, buffer_max: usize) -> Result<Self> {
        if buffer_max == 0 {
            return Err(Error::InvalidArgument("buffer_max must be > 0".into()));
        }

        let align = unsafe { sys::fd_grpc_client_align() };
        let footprint = unsafe { sys::fd_grpc_client_footprint(buffer_max as u64) };

        if footprint == 0 {
            return Err(Error::InternalError("Invalid footprint".into()));
        }

        let mut memory = Vec::with_capacity(footprint as usize + align as usize);
        memory.resize(footprint as usize + align as usize, 0);

        let aligned_ptr = {
            let ptr = memory.as_mut_ptr() as usize;
            let aligned = (ptr + align as usize - 1) & !(align as usize - 1);
            aligned as *mut u8
        };

        let metrics = MaybeUninit::<sys::fd_grpc_client_metrics_t>::zeroed();
        let metrics = unsafe { metrics.assume_init() };
        let callback_ctx = Box::into_raw(Box::new(CallbackContext { callbacks }));

        let sys_callbacks = sys::fd_grpc_client_callbacks_t {
            conn_established: Some(Self::conn_established_trampoline),
            conn_dead: Some(Self::conn_dead_trampoline),
            tx_complete: Some(Self::tx_complete_trampoline),
            rx_start: Some(Self::rx_start_trampoline),
            rx_msg: Some(Self::rx_msg_trampoline),
            rx_end: Some(Self::rx_end_trampoline),
            rx_timeout: Some(Self::rx_timeout_trampoline),
            ping_ack: Some(Self::ping_ack_trampoline),
        };

        let client = unsafe {
            sys::fd_grpc_client_new(
                aligned_ptr as *mut core::ffi::c_void,
                &sys_callbacks,
                &metrics as *const _ as *mut _,
                callback_ctx as *mut core::ffi::c_void,
                buffer_max as u64,
                0, // RNG seed
            )
        };

        if client.is_null() {
            unsafe {
                let _ = Box::from_raw(callback_ctx);
            }
            return Err(Error::InternalError("Failed to create gRPC client".into()));
        }

        Ok(Self {
            client,
            callback_ctx,
            metrics,
            _memory: memory,
        })
    }

    /// Set the authority (hostname and port) for requests
    pub fn set_authority(&mut self, host: &str, port: u16) -> Result<()> {
        if host.len() > 255 {
            return Err(Error::InvalidArgument("Host name too long".into()));
        }

        unsafe {
            sys::fd_grpc_client_set_authority(
                self.client,
                host.as_ptr() as *const i8,
                host.len() as u64,
                port,
            );
        }

        Ok(())
    }

    pub fn set_version(&mut self, version: &str) -> Result<()> {
        if version.len() > MAX_VERSION_LEN {
            return Err(Error::InvalidArgument("Version string too long".into()));
        }

        unsafe {
            sys::fd_grpc_client_set_version(
                self.client,
                version.as_ptr() as *const i8,
                version.len() as u64,
            );
        }

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        unsafe { sys::fd_grpc_client_is_connected(self.client) != 0 }
    }

    pub fn is_request_blocked(&self) -> bool {
        unsafe { sys::fd_grpc_client_request_is_blocked(self.client) != 0 }
    }

    pub fn request_start(
        &mut self,
        path: &str,
        request_ctx: RequestContext,
        fields: *const sys::pb_msgdesc_t,
        message: *const core::ffi::c_void,
        auth_token: Option<&str>,
    ) -> Result<StreamHandle> {
        if path.len() >= 128 {
            return Err(Error::InvalidArgument("Path too long".into()));
        }

        let (auth_ptr, auth_len) = match auth_token {
            Some(token) => (token.as_ptr() as *const i8, token.len() as u64),
            None => (ptr::null(), 0),
        };

        let stream = unsafe {
            sys::fd_grpc_client_request_start(
                self.client,
                path.as_ptr() as *const i8,
                path.len() as u64,
                request_ctx.id,
                fields,
                message,
                auth_ptr,
                auth_len,
            )
        };

        StreamHandle::new(stream).ok_or(Error::RequestBlocked)
    }

    pub fn set_deadline(
        &mut self,
        stream: &StreamHandle,
        deadline_kind: DeadlineKind,
        deadline_nanos: i64,
    ) -> Result<()> {
        unsafe {
            sys::fd_grpc_client_deadline_set(stream.as_ptr(), deadline_kind as i32, deadline_nanos);
        }
        Ok(())
    }

    pub fn set_header_deadline(
        &mut self,
        stream: &StreamHandle,
        deadline_nanos: i64,
    ) -> Result<()> {
        self.set_deadline(stream, DeadlineKind::Header, deadline_nanos)
    }

    pub fn set_response_deadline(
        &mut self,
        stream: &StreamHandle,
        deadline_nanos: i64,
    ) -> Result<()> {
        self.set_deadline(stream, DeadlineKind::ResponseEnd, deadline_nanos)
    }

    pub fn reset(&mut self) {
        unsafe {
            sys::fd_grpc_client_reset(self.client);
        }
    }

    pub fn metrics(&self) -> ClientMetrics {
        ClientMetrics::from_raw(&self.metrics)
    }

    unsafe extern "C" fn conn_established_trampoline(app_ctx: *mut core::ffi::c_void) {
        if app_ctx.is_null() {
            return;
        }

        let ctx = &mut *(app_ctx as *mut CallbackContext<C>);
        ctx.callbacks.on_connection_established();
    }

    unsafe extern "C" fn conn_dead_trampoline(
        app_ctx: *mut core::ffi::c_void,
        h2_err: u32,
        closed_by: i32,
    ) {
        if app_ctx.is_null() {
            return;
        }

        let ctx = &mut *(app_ctx as *mut CallbackContext<C>);
        ctx.callbacks.on_connection_dead(h2_err, closed_by);
    }

    unsafe extern "C" fn tx_complete_trampoline(app_ctx: *mut core::ffi::c_void, request_ctx: u64) {
        if app_ctx.is_null() {
            return;
        }

        let ctx = &mut *(app_ctx as *mut CallbackContext<C>);
        ctx.callbacks
            .on_tx_complete(RequestContext::new(request_ctx));
    }

    unsafe extern "C" fn rx_start_trampoline(app_ctx: *mut core::ffi::c_void, request_ctx: u64) {
        if app_ctx.is_null() {
            return;
        }

        let ctx = &mut *(app_ctx as *mut CallbackContext<C>);
        ctx.callbacks.on_rx_start(RequestContext::new(request_ctx));
    }

    unsafe extern "C" fn rx_msg_trampoline(
        app_ctx: *mut core::ffi::c_void,
        protobuf: *const core::ffi::c_void,
        protobuf_sz: u64,
        request_ctx: u64,
    ) {
        if app_ctx.is_null() || protobuf.is_null() {
            return;
        }

        let ctx = &mut *(app_ctx as *mut CallbackContext<C>);
        let data = slice::from_raw_parts(protobuf as *const u8, protobuf_sz as usize);
        ctx.callbacks
            .on_rx_message(RequestContext::new(request_ctx), data);
    }

    unsafe extern "C" fn rx_end_trampoline(
        app_ctx: *mut core::ffi::c_void,
        request_ctx: u64,
        resp: *mut sys::fd_grpc_resp_hdrs_t,
    ) {
        if app_ctx.is_null() || resp.is_null() {
            return;
        }

        let ctx = &mut *(app_ctx as *mut CallbackContext<C>);
        let headers = ResponseHeaders::from_raw(&*resp);
        ctx.callbacks
            .on_rx_end(RequestContext::new(request_ctx), headers);
    }

    unsafe extern "C" fn rx_timeout_trampoline(
        app_ctx: *mut core::ffi::c_void,
        request_ctx: u64,
        deadline_kind: i32,
    ) {
        if app_ctx.is_null() {
            return;
        }

        let ctx = &mut *(app_ctx as *mut CallbackContext<C>);
        let deadline = match deadline_kind as u32 {
            sys::FD_GRPC_DEADLINE_HEADER => DeadlineKind::Header,
            sys::FD_GRPC_DEADLINE_RX_END => DeadlineKind::ResponseEnd,
            _ => return,
        };
        ctx.callbacks
            .on_rx_timeout(RequestContext::new(request_ctx), deadline);
    }

    unsafe extern "C" fn ping_ack_trampoline(app_ctx: *mut core::ffi::c_void) {
        if app_ctx.is_null() {
            return;
        }

        let ctx = &mut *(app_ctx as *mut CallbackContext<C>);
        ctx.callbacks.on_ping_ack();
    }
}

impl<C: ClientCallbacks> Drop for Client<C> {
    fn drop(&mut self) {
        if !self.client.is_null() {
            unsafe {
                sys::fd_grpc_client_delete(self.client);
            }
        }

        if !self.callback_ctx.is_null() {
            unsafe {
                let _ = Box::from_raw(self.callback_ctx);
            }
        }
    }
}

unsafe impl<C: ClientCallbacks + Send> Send for Client<C> {}
unsafe impl<C: ClientCallbacks + Sync> Sync for Client<C> {}

pub mod utils {
    use super::*;

    pub fn status_code_str(status: u32) -> &'static str {
        unsafe {
            let ptr = sys::fd_grpc_status_cstr(status);
            if ptr.is_null() {
                "UNKNOWN"
            } else {
                let cstr = core::ffi::CStr::from_ptr(ptr);
                cstr.to_str().unwrap_or("INVALID")
            }
        }
    }

    pub fn duration_to_deadline_nanos(duration: Duration) -> i64 {
        let now_nanos = current_time_nanos();
        now_nanos.saturating_add(duration.as_nanos() as i64)
    }

    fn current_time_nanos() -> i64 {
        #[cfg(target_os = "macos")]
        {
            use core::mem::MaybeUninit;
            extern "C" {
                fn mach_absolute_time() -> u64;
                fn mach_timebase_info(info: *mut mach_timebase_info_t) -> i32;
            }

            #[repr(C)]
            struct mach_timebase_info_t {
                numer: u32,
                denom: u32,
            }

            unsafe {
                let mut timebase = MaybeUninit::<mach_timebase_info_t>::uninit();
                if mach_timebase_info(timebase.as_mut_ptr()) == 0 {
                    let timebase = timebase.assume_init();
                    let ticks = mach_absolute_time();
                    ((ticks as u128 * timebase.numer as u128) / timebase.denom as u128) as i64
                } else {
                    0
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            use core::mem::MaybeUninit;

            // clock_gettime - CLOCK_MONOTONIC
            extern "C" {
                fn clock_gettime(clock_id: i32, tp: *mut timespec) -> i32;
            }

            #[repr(C)]
            struct timespec {
                tv_sec: i64,
                tv_nsec: i64,
            }

            const CLOCK_MONOTONIC: i32 = 1;

            unsafe {
                let mut ts = MaybeUninit::<timespec>::uninit();
                if clock_gettime(CLOCK_MONOTONIC, ts.as_mut_ptr()) == 0 {
                    let ts = ts.assume_init();
                    ts.tv_sec * 1_000_000_000 + ts.tv_nsec
                } else {
                    0
                }
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            use core::sync::atomic::{AtomicI64, Ordering};
            static COUNTER: AtomicI64 = AtomicI64::new(1_000_000_000_000);
            COUNTER.fetch_add(1_000_000, Ordering::SeqCst)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_code_conversion() {
        assert_eq!(
            StatusCode::from_raw(sys::FD_GRPC_STATUS_OK),
            Some(StatusCode::Ok)
        );
        assert_eq!(
            StatusCode::from_raw(sys::FD_GRPC_STATUS_CANCELLED),
            Some(StatusCode::Cancelled)
        );
        assert_eq!(StatusCode::from_raw(999), None);
    }

    #[test]
    fn test_status_code_strings() {
        assert_eq!(StatusCode::Ok.as_str(), "OK");
        assert_eq!(StatusCode::InvalidArgument.as_str(), "INVALID_ARGUMENT");
        assert_eq!(StatusCode::Internal.as_str(), "INTERNAL");
    }

    #[test]
    fn test_request_context() {
        let ctx = RequestContext::new(42);
        assert_eq!(ctx.id, 42);
    }

    #[test]
    fn test_deadline_kind() {
        assert_eq!(
            DeadlineKind::Header as i32,
            sys::FD_GRPC_DEADLINE_HEADER as i32
        );
        assert_eq!(
            DeadlineKind::ResponseEnd as i32,
            sys::FD_GRPC_DEADLINE_RX_END as i32
        );
    }

    #[test]
    fn test_client_creation() {
        let callbacks = DefaultCallbacks;
        let result = Client::new(callbacks, 4096);
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_creation_invalid_buffer() {
        let callbacks = DefaultCallbacks;
        let result = Client::new(callbacks, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_utils_status_str() {
        let status_str = utils::status_code_str(sys::FD_GRPC_STATUS_OK);
        assert!(!status_str.is_empty());
    }

    #[test]
    fn test_duration_to_deadline_nanos() {
        let duration = Duration::from_millis(100);
        let deadline = utils::duration_to_deadline_nanos(duration);

        assert!(deadline > 0);

        // monotonicity
        let deadline1 = utils::duration_to_deadline_nanos(Duration::from_millis(50));
        let deadline2 = utils::duration_to_deadline_nanos(Duration::from_millis(50));

        assert!(deadline2 >= deadline1);
    }

    #[test]
    fn test_error_display() {
        let errors = [
            Error::BufferTooSmall,
            Error::InvalidData("test".into()),
            Error::NotConnected,
            Error::RequestBlocked,
            Error::Timeout,
            Error::NullPointer,
        ];

        for error in &errors {
            let display = alloc::format!("{}", error);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_callback_trait() {
        struct TestCallbacks {
            called: bool,
        }

        impl ClientCallbacks for TestCallbacks {
            fn on_connection_established(&mut self) {
                self.called = true;
            }

            fn on_rx_message(&mut self, _ctx: RequestContext, data: &[u8]) {
                assert!(!data.is_empty());
            }
        }

        let mut callbacks = TestCallbacks { called: false };
        callbacks.on_connection_established();
        assert!(callbacks.called);

        callbacks.on_rx_message(RequestContext::new(42), &[1, 2, 3]);
    }

    #[test]
    fn test_default_callbacks() {
        let mut callbacks = DefaultCallbacks;

        callbacks.on_connection_established();
        callbacks.on_connection_dead(0, 0);
        callbacks.on_tx_complete(RequestContext::new(1));
        callbacks.on_rx_start(RequestContext::new(2));
        callbacks.on_rx_message(RequestContext::new(3), &[1, 2, 3]);
        callbacks.on_rx_end(
            RequestContext::new(4),
            ResponseHeaders {
                http_status: 200,
                is_grpc_proto: true,
                grpc_status: Some(StatusCode::Ok),
                grpc_message: None,
            },
        );
        callbacks.on_rx_timeout(RequestContext::new(5), DeadlineKind::Header);
        callbacks.on_ping_ack();
    }
}
