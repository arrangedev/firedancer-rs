//! Safe API for `fd_quic_sys`

use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ffi::c_void;
use std::marker::PhantomData;
use std::net::{SocketAddrV4, UdpSocket};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[cfg(target_os = "linux")]
use std::ffi::CString;

#[derive(Debug, Clone)]
pub enum NetworkBackend {
    Xdp {
        xsk_map_fd: i32,
        prog_link_fd: i32,
        interface_name: String,
        listen_ports: Vec<u16>,
    },
    Udp {
        // TODO: mutexes are gay
        socket: Arc<Mutex<UdpSocket>>,
    },
    Stub,
}

pub struct NetworkContext {
    pub backend: NetworkBackend,
}

pub struct ConnectionTracker {
    pub new_connections: Arc<Mutex<Vec<*mut fd_quic_sys::fd_quic_conn_t>>>,
}

unsafe impl Send for ConnectionTracker {}
unsafe impl Sync for ConnectionTracker {}

pub struct ServerCallbackContext {
    pub connection_tracker: Arc<ConnectionTracker>,
}

unsafe impl Send for ServerCallbackContext {}
unsafe impl Sync for ServerCallbackContext {}

unsafe extern "C" fn conn_new_cb(_conn: *mut fd_quic_sys::fd_quic_conn_t, quic_ctx: *mut c_void) {
    if !quic_ctx.is_null() {
        let ctx = &*(quic_ctx as *const ServerCallbackContext);
        if let Ok(mut connections) = ctx.connection_tracker.new_connections.lock() {
            connections.push(_conn);
        }
    }
}

unsafe extern "C" fn conn_handshake_complete_cb(
    _conn: *mut fd_quic_sys::fd_quic_conn_t,
    _quic_ctx: *mut c_void,
) {
}

unsafe extern "C" fn conn_final_cb(
    _conn: *mut fd_quic_sys::fd_quic_conn_t,
    _quic_ctx: *mut c_void,
) {
}

unsafe extern "C" fn stream_notify_cb(
    _stream: *mut fd_quic_sys::fd_quic_stream_t,
    _stream_ctx: *mut c_void,
    _notify_type: i32,
) {
}

unsafe extern "C" fn stream_rx_cb(
    _conn: *mut fd_quic_sys::fd_quic_conn_t,
    _stream_id: u64,
    _offset: u64,
    _data: *const u8,
    _data_sz: u64,
    _fin: i32,
) -> i32 {
    0
}

unsafe extern "C" fn now_cb(_ctx: *mut c_void) -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

unsafe extern "C" fn _send_via_aio_be(
    ctx: *mut c_void,
    batch: *const fd_quic_sys::fd_aio_pkt_info,
    batch_cnt: u64,
    _opt_batch_idx: *mut u64,
    _flush: i32,
) -> i32 {
    if ctx.is_null() || batch.is_null() {
        return 0;
    }

    let net_ctx = &*(ctx as *const NetworkContext);

    match &net_ctx.backend {
        NetworkBackend::Xdp { .. } => batch_cnt as i32,
        NetworkBackend::Udp { socket } => {
            let socket = match socket.lock() {
                Ok(socket) => socket,
                Err(_) => return 0,
            };

            let mut sent_count = 0;
            for i in 0..batch_cnt {
                let pkt = &*batch.add(i as usize);
                if pkt.buf.is_null() || pkt.buf_sz == 0 {
                    continue;
                }

                let packet_data =
                    core::slice::from_raw_parts(pkt.buf as *const u8, pkt.buf_sz as usize);

                if let Ok(_) = socket.send(packet_data) {
                    sent_count += 1;
                }
            }
            sent_count
        }
        NetworkBackend::Stub => batch_cnt as i32,
    }
}

#[derive(Debug)]
pub enum QuicError {
    ConnectionFailed(String),
    StreamError(String),
    TlsError(String),
    ConfigError(String),
    Io(std::io::Error),
    Timeout,
    InvalidState(String),
    AllocationFailed,
    Internal(String),
}

impl std::fmt::Display for QuicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuicError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            QuicError::StreamError(msg) => write!(f, "Stream error: {}", msg),
            QuicError::TlsError(msg) => write!(f, "TLS error: {}", msg),
            QuicError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            QuicError::Io(err) => write!(f, "I/O error: {}", err),
            QuicError::Timeout => write!(f, "Timeout occurred"),
            QuicError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            QuicError::AllocationFailed => write!(f, "Memory allocation failed"),
            QuicError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for QuicError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QuicError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for QuicError {
    fn from(err: std::io::Error) -> Self {
        QuicError::Io(err)
    }
}

