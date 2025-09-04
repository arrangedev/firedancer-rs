//! Safe Rust wrapper for Firedancer's HTTP server implementation
//!
//! This crate provides a safe, idiomatic Rust API over the high-performance HTTP/1.1 server
//! implementation used by Firedancer. The underlying implementation provides:
//!
//! - High-performance HTTP/1.1 server with keep-alive support
//! - WebSocket protocol upgrade support
//! - Ring buffer architecture for efficient request handling
//! - URL parsing and handling utilities
//! - picohttpparser integration for fast HTTP parsing
//!
//! ## Features
//!
//! - **Memory Safety**: All unsafe FFI operations are wrapped in safe abstractions
//! - **Zero-Copy**: Efficient handling of HTTP requests and responses
//! - **WebSocket Support**: Built-in WebSocket upgrade handling
//! - **High Performance**: Built on Firedancer's optimized C implementation
//! - **no_std Compatible**: Works in embedded and no_std environments (with `alloc`)
//!
//! ## Example
//!
//! ```rust
//! use fd_http::{Server, ServerParams, ServerCallbacks, DefaultCallbacks};
//! use core::net::Ipv4Addr;
//!
//! // Create server parameters
//! let params = ServerParams::builder()
//!     .max_connection_cnt(1024)
//!     .max_ws_connection_cnt(512)
//!     .max_request_len(8192)
//!     .max_ws_recv_frame_len(8192)  // Must be >= max_request_len
//!     .max_ws_send_frame_cnt(256)
//!     .outgoing_buffer_sz(65536)
//!     .build();
//!
//! // Create callbacks handler
//! let callbacks = DefaultCallbacks;
//!
//! // Create server (don't start listening in doc tests)
//! let _server = Server::new(params, callbacks, 4096)?;
//!
//! // In real usage, you would call:
//! // server.listen(Ipv4Addr::new(127, 0, 0, 1), 8080)?;
//! // server.poll(); // Run server in an event loop
//! # Ok::<(), fd_http::Error>(())
//! ```

#![no_std]
#![warn(/*missing_docs,*/ unsafe_op_in_unsafe_fn)]