pub type Result<T> = std::result::Result<T, QuicError>;

pub mod config {

    #[derive(Clone, Debug)]
    pub struct Limits {
        /// max concurrent connections
        pub conn_cnt: usize,
        /// max concurrent handshakes
        pub handshake_cnt: usize,
        /// log cache depth
        pub log_depth: usize,
        /// connection_id count per connection (min 4)
        pub conn_id_cnt: usize,
        /// max concurrent stream_ids per connection
        pub stream_id_cnt: usize,
        /// total max inflight frame count
        pub inflight_frame_cnt: usize,
        /// min inflight frame count per connection
        pub min_inflight_frame_cnt_conn: usize,
        /// transmit buffer size per stream (bytes)
        pub tx_buf_sz: usize,
        /// number of streams in stream_pool
        pub stream_pool_cnt: usize,
    }

    impl Default for Limits {
        fn default() -> Self {
            Self {
                conn_cnt: 16,
                handshake_cnt: 8,
                log_depth: 1024,
                conn_id_cnt: 4,
                stream_id_cnt: 64,
                inflight_frame_cnt: 256,
                min_inflight_frame_cnt_conn: 16,
                tx_buf_sz: 65536,
                stream_pool_cnt: 128,
            }
        }
    }

    impl From<Limits> for fd_quic_sys::fd_quic_limits_t {
        fn from(limits: Limits) -> Self {
            fd_quic_sys::fd_quic_limits_t {
                conn_cnt: limits.conn_cnt as u64,
                handshake_cnt: limits.handshake_cnt as u64,
                log_depth: limits.log_depth as u64,
                conn_id_cnt: limits.conn_id_cnt as u64,
                stream_id_cnt: limits.stream_id_cnt as u64,
                inflight_frame_cnt: limits.inflight_frame_cnt as u64,
                min_inflight_frame_cnt_conn: limits.min_inflight_frame_cnt_conn as u64,
                tx_buf_sz: limits.tx_buf_sz as u64,
                stream_pool_cnt: limits.stream_pool_cnt as u64,
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Role {
        Client,
        Server,
    }

    impl From<Role> for i32 {
        fn from(role: Role) -> Self {
            match role {
                Role::Client => fd_quic_sys::FD_QUIC_ROLE_CLIENT as i32,
                Role::Server => fd_quic_sys::FD_QUIC_ROLE_SERVER as i32,
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct Config {
        /// client or server
        pub role: Role,
        /// enable address validation with retry packets
        pub retry: bool,
        /// clock ticks per micro
        pub tick_per_us: f64,
        /// idle timeout in nanos
        pub idle_timeout: u64,
        /// PING frames
        pub keep_alive: bool,
        /// ack delay in nanos
        pub ack_delay: u64,
        pub ack_threshold: u64,
        /// retry token ttl in nanos
        pub retry_ttl: u64,
        /// tls handshake ttl in nanos
        pub tls_hs_ttl: u64,
        pub initial_rx_max_stream_data: u64,
        /// differentiated services code point
        pub dscp: u8,
        /// Ed25519 id key
        pub identity_public_key: [u8; 32],
        pub keylog_file: Option<String>,
    }

    impl Default for Config {
        fn default() -> Self {
            let mut identity_key = [0u8; 32];
            for (i, byte) in identity_key.iter_mut().enumerate() {
                *byte = (i + 1) as u8;
            }

            Self {
                role: Role::Client,
                retry: false,
                tick_per_us: 1000.0,
                idle_timeout: 1_000_000_000,
                keep_alive: false,
                ack_delay: 50_000_000,
                ack_threshold: 65536,
                retry_ttl: 1_000_000_000,
                tls_hs_ttl: 3_000_000_000,
                initial_rx_max_stream_data: 65536,
                dscp: 0,
                identity_public_key: identity_key,
                keylog_file: None,
            }
        }
    }
}

#[repr(C)]
pub struct Quic {
    quic: *mut fd_quic_sys::fd_quic_t,
    mem: *mut u8,
    layout: Layout,
    _marker: PhantomData<fd_quic_sys::fd_quic_t>,
}

unsafe impl Send for Quic {}
unsafe impl Sync for Quic {}

impl Quic {
    pub fn new(limits: config::Limits) -> Result<Self> {
        let limits_sys: fd_quic_sys::fd_quic_limits_t = limits.into();

        unsafe {
            let align = fd_quic_sys::fd_quic_align() as usize;
            let footprint = fd_quic_sys::fd_quic_footprint(&limits_sys) as usize;

            if footprint == 0 {
                return Err(QuicError::ConfigError("invalid limits".to_string()));
            }

            let layout = Layout::from_size_align(footprint, align)
                .map_err(|_| QuicError::AllocationFailed)?;

            let mem = alloc_zeroed(layout);
            if mem.is_null() {
                return Err(QuicError::AllocationFailed);
            }

            let quic = fd_quic_sys::fd_quic_new(mem as *mut _, &limits_sys);
            if quic.is_null() {
                dealloc(mem, layout);
                return Err(QuicError::Internal(
                    "QUIC initialization failed".to_string(),
                ));
            }

            Ok(Quic {
                quic: quic as *mut fd_quic_sys::fd_quic_t,
                mem,
                layout,
                _marker: PhantomData,
            })
        }
    }

    pub fn join(&mut self) -> Result<QuicHandle> {
        unsafe {
            let handle = fd_quic_sys::fd_quic_join(self.quic as *mut _);
            if handle.is_null() {
                return Err(QuicError::Internal(
                    "failed to join QUIC instance".to_string(),
                ));
            }
            Ok(QuicHandle {
                quic: handle,
                _marker: PhantomData,
            })
        }
    }

    /// SAFETY: The caller must ensure that the returned pointer is not used after the
    /// Quic is dropped, and that any operations on it are thread-safe.
    pub unsafe fn as_raw(&self) -> *mut fd_quic_sys::fd_quic_t {
        self.quic
    }
}

impl Drop for Quic {
    fn drop(&mut self) {
        unsafe {
            let _returned_mem = fd_quic_sys::fd_quic_delete(self.quic);
            dealloc(self.mem, self.layout);
        }
    }
}

#[repr(C)]
pub struct QuicHandle {
    quic: *mut fd_quic_sys::fd_quic_t,
    _marker: PhantomData<fd_quic_sys::fd_quic_t>,
}

impl QuicHandle {
    pub fn init(
        self,
        config: &config::Config,
        network_context: Option<&NetworkContext>,
    ) -> Result<ActiveQuic> {
        unsafe {
            let quic_ptr = self.quic as *mut fd_quic_sys::fd_quic_t;
            let config_ptr = &mut (*quic_ptr).config;

            config_ptr.role = config.role.into();
            config_ptr.retry = if config.retry { 1 } else { 0 };
            config_ptr.tick_per_us = config.tick_per_us;
            config_ptr.idle_timeout = config.idle_timeout;
            config_ptr.keep_alive = if config.keep_alive { 1 } else { 0 };
            config_ptr.ack_delay = config.ack_delay;
            config_ptr.ack_threshold = config.ack_threshold;
            config_ptr.retry_ttl = config.retry_ttl;
            config_ptr.tls_hs_ttl = config.tls_hs_ttl;
            config_ptr.initial_rx_max_stream_data = config.initial_rx_max_stream_data;
            config_ptr.net.dscp = config.dscp;

            config_ptr
                .identity_public_key
                .copy_from_slice(&config.identity_public_key);

            config_ptr.sign = None;
            config_ptr.sign_ctx = std::ptr::null_mut();

            config_ptr.keylog_file.fill(0);
            if let Some(keylog_path) = &config.keylog_file {
                let keylog_bytes = keylog_path.as_bytes();
                let copy_len = keylog_bytes.len().min(config_ptr.keylog_file.len() - 1);
                for (i, &byte) in keylog_bytes[..copy_len].iter().enumerate() {
                    config_ptr.keylog_file[i] = byte as i8;
                }
            }

            config_ptr.keep_timed_out = 0;

            let aio_tx = Box::leak(Box::new(fd_quic_sys::fd_aio_private {
                ctx: if let Some(net_ctx) = network_context {
                    net_ctx as *const NetworkContext as *mut c_void
                } else {
                    std::ptr::null_mut()
                },
                send_func: Some(_send_via_aio_be),
            }));

            fd_quic_sys::fd_quic_set_aio_net_tx(quic_ptr, aio_tx as *const _);

            let callbacks = &mut (*quic_ptr).cb;
            let callback_context = if config.role == config::Role::Server {
                let tracker = Arc::new(ConnectionTracker {
                    new_connections: Arc::new(Mutex::new(Vec::new())),
                });
                let ctx = Box::leak(Box::new(ServerCallbackContext {
                    connection_tracker: tracker.clone(),
                }));
                callbacks.quic_ctx = ctx as *mut ServerCallbackContext as *mut c_void;
                Some(tracker)
            } else {
                callbacks.quic_ctx = std::ptr::null_mut();
                None
            };

            callbacks.conn_new = Some(conn_new_cb);
            callbacks.conn_hs_complete = Some(conn_handshake_complete_cb);
            callbacks.conn_final = Some(conn_final_cb);
            callbacks.stream_notify = Some(stream_notify_cb);
            callbacks.stream_rx = Some(stream_rx_cb);
            callbacks.tls_keylog = None;
            callbacks.now = Some(now_cb);
            callbacks.now_ctx = std::ptr::null_mut();

            let quic = fd_quic_sys::fd_quic_init(self.quic);
            if quic.is_null() {
                return Err(QuicError::Internal("QUIC init failed".to_string()));
            }
            Ok(ActiveQuic {
                quic,
                connection_tracker: callback_context,
                _marker: PhantomData,
            })
        }
    }

    /// SAFETY: The caller must ensure that the returned pointer is not used after the
    /// QuicHandle is dropped, and that any operations on it are thread-safe.
    pub unsafe fn as_raw(&self) -> *mut fd_quic_sys::fd_quic_t {
        self.quic
    }
}

impl Drop for QuicHandle {
    fn drop(&mut self) {
        unsafe {
            fd_quic_sys::fd_quic_leave(self.quic);
        }
    }
}

#[repr(C)]
pub struct ActiveQuic {
    quic: *mut fd_quic_sys::fd_quic_t,
    connection_tracker: Option<Arc<ConnectionTracker>>,
    _marker: PhantomData<fd_quic_sys::fd_quic_t>,
}

impl ActiveQuic {
    pub fn connect(
        &mut self,
        dst_addr: SocketAddrV4,
        src_addr: SocketAddrV4,
    ) -> Result<conn::Connection> {
        unsafe {
            let dst_ip = u32::from(*dst_addr.ip()).to_be();
            let dst_port = dst_addr.port();
            let src_ip = u32::from(*src_addr.ip()).to_be();
            let src_port = src_addr.port();

            let conn = fd_quic_sys::fd_quic_connect(self.quic, dst_ip, dst_port, src_ip, src_port);

            if conn.is_null() {
                return Err(QuicError::ConnectionFailed(
                    "failed to create connection".to_string(),
                ));
            }

            Ok(conn::Connection {
                conn,
                _marker: PhantomData,
            })
        }
    }

    pub fn service(&mut self) -> usize {
        unsafe { fd_quic_sys::fd_quic_service(self.quic) as usize }
    }

    pub fn get_next_wakeup(&self) -> u64 {
        unsafe { fd_quic_sys::fd_quic_get_next_wakeup(self.quic) }
    }

    pub fn process_packet(&mut self, data: &mut [u8]) {
        unsafe {
            fd_quic_sys::fd_quic_process_packet(self.quic, data.as_mut_ptr(), data.len() as u64);
        }
    }

    pub fn get_new_connections(&mut self) -> Vec<*mut fd_quic_sys::fd_quic_conn_t> {
        if let Some(ref tracker) = self.connection_tracker {
            if let Ok(mut connections) = tracker.new_connections.lock() {
                let new_conns = connections.drain(..).collect();
                return new_conns;
            }
        }
        Vec::new()
    }

    /// SAFETY: The caller must ensure that the returned pointer is not used after the
    /// ActiveQuic is dropped, and that any operations on it are thread-safe.
    pub unsafe fn as_raw(&self) -> *mut fd_quic_sys::fd_quic_t {
        self.quic
    }
}

impl Drop for ActiveQuic {
    fn drop(&mut self) {
        unsafe {
            fd_quic_sys::fd_quic_fini(self.quic);
        }
    }
}

pub mod conn {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum State {
        Invalid = 0,
        Handshake = 1,
        HandshakeComplete = 2,
        Active = 3,
        PeerClose = 4,
        Abort = 5,
        ClosePending = 6,
        Dead = 7,
        TimedOut = 8,
    }

    #[repr(C)]
    pub struct Connection {
        pub(crate) conn: *mut fd_quic_sys::fd_quic_conn_t,
        pub(crate) _marker: PhantomData<fd_quic_sys::fd_quic_conn_t>,
    }

    impl Connection {
        pub fn close(&mut self, reason: u32) {
            unsafe {
                fd_quic_sys::fd_quic_conn_close(self.conn, reason);
            }
        }

        pub fn let_die(&mut self, keep_alive_duration_ticks: u64) {
            unsafe {
                fd_quic_sys::fd_quic_conn_let_die(self.conn, keep_alive_duration_ticks);
            }
        }

        pub fn new_stream(&mut self) -> Result<stream::Stream> {
            unsafe {
                let stream = fd_quic_sys::fd_quic_conn_new_stream(self.conn);
                if stream.is_null() {
                    return Err(QuicError::StreamError(
                        "failed to create stream".to_string(),
                    ));
                }
                Ok(stream::Stream {
                    stream,
                    _marker: PhantomData,
                })
            }
        }

        pub fn is_active(&self) -> bool {
            unsafe {
                (*self.conn).state == 3 // FD_QUIC_CONN_STATE_ACTIVE
            }
        }

        /// SAFETY: The caller must ensure that the returned pointer is not used after the
        /// Connection is dropped, and that any operations on it are thread-safe.
        pub unsafe fn as_raw(&self) -> *mut fd_quic_sys::fd_quic_conn_t {
            self.conn
        }
    }
}

pub mod stream {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum SendError {
        InvalidStream = -1,
        InvalidConnection = -2,
        Again = -3,
    }

    #[repr(C)]
    pub struct Stream {
        pub(crate) stream: *mut fd_quic_sys::fd_quic_stream_t,
        pub(crate) _marker: PhantomData<fd_quic_sys::fd_quic_stream_t>,
    }

    impl Stream {
        pub fn send(&mut self, data: &[u8], fin: bool) -> Result<()> {
            unsafe {
                let result = fd_quic_sys::fd_quic_stream_send(
                    self.stream,
                    data.as_ptr() as *const _,
                    data.len() as u64,
                    if fin { 1 } else { 0 },
                );

                match result {
                    0 => Ok(()),
                    -1 => Err(QuicError::StreamError("invalid stream".to_string())),
                    -2 => Err(QuicError::StreamError("invalid connection".to_string())),
                    -3 => Err(QuicError::StreamError("try again".to_string())),
                    _ => Err(QuicError::StreamError("unknown error".to_string())),
                }
            }
        }

        pub fn fin(&mut self) {
            unsafe {
                fd_quic_sys::fd_quic_stream_fin(self.stream);
            }
        }

        /// SAFETY: The caller must ensure that the returned pointer is not used after the
        /// Stream is dropped, and that any operations on it are thread-safe.
        pub unsafe fn as_raw(&self) -> *mut fd_quic_sys::fd_quic_stream_t {
            self.stream
        }
    }
}

#[derive(Clone, Debug)]
pub struct QuicClientConfig {
    pub server_name: Option<String>,
    pub limits: config::Limits,
    pub config: config::Config,
}

impl QuicClientConfig {
    pub fn new() -> Self {
        Self {
            server_name: None,
            limits: config::Limits::default(),
            config: config::Config {
                role: config::Role::Client,
                ..config::Config::default()
            },
        }
    }

    pub fn with_server_name<S: Into<String>>(mut self, name: S) -> Self {
        self.server_name = Some(name.into());
        self
    }

    pub fn with_limits(mut self, limits: config::Limits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_idle_timeout(mut self, timeout_ns: u64) -> Self {
        self.config.idle_timeout = timeout_ns;
        self
    }

    pub fn with_keep_alive(mut self, keep_alive: bool) -> Self {
        self.config.keep_alive = keep_alive;
        self
    }

    pub fn with_identity_key(mut self, key: [u8; 32]) -> Self {
        self.config.identity_public_key = key;
        self
    }

    pub fn with_keylog_file<S: Into<String>>(mut self, path: S) -> Self {
        self.config.keylog_file = Some(path.into());
        self
    }
}

impl Default for QuicClientConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct QuicServerConfig {
    pub certificate_chain: Option<String>,
    pub private_key: Option<String>,
    pub limits: config::Limits,
    pub config: config::Config,
}

impl QuicServerConfig {
    pub fn new() -> Self {
        Self {
            certificate_chain: None,
            private_key: None,
            limits: config::Limits::default(),
            config: config::Config {
                role: config::Role::Server,
                retry: true,
                ..config::Config::default()
            },
        }
    }

    pub fn with_certificate_chain<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        self.certificate_chain = Some(
            path.as_ref()
                .to_str()
                .ok_or_else(|| QuicError::ConfigError("invalid certificate path".to_string()))?
                .to_string(),
        );
        Ok(self)
    }

    pub fn with_private_key<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        self.private_key = Some(
            path.as_ref()
                .to_str()
                .ok_or_else(|| QuicError::ConfigError("invalid private key path".to_string()))?
                .to_string(),
        );
        Ok(self)
    }

    pub fn with_limits(mut self, limits: config::Limits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_retry(mut self, retry: bool) -> Self {
        self.config.retry = retry;
        self
    }

    pub fn with_identity_key(mut self, key: [u8; 32]) -> Self {
        self.config.identity_public_key = key;
        self
    }

    pub fn with_keylog_file<S: Into<String>>(mut self, path: S) -> Self {
        self.config.keylog_file = Some(path.into());
        self
    }
}

impl Default for QuicServerConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub struct QuicClient {
    quic: Quic,
    config: QuicClientConfig,
}

impl QuicClient {
    pub fn new(config: QuicClientConfig) -> Result<Self> {
        let quic = Quic::new(config.limits.clone())?;
        Ok(Self { quic, config })
    }

    pub fn connect(
        &mut self,
        server_addr: SocketAddrV4,
        local_addr: SocketAddrV4,
    ) -> Result<QuicConnection> {
        let handle = self.quic.join()?;
        let mut active = handle.init(&self.config.config, None)?; // Client doesn't need network context for now
        let connection = active.connect(server_addr, local_addr)?;

        Ok(QuicConnection {
            connection,
            active_quic: active,
        })
    }
}

pub struct QuicServer {
    quic: Quic,
    config: QuicServerConfig,
    bind_addr: Option<SocketAddrV4>,
    network_context: Option<Box<NetworkContext>>,
    interface_name: Option<String>,
    active_quic: Option<ActiveQuic>,
}

impl QuicServer {
    pub fn new(config: QuicServerConfig) -> Result<Self> {
        let quic = Quic::new(config.limits.clone())?;
        Ok(Self {
            quic,
            config,
            bind_addr: None,
            network_context: None,
            interface_name: None,
            active_quic: None,
        })
    }

    pub fn with_interface<S: Into<String>>(mut self, interface: S) -> Self {
        self.interface_name = Some(interface.into());
        self
    }

    pub fn bind(&mut self, addr: SocketAddrV4) -> Result<()> {
        self.bind_addr = Some(addr);

        let network_context = self.try_setup_xdp(addr).or_else(|_| self.setup_udp(addr))?;
        let handle = self.quic.join()?;
        let active_quic = handle.init(&self.config.config, Some(&network_context))?;

        self.network_context = Some(network_context);
        self.active_quic = Some(active_quic);
        Ok(())
    }

    fn try_setup_xdp(&self, _addr: SocketAddrV4) -> Result<Box<NetworkContext>> {
        #[cfg(target_os = "linux")]
        {
            let interface_name = self.interface_name.as_ref().ok_or_else(|| {
                QuicError::Internal("Interface name required for XDP setup".to_string())
            })?;

            let interface_c_str = CString::new(interface_name.as_str())
                .map_err(|e| QuicError::Internal(format!("Invalid interface name: {}", e)))?;

            let if_idx = unsafe { libc::if_nametoindex(interface_c_str.as_ptr()) };
            if if_idx == 0 {
                return Err(QuicError::Internal(format!(
                    "Interface {} not found",
                    interface_name
                )));
            }

            let ports = vec![_addr.port()];
            let listen_ip = u32::from_be_bytes(_addr.ip().octets());

            let xdp_fds = unsafe {
                fd_quic_sys::fd_xdp_install(
                    if_idx,
                    listen_ip,
                    1,
                    ports.as_ptr(),
                    "skb\0".as_ptr() as *const i8,
                )
            };

            let backend = NetworkBackend::Xdp {
                xsk_map_fd: xdp_fds.xsk_map_fd,
                prog_link_fd: xdp_fds.prog_link_fd,
                interface_name: interface_name.clone(),
                listen_ports: ports,
            };

            let context = Box::new(NetworkContext { backend });
            Ok(context)
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err(QuicError::Internal(
                "XDP not available on this platform".to_string(),
            ))
        }
    }

    fn setup_udp(&self, addr: SocketAddrV4) -> Result<Box<NetworkContext>> {
        let socket = UdpSocket::bind(addr)
            .map_err(|e| QuicError::Internal(format!("Failed to bind UDP socket: {}", e)))?;

        socket.set_nonblocking(true).map_err(|e| {
            QuicError::Internal(format!("Failed to set socket non-blocking: {}", e))
        })?;

        let backend = NetworkBackend::Udp {
            socket: Arc::new(Mutex::new(socket)),
        };

        let context = Box::new(NetworkContext { backend });
        Ok(context)
    }

    pub fn accept(&mut self) -> Result<Option<QuicConnection>> {
        let network_context = self
            .network_context
            .as_ref()
            .ok_or_else(|| QuicError::InvalidState("Server not bound".to_string()))?;

        let active_quic = self
            .active_quic
            .as_mut()
            .ok_or_else(|| QuicError::InvalidState("Server not initialized".to_string()))?;

        match &network_context.backend {
            NetworkBackend::Udp { socket } => {
                let socket = socket
                    .lock()
                    .map_err(|_| QuicError::Internal("Failed to lock socket".to_string()))?;

                let mut buffer = [0u8; 1500];
                match socket.recv_from(&mut buffer) {
                    Ok((len, src_addr)) => {
                        active_quic.process_packet(&mut buffer[..len]);
                        let processed = active_quic.service();

                        let new_connections = active_quic.get_new_connections();
                        if !new_connections.is_empty() {
                            return Ok(Some(QuicConnection {
                                connection: conn::Connection {
                                    conn: new_connections[0],
                                    _marker: PhantomData,
                                },
                                active_quic: ActiveQuic {
                                    quic: std::ptr::null_mut(),
                                    connection_tracker: None,
                                    _marker: PhantomData,
                                },
                            }));
                        }

                        Ok(None)
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
                    Err(e) => Err(QuicError::Io(e)),
                }
            }
            NetworkBackend::Xdp { interface_name, .. } => {
                let processed = active_quic.service();
                Ok(None)
            }
            NetworkBackend::Stub => Ok(None),
        }
    }
}

pub struct QuicConnection {
    connection: conn::Connection,
    active_quic: ActiveQuic,
}

impl QuicConnection {
    pub fn open_stream(&mut self) -> Result<QuicStream> {
        let stream = self.connection.new_stream()?;
        Ok(QuicStream { stream })
    }

    pub fn is_active(&self) -> bool {
        self.connection.is_active()
    }

    pub fn close(&mut self, reason: u32) {
        self.connection.close(reason);
    }

    pub fn service(&mut self) -> usize {
        self.active_quic.service()
    }

    pub fn process_packet(&mut self, data: &mut [u8]) {
        self.active_quic.process_packet(data);
    }
}

pub struct QuicStream {
    stream: stream::Stream,
}

impl QuicStream {
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.stream.send(data, false)
    }

    pub fn write_all(&mut self, data: &[u8]) -> Result<()> {
        // TODO: handle partial writes
        self.stream.send(data, false)
    }

    pub fn finish(&mut self) -> Result<()> {
        self.stream.fin();
        Ok(())
    }

    pub fn send_with_fin(&mut self, data: &[u8]) -> Result<()> {
        self.stream.send(data, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::Role;

    #[test]
    fn test_client_config() {
        let config = QuicClientConfig::new()
            .with_server_name("example.com")
            .with_idle_timeout(5_000_000_000)
            .with_keep_alive(true);

        assert_eq!(config.server_name, Some("example.com".to_string()));
        assert_eq!(config.config.idle_timeout, 5_000_000_000);
        assert_eq!(config.config.keep_alive, true);
        assert_eq!(config.config.role, Role::Client);
    }

    #[test]
    fn test_server_config() {
        let config = QuicServerConfig::new().with_retry(false);

        assert_eq!(config.config.retry, false);
        assert_eq!(config.config.role, Role::Server);
    }

    #[test]
    fn test_client_creation() {
        let config = QuicClientConfig::new();
        let result = QuicClient::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_server_creation() {
        let config = QuicServerConfig::new();
        let result = QuicServer::new(config);
        assert!(result.is_ok());
    }
}