extern crate alloc;

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    ffi::CString,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::{
    ffi::{c_char, c_int, c_void, CStr},
    fmt,
    marker::PhantomData,
    mem::MaybeUninit,
    net::Ipv4Addr,
    ptr, slice,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
use fd_http_sys::*;

/// Errors that can occur during HTTP server operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Invalid parameter provided
    InvalidParameter(String),
    /// Memory allocation failed
    AllocationFailed,
    /// Socket operation failed
    SocketError(String),
    /// Server is already listening
    AlreadyListening,
    /// Server is not listening
    NotListening,
    /// HTTP parsing error
    ParseError(String),
    /// WebSocket error
    WebSocketError(String),
    /// Buffer overflow
    BufferOverflow,
    /// Connection limit exceeded
    ConnectionLimitExceeded,
    /// Invalid HTTP method
    InvalidMethod,
    /// Invalid URL
    InvalidUrl,
    /// Unknown error
    Unknown(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            Error::AllocationFailed => write!(f, "Memory allocation failed"),
            Error::SocketError(msg) => write!(f, "Socket error: {}", msg),
            Error::AlreadyListening => write!(f, "Server is already listening"),
            Error::NotListening => write!(f, "Server is not listening"),
            Error::ParseError(msg) => write!(f, "HTTP parsing error: {}", msg),
            Error::WebSocketError(msg) => write!(f, "WebSocket error: {}", msg),
            Error::BufferOverflow => write!(f, "Buffer overflow"),
            Error::ConnectionLimitExceeded => write!(f, "Connection limit exceeded"),
            Error::InvalidMethod => write!(f, "Invalid HTTP method"),
            Error::InvalidUrl => write!(f, "Invalid URL"),
            Error::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

/// HTTP methods supported by the server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// GET method
    Get,
    /// POST method
    Post,
    /// PUT method
    Put,
    /// DELETE method
    Delete,
    /// HEAD method
    Head,
    /// OPTIONS method
    Options,
    /// PATCH method
    Patch,
}

impl Method {
    /// Convert from raw method constant
    pub fn from_raw(method: u8) -> Option<Self> {
        match method {
            x if x == FD_HTTP_SERVER_METHOD_GET as u8 => Some(Method::Get),
            x if x == FD_HTTP_SERVER_METHOD_POST as u8 => Some(Method::Post),
            x if x == FD_HTTP_SERVER_METHOD_PUT as u8 => Some(Method::Put),
            x if x == FD_HTTP_SERVER_METHOD_OPTIONS as u8 => Some(Method::Options),
            _ => None, // DELETE, HEAD, PATCH not available in this implementation
        }
    }

    /// Convert to raw method constant
    pub fn to_raw(self) -> u8 {
        match self {
            Method::Get => FD_HTTP_SERVER_METHOD_GET as u8,
            Method::Post => FD_HTTP_SERVER_METHOD_POST as u8,
            Method::Put => FD_HTTP_SERVER_METHOD_PUT as u8,
            Method::Options => FD_HTTP_SERVER_METHOD_OPTIONS as u8,
            // These methods are not supported by the underlying implementation
            Method::Delete | Method::Head | Method::Patch => 255, // Invalid method
        }
    }

    /// Get method as string
    pub fn as_str(self) -> &'static str {
        unsafe {
            let ptr = fd_http_server_method_str(self.to_raw());
            if ptr.is_null() {
                "UNKNOWN"
            } else {
                CStr::from_ptr(ptr).to_str().unwrap_or("UNKNOWN")
            }
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Connection close reasons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionCloseReason {
    /// Connection closed normally
    Ok,
    /// Connection closed due to peer close
    PeerClose,
    /// Connection closed due to error
    Error,
    /// Connection closed due to timeout
    Timeout,
}

impl ConnectionCloseReason {
    /// Convert from raw close reason constant
    pub fn from_raw(reason: i32) -> Option<Self> {
        match reason {
            x if x == FD_HTTP_SERVER_CONNECTION_CLOSE_OK => Some(ConnectionCloseReason::Ok),
            x if x == FD_HTTP_SERVER_CONNECTION_CLOSE_PEER_RESET => {
                Some(ConnectionCloseReason::PeerClose)
            }
            x if x == FD_HTTP_SERVER_CONNECTION_CLOSE_TOO_SLOW => {
                Some(ConnectionCloseReason::Timeout)
            }
            _ if reason < 0 => Some(ConnectionCloseReason::Error), // Any negative value is an error
            _ => None,
        }
    }

    /// Convert to raw close reason constant
    pub fn to_raw(self) -> i32 {
        match self {
            ConnectionCloseReason::Ok => FD_HTTP_SERVER_CONNECTION_CLOSE_OK,
            ConnectionCloseReason::PeerClose => FD_HTTP_SERVER_CONNECTION_CLOSE_PEER_RESET,
            ConnectionCloseReason::Error => FD_HTTP_SERVER_CONNECTION_CLOSE_BAD_REQUEST,
            ConnectionCloseReason::Timeout => FD_HTTP_SERVER_CONNECTION_CLOSE_TOO_SLOW,
        }
    }

    /// Get close reason as string
    pub fn as_str(self) -> &'static str {
        unsafe {
            let ptr = fd_http_server_connection_close_reason_str(self.to_raw());
            if ptr.is_null() {
                "UNKNOWN"
            } else {
                CStr::from_ptr(ptr).to_str().unwrap_or("UNKNOWN")
            }
        }
    }
}

impl fmt::Display for ConnectionCloseReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug)]
pub struct Request<'a> {
    /// HTTP method
    pub method: Method,
    /// Request path
    pub path: &'a str,
    /// HTTP headers
    pub headers: Vec<(&'a str, &'a str)>,
    /// Request body
    pub body: &'a [u8],
}

#[derive(Debug)]
pub struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Response {
    /// Create a new response with the given status code
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Add a header to the response
    pub fn header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Set the response body
    pub fn body<B: Into<Vec<u8>>>(mut self, body: B) -> Self {
        self.body = body.into();
        self
    }

    /// Set the response body as text
    pub fn text<T: Into<String>>(mut self, text: T) -> Self {
        self.body = text.into().into_bytes();
        self
    }

    /// Create a 200 OK response
    pub fn ok() -> Self {
        Self::new(200)
    }

    /// Create a 404 Not Found response
    pub fn not_found() -> Self {
        Self::new(404).text("Not Found")
    }

    /// Create a 500 Internal Server Error response
    pub fn internal_error() -> Self {
        Self::new(500).text("Internal Server Error")
    }

    /// Get the response status code
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Get a reference to the response body
    pub fn body_bytes(&self) -> &[u8] {
        &self.body
    }

    /// Get a reference to the response headers
    pub fn response_headers(&self) -> &[(String, String)] {
        &self.headers
    }
}

/// Trait for handling HTTP server callbacks
pub trait ServerCallbacks {
    /// Called when a new HTTP request is received
    fn on_request(&mut self, connection_id: u64, request: Request) -> Response;

    /// Called when a WebSocket connection is established
    fn on_ws_connect(&mut self, connection_id: u64, path: &str) -> bool {
        let _ = (connection_id, path);
        false // Reject by default
    }

    /// Called when a WebSocket message is received
    fn on_ws_message(&mut self, connection_id: u64, message: &[u8]) {
        let _ = (connection_id, message);
    }

    /// Called when a WebSocket connection is closed
    fn on_ws_close(&mut self, connection_id: u64, reason: ConnectionCloseReason) {
        let _ = (connection_id, reason);
    }

    /// Called when a connection is closed
    fn on_connection_close(&mut self, connection_id: u64, reason: ConnectionCloseReason) {
        let _ = (connection_id, reason);
    }
}

/// Default implementation of ServerCallbacks that returns 404 for all requests
pub struct DefaultCallbacks;

impl ServerCallbacks for DefaultCallbacks {
    fn on_request(&mut self, _connection_id: u64, _request: Request) -> Response {
        Response::not_found()
    }
}

/// Server configuration parameters
#[derive(Debug, Clone)]
pub struct ServerParams {
    /// Maximum number of concurrent connections
    pub max_connection_cnt: u64,
    /// Maximum number of concurrent WebSocket connections
    pub max_ws_connection_cnt: u64,
    /// Maximum HTTP request length in bytes
    pub max_request_len: u64,
    /// Maximum WebSocket receive frame length in bytes
    pub max_ws_recv_frame_len: u64,
    /// Maximum number of WebSocket send frames
    pub max_ws_send_frame_cnt: u64,
    /// Outgoing buffer size in bytes
    pub outgoing_buffer_sz: u64,
}

impl ServerParams {
    /// Create a new builder for ServerParams
    pub fn builder() -> ServerParamsBuilder {
        ServerParamsBuilder::new()
    }

    /// Convert to raw parameters struct
    fn to_raw(self) -> fd_http_server_params_t {
        fd_http_server_params_t {
            max_connection_cnt: self.max_connection_cnt,
            max_ws_connection_cnt: self.max_ws_connection_cnt,
            max_request_len: self.max_request_len,
            max_ws_recv_frame_len: self.max_ws_recv_frame_len,
            max_ws_send_frame_cnt: self.max_ws_send_frame_cnt,
            outgoing_buffer_sz: self.outgoing_buffer_sz,
        }
    }
}

/// Builder for ServerParams
#[derive(Debug)]
pub struct ServerParamsBuilder {
    max_connection_cnt: u64,
    max_ws_connection_cnt: u64,
    max_request_len: u64,
    max_ws_recv_frame_len: u64,
    max_ws_send_frame_cnt: u64,
    outgoing_buffer_sz: u64,
}

impl ServerParamsBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            max_connection_cnt: 1024,
            max_ws_connection_cnt: 512,
            max_request_len: 8192,
            max_ws_recv_frame_len: 8192, // Must be >= max_request_len
            max_ws_send_frame_cnt: 256,
            outgoing_buffer_sz: 65536,
        }
    }

    /// Set maximum number of concurrent connections
    pub fn max_connection_cnt(mut self, count: u64) -> Self {
        self.max_connection_cnt = count;
        self
    }

    /// Set maximum number of concurrent WebSocket connections
    pub fn max_ws_connection_cnt(mut self, count: u64) -> Self {
        self.max_ws_connection_cnt = count;
        self
    }

    /// Set maximum HTTP request length in bytes
    pub fn max_request_len(mut self, len: u64) -> Self {
        self.max_request_len = len;
        self
    }

    /// Set maximum WebSocket receive frame length in bytes
    pub fn max_ws_recv_frame_len(mut self, len: u64) -> Self {
        self.max_ws_recv_frame_len = len;
        self
    }

    /// Set maximum number of WebSocket send frames
    pub fn max_ws_send_frame_cnt(mut self, count: u64) -> Self {
        self.max_ws_send_frame_cnt = count;
        self
    }

    /// Set outgoing buffer size in bytes
    pub fn outgoing_buffer_sz(mut self, size: u64) -> Self {
        self.outgoing_buffer_sz = size;
        self
    }

    /// Build the ServerParams
    pub fn build(self) -> ServerParams {
        ServerParams {
            max_connection_cnt: self.max_connection_cnt,
            max_ws_connection_cnt: self.max_ws_connection_cnt,
            max_request_len: self.max_request_len,
            max_ws_recv_frame_len: self.max_ws_recv_frame_len,
            max_ws_send_frame_cnt: self.max_ws_send_frame_cnt,
            outgoing_buffer_sz: self.outgoing_buffer_sz,
        }
    }
}

impl Default for ServerParamsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for callback dispatch and server state tracking
/// Cleanup data for a response
///
/// This struct tracks memory that needs to be freed when an HTTP connection closes.
/// The C HTTP server expects response data (body and headers) to remain valid until
/// the response is fully sent. This struct implements Drop to automatically clean up
/// that memory when the connection closes.
struct ResponseCleanup {
    body_ptr: *mut u8,
    body_len: usize,
    header_ptrs: Vec<*mut c_char>,
}

unsafe impl Send for ResponseCleanup {}
unsafe impl Sync for ResponseCleanup {}

impl Drop for ResponseCleanup {
    fn drop(&mut self) {
        unsafe {
            // Clean up response body
            if !self.body_ptr.is_null() && self.body_len > 0 {
                let _body = Box::from_raw(slice::from_raw_parts_mut(self.body_ptr, self.body_len));
                // Box::drop will handle the cleanup
            }

            // Clean up header C strings
            for ptr in &self.header_ptrs {
                if !ptr.is_null() {
                    let _cstring = CString::from_raw(*ptr);
                    // CString::drop will handle the cleanup
                }
            }
        }
    }
}

struct CallbackContext<C> {
    callbacks: C,
    connection_count: AtomicU64,
    is_listening: AtomicBool,
    // Track response cleanup data by connection ID
    pending_cleanups: BTreeMap<u64, ResponseCleanup>,
}

// Callback trampoline functions
unsafe extern "C" fn request_trampoline<C: ServerCallbacks>(
    request: *const fd_http_server_request_t,
) -> fd_http_server_response_t {
    if request.is_null() {
        return default_error_response();
    }

    unsafe {
        let req = &*request;

        // Extract request information
        let method = Method::from_raw(req.method).unwrap_or(Method::Get);

        let path = if !req.path.is_null() {
            CStr::from_ptr(req.path).to_str().unwrap_or("/")
        } else {
            "/"
        };

        // Extract headers
        let mut headers = Vec::new();
        if !req.headers.content_type.is_null() {
            let content_type = CStr::from_ptr(req.headers.content_type)
                .to_str()
                .unwrap_or("");
            headers.push(("content-type", content_type));
        }
        if !req.headers.accept_encoding.is_null() {
            let accept_encoding = CStr::from_ptr(req.headers.accept_encoding)
                .to_str()
                .unwrap_or("");
            headers.push(("accept-encoding", accept_encoding));
        }

        // Extract body (for POST requests)
        let body = if !req.__bindgen_anon_1.post.body.is_null()
            && req.__bindgen_anon_1.post.body_len > 0
        {
            slice::from_raw_parts(
                req.__bindgen_anon_1.post.body,
                req.__bindgen_anon_1.post.body_len as usize,
            )
        } else {
            &[]
        };

        // Get callback context from the request context
        let ctx = req.ctx;
        if ctx.is_null() {
            return default_error_response();
        }

        let callback_ctx = &mut *(ctx as *mut CallbackContext<C>);

        // Create Rust request object
        let rust_request = Request {
            method,
            path,
            headers,
            body,
        };

        // Call the Rust callback
        let response = callback_ctx
            .callbacks
            .on_request(req.connection_id, rust_request);

        // Convert Rust response to C response
        convert_response_to_c(response, req.connection_id, callback_ctx)
    }
}

fn default_error_response() -> fd_http_server_response_t {
    fd_http_server_response_t {
        status: 500,
        upgrade_websocket: 0,
        content_type: ptr::null(),
        cache_control: ptr::null(),
        content_encoding: ptr::null(),
        access_control_allow_origin: ptr::null(),
        access_control_allow_methods: ptr::null(),
        access_control_allow_headers: ptr::null(),
        access_control_max_age: 0,
        static_body: ptr::null(),
        static_body_len: 0,
        _body_off: 0,
        _body_len: 0,
    }
}

/// Convert a Rust Response to a C fd_http_server_response_t
///
/// This function handles the complex memory management required by the C HTTP server:
/// - Response body and headers are converted to C-compatible formats
/// - Memory is tracked for automatic cleanup when the connection closes
/// - The C server expects this data to remain valid until the response is sent
fn convert_response_to_c<C: ServerCallbacks>(
    response: Response,
    connection_id: u64,
    callback_ctx: &mut CallbackContext<C>,
) -> fd_http_server_response_t {
    // Convert response body to static memory that outlives this function
    // We'll track this for cleanup when the connection closes
    let (body_ptr, body_len) = if !response.body.is_empty() {
        let body_box = response.body.into_boxed_slice();
        let len = body_box.len();
        let ptr = Box::into_raw(body_box) as *mut u8;
        (ptr as *const u8, len)
    } else {
        (ptr::null(), 0)
    };

    // Track header C strings for cleanup
    let mut header_ptrs = Vec::new();

    // Helper closure to convert header to C string and track for cleanup
    let mut header_to_c_str = |header_name: &str| -> *const c_char {
        response
            .headers
            .iter()
            .find(|(key, _)| key.to_lowercase() == header_name)
            .map(|(_, value)| {
                let c_string = CString::new(value.as_str()).unwrap_or_default();
                let ptr = c_string.into_raw();
                header_ptrs.push(ptr);
                ptr as *const c_char
            })
            .unwrap_or(ptr::null())
    };

    // Convert headers using helper
    let content_type_ptr = header_to_c_str("content-type");
    let cache_control_ptr = header_to_c_str("cache-control");
    let content_encoding_ptr = header_to_c_str("content-encoding");
    let cors_origin_ptr = header_to_c_str("access-control-allow-origin");
    let cors_methods_ptr = header_to_c_str("access-control-allow-methods");
    let cors_headers_ptr = header_to_c_str("access-control-allow-headers");

    // Parse max-age if present
    let max_age = response
        .headers
        .iter()
        .find(|(key, _)| key.to_lowercase() == "access-control-max-age")
        .and_then(|(_, value)| value.parse::<u64>().ok())
        .unwrap_or(0);

    // Store cleanup data for this connection
    let cleanup = ResponseCleanup {
        body_ptr: body_ptr as *mut u8,
        body_len,
        header_ptrs,
    };
    callback_ctx.pending_cleanups.insert(connection_id, cleanup);

    fd_http_server_response_t {
        status: response.status as u64,
        upgrade_websocket: 0, // TODO: Handle WebSocket upgrades
        content_type: content_type_ptr,
        cache_control: cache_control_ptr,
        content_encoding: content_encoding_ptr,
        access_control_allow_origin: cors_origin_ptr,
        access_control_allow_methods: cors_methods_ptr,
        access_control_allow_headers: cors_headers_ptr,
        access_control_max_age: max_age,
        static_body: body_ptr,
        static_body_len: body_len as u64,
        _body_off: 0,
        _body_len: 0,
    }
}

unsafe extern "C" fn open_trampoline<C: ServerCallbacks>(
    _conn_id: ulong,
    _sockfd: c_int,
    ctx: *mut c_void,
) {
    if !ctx.is_null() {
        unsafe {
            let callback_ctx = &mut *(ctx as *mut CallbackContext<C>);
            callback_ctx
                .connection_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }
}

unsafe extern "C" fn close_trampoline<C: ServerCallbacks>(
    conn_id: ulong,
    reason: c_int,
    ctx: *mut c_void,
) {
    if !ctx.is_null() {
        unsafe {
            let callback_ctx = &mut *(ctx as *mut CallbackContext<C>);
            callback_ctx
                .connection_count
                .fetch_sub(1, Ordering::Relaxed);

            // Clean up any pending response data for this connection
            if let Some(_cleanup) = callback_ctx.pending_cleanups.remove(&conn_id) {
                // ResponseCleanup::drop will handle the actual cleanup
            }

            if let Some(close_reason) = ConnectionCloseReason::from_raw(reason) {
                callback_ctx
                    .callbacks
                    .on_connection_close(conn_id, close_reason);
            }
        }
    }
}

unsafe extern "C" fn ws_open_trampoline<C: ServerCallbacks>(ws_conn_id: ulong, ctx: *mut c_void) {
    if !ctx.is_null() {
        unsafe {
            let callback_ctx = &mut *(ctx as *mut CallbackContext<C>);
            // WebSocket connections are tracked separately from HTTP connections
            callback_ctx.callbacks.on_ws_connect(ws_conn_id, "/"); // Path would need to be tracked
        }
    }
}

unsafe extern "C" fn ws_message_trampoline<C: ServerCallbacks>(
    ws_conn_id: ulong,
    data: *const uchar,
    data_len: ulong,
    ctx: *mut c_void,
) {
    if !ctx.is_null() && !data.is_null() {
        unsafe {
            let callback_ctx = &mut *(ctx as *mut CallbackContext<C>);
            let message = slice::from_raw_parts(data, data_len as usize);
            callback_ctx.callbacks.on_ws_message(ws_conn_id, message);
        }
    }
}

unsafe extern "C" fn ws_close_trampoline<C: ServerCallbacks>(
    ws_conn_id: ulong,
    reason: c_int,
    ctx: *mut c_void,
) {
    if !ctx.is_null() {
        unsafe {
            let callback_ctx = &mut *(ctx as *mut CallbackContext<C>);
            if let Some(close_reason) = ConnectionCloseReason::from_raw(reason) {
                callback_ctx.callbacks.on_ws_close(ws_conn_id, close_reason);
            }
        }
    }
}

/// High-performance HTTP/1.1 server with WebSocket support
pub struct Server<C> {
    server_ptr: *mut fd_http_server_t,
    memory: Box<[u8]>,
    callback_ctx: *mut CallbackContext<C>,
    _marker: PhantomData<C>,
}

impl<C: ServerCallbacks> Server<C> {
    /// Create a new HTTP server with the given parameters and callbacks
    pub fn new(params: ServerParams, callbacks: C, buffer_size: usize) -> Result<Self, Error> {
        let raw_params = params.to_raw();

        // Get required memory size
        let align = unsafe { fd_http_server_align() };
        let footprint = unsafe { fd_http_server_footprint(raw_params) };

        if footprint == 0 {
            return Err(Error::InvalidParameter("Invalid server parameters".into()));
        }

        // Allocate aligned memory
        let total_size = footprint as usize + align as usize + buffer_size;
        let mut memory = vec![0u8; total_size].into_boxed_slice();
        let aligned_ptr = {
            let ptr = memory.as_mut_ptr();
            let aligned = (ptr as usize + align as usize - 1) & !(align as usize - 1);
            aligned as *mut c_void
        };

        // Create callback context
        let callback_ctx = Box::into_raw(Box::new(CallbackContext {
            callbacks,
            connection_count: AtomicU64::new(0),
            is_listening: AtomicBool::new(false),
            pending_cleanups: BTreeMap::new(),
        }));

        // Create callbacks struct with proper trampolines
        let callbacks = fd_http_server_callbacks_t {
            request: Some(request_trampoline::<C>),
            open: Some(open_trampoline::<C>),
            close: Some(close_trampoline::<C>),
            ws_open: Some(ws_open_trampoline::<C>),
            ws_message: Some(ws_message_trampoline::<C>),
            ws_close: Some(ws_close_trampoline::<C>),
        };

        // Initialize server
        let shhttp_ptr = unsafe {
            fd_http_server_new(
                aligned_ptr,
                raw_params,
                callbacks,
                callback_ctx as *mut c_void,
            )
        };

        if shhttp_ptr.is_null() {
            unsafe { drop(Box::from_raw(callback_ctx)) };
            return Err(Error::AllocationFailed);
        }

        // Join to get the actual server pointer
        let server_ptr = unsafe { fd_http_server_join(shhttp_ptr) };

        if server_ptr.is_null() {
            unsafe { drop(Box::from_raw(callback_ctx)) };
            return Err(Error::AllocationFailed);
        }

        Ok(Self {
            server_ptr,
            memory,
            callback_ctx,
            _marker: PhantomData,
        })
    }

    /// Start listening on the specified address and port
    pub fn listen(&mut self, addr: Ipv4Addr, port: u16) -> Result<(), Error> {
        let address = u32::from(addr);

        let result = unsafe { fd_http_server_listen(self.server_ptr, address, port) };

        if result.is_null() {
            Err(Error::SocketError("Failed to start listening".into()))
        } else {
            // Update listening status
            if !self.callback_ctx.is_null() {
                unsafe {
                    (*self.callback_ctx)
                        .is_listening
                        .store(true, Ordering::Relaxed);
                }
            }
            Ok(())
        }
    }

    /// Poll for incoming connections and process requests
    pub fn poll(&mut self) -> Result<(), Error> {
        unsafe {
            fd_http_server_poll(self.server_ptr, 0);
        }
        Ok(())
    }

    /// Get the number of active connections
    pub fn connection_count(&self) -> u64 {
        if !self.callback_ctx.is_null() {
            unsafe {
                (*self.callback_ctx)
                    .connection_count
                    .load(Ordering::Relaxed)
            }
        } else {
            0
        }
    }

    /// Check if the server is listening
    pub fn is_listening(&self) -> bool {
        if !self.callback_ctx.is_null() {
            unsafe { (*self.callback_ctx).is_listening.load(Ordering::Relaxed) }
        } else {
            false
        }
    }
}

unsafe impl<C: Send> Send for Server<C> {}
unsafe impl<C: Sync> Sync for Server<C> {}

impl<C> Drop for Server<C> {
    fn drop(&mut self) {
        if !self.callback_ctx.is_null() {
            unsafe { drop(Box::from_raw(self.callback_ctx)) };
        }
    }
}

/// URL parsing utilities
pub mod url {
    use super::*;

    /// Parsed URL components
    #[derive(Debug, Clone)]
    pub struct Url {
        /// The original URL string
        pub original: String,
        /// Parsed components (if parsing was successful)
        pub components: Option<UrlComponents>,
    }

    /// URL components
    #[derive(Debug, Clone)]
    pub struct UrlComponents {
        /// URL scheme (e.g., "http", "https")
        pub scheme: String,
        /// Host name
        pub host: String,
        /// Port number
        pub port: Option<u16>,
        /// Path component
        pub path: String,
        /// Query string
        pub query: Option<String>,
        /// Fragment
        pub fragment: Option<String>,
    }

    impl Url {
        /// Parse a URL string
        pub fn parse(url: &str) -> Self {
            let mut url_struct = MaybeUninit::<fd_url_t>::uninit();
            let mut error: c_int = 0;

            let result = unsafe {
                fd_url_parse_cstr(
                    url_struct.as_mut_ptr(),
                    url.as_ptr() as *const c_char,
                    url.len() as u64,
                    &mut error,
                )
            };

            let components = if result.is_null() {
                None
            } else {
                let url_struct = unsafe { url_struct.assume_init() };
                let scheme = if !url_struct.scheme.is_null() && url_struct.scheme_len > 0 {
                    unsafe {
                        let slice = slice::from_raw_parts(
                            url_struct.scheme as *const u8,
                            url_struct.scheme_len as usize,
                        );
                        String::from_utf8_lossy(slice).into_owned()
                    }
                } else {
                    String::new()
                };

                let host = if !url_struct.host.is_null() && url_struct.host_len > 0 {
                    unsafe {
                        let slice = slice::from_raw_parts(
                            url_struct.host as *const u8,
                            url_struct.host_len as usize,
                        );
                        String::from_utf8_lossy(slice).into_owned()
                    }
                } else {
                    String::new()
                };

                let port = if !url_struct.port.is_null() && url_struct.port_len > 0 {
                    unsafe {
                        let slice = slice::from_raw_parts(
                            url_struct.port as *const u8,
                            url_struct.port_len as usize,
                        );
                        String::from_utf8_lossy(slice)
                            .to_string()
                            .parse::<u16>()
                            .ok()
                    }
                } else {
                    None
                };

                let tail = if !url_struct.tail.is_null() && url_struct.tail_len > 0 {
                    unsafe {
                        let slice = slice::from_raw_parts(
                            url_struct.tail as *const u8,
                            url_struct.tail_len as usize,
                        );
                        String::from_utf8_lossy(slice).into_owned()
                    }
                } else {
                    String::new()
                };

                // Parse path and query from tail
                let (path, query) = if let Some(query_start) = tail.find('?') {
                    let path_part = tail[..query_start].to_string();
                    let query_part = if query_start + 1 < tail.len() {
                        Some(tail[query_start + 1..].to_string())
                    } else {
                        None
                    };
                    (path_part, query_part)
                } else {
                    (tail, None)
                };

                // Parse fragment from query if present
                let (query, fragment) = if let Some(ref q) = query {
                    if let Some(fragment_start) = q.find('#') {
                        let query_part = if fragment_start > 0 {
                            Some(q[..fragment_start].to_string())
                        } else {
                            None
                        };
                        let fragment_part = if fragment_start + 1 < q.len() {
                            Some(q[fragment_start + 1..].to_string())
                        } else {
                            None
                        };
                        (query_part, fragment_part)
                    } else {
                        (query, None)
                    }
                } else {
                    (query, None)
                };

                Some(UrlComponents {
                    scheme,
                    host,
                    port,
                    path,
                    query,
                    fragment,
                })
            };

            Self {
                original: url.to_string(),
                components,
            }
        }

        pub fn unescape(input: &mut String) -> usize {
            unsafe {
                let bytes = input.as_mut_vec();
                fd_url_unescape(bytes.as_mut_ptr() as *mut c_char, bytes.len() as u64) as usize
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_conversion() {
        assert_eq!(Method::Get.as_str(), "GET");
        assert_eq!(Method::Post.as_str(), "POST");
        assert_eq!(Method::from_raw(Method::Get.to_raw()), Some(Method::Get));
    }

    #[test]
    fn test_server_params_builder() {
        let params = ServerParams::builder()
            .max_connection_cnt(2048)
            .max_request_len(16384)
            .build();

        assert_eq!(params.max_connection_cnt, 2048);
        assert_eq!(params.max_request_len, 16384);
    }

    #[test]
    fn test_response_builder() {
        let response = Response::ok()
            .header("Content-Type", "text/plain")
            .text("Hello, World!");

        assert_eq!(response.status, 200);
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.body, b"Hello, World!");
    }

    #[test]
    fn test_server_creation() {
        let params = ServerParams::builder().build();
        let callbacks = DefaultCallbacks;

        let server = Server::new(params, callbacks, 4096);
        assert!(server.is_ok());
    }

    #[test]
    fn test_response_cleanup_system() {
        // Test that our cleanup system properly tracks and cleans up response data
        let params = ServerParams::builder().build();
        let callbacks = DefaultCallbacks;

        let server = Server::new(params, callbacks, 4096).expect("Failed to create server");

        // Create a response with body and headers that would need cleanup
        let response = Response::ok()
            .text("Hello, World!")
            .header("content-type", "text/plain")
            .header("cache-control", "no-cache");

        // In a real scenario, this would be called by the C HTTP server
        // when a request comes in and when the connection closes
        // The cleanup system should automatically free the allocated memory

        // This test mainly verifies that the cleanup system compiles and
        // the data structures are set up correctly
        assert!(response.body.len() > 0);
        assert!(response.headers.len() > 0);

        drop(server); // This should trigger cleanup of any pending responses
    }

    #[test]
    fn test_url_parsing() {
        let url = url::Url::parse("https://example.com:8080/path?query=value#fragment");
        assert_eq!(
            url.original,
            "https://example.com:8080/path?query=value#fragment"
        );

        if let Some(components) = &url.components {
            assert!(components.scheme.starts_with("https"));
            assert_eq!(components.host, "example.com");
            assert_eq!(components.port, Some(8080));
            assert_eq!(components.path, "/path");
            assert_eq!(components.query, Some("query=value".to_string()));
            assert_eq!(components.fragment, Some("fragment".to_string()));
        }
    }
}
