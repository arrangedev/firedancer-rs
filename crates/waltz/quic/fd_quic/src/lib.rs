//! Safe API for Firedancer QUIC implementation
//!
//! This wraps the FFI bindings provided by `fd_quic_sys` and provides
//! safer abstractions for their use.
//!
//! ## Structure
//!
//! - `quic`: QUIC instance creation and management
//! - `conn`: Connection handling and lifecycle
//! - `stream`: Stream multiplexing and data transfer
//! - `config`: Configuration and limits
//!
//! ## Features
//!
//! - **RFC 9000/9001 Compliant**: Full QUIC protocol implementation
//! - **High Performance**: Zero-copy networking and efficient memory management
//! - **TLS 1.3 Integration**: Secure handshakes and encryption
//! - **Stream Multiplexing**: Concurrent bidirectional and unidirectional streams
//! - **Flow Control**: Connection and stream-level flow control
//! - **Loss Recovery**: Packet retransmission and congestion control
//!
//! ## TODO -- Missing Deps
//!
//! - **`fd_aio`**: Asynchronous I/O for network operations
//!   - Located at: `vendor/waltz/aio/`
//! - **`fd_tls`**: TLS 1.3 implementation for QUIC handshakes
//!   - Located at: `vendor/waltz/tls/`
//! - **`fd_util`**: Core utility functions and memory operations
//!   - Located at: `vendor/util/`
//! - **`fd_ballet`**: Cryptographic primitives
//!   - Located at: `vendor/ballet/`

use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::net::{Ipv4Addr, SocketAddrV4};

pub mod config {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct Limits {
        /// Max concurrent connections
        pub conn_cnt: usize,
        /// Max concurrent handshakes
        pub handshake_cnt: usize,
        /// Log cache depth
        pub log_depth: usize,
        /// connection_id count per connection (min 4)
        pub conn_id_cnt: usize,
        /// Max concurrent stream_ids per connection
        pub stream_id_cnt: usize,
        /// Total max inflight frame count
        pub inflight_frame_cnt: usize,
        /// Min inflight frame count per connection
        pub min_inflight_frame_cnt_conn: usize,
        /// Transmit buffer size per stream (bytes)
        pub tx_buf_sz: usize,
        /// Number of streams in stream_pool
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
                tx_buf_sz: 65536, // 64 KiB
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
        /// Protocol role (client or server)
        pub role: Role,
        /// Enable address validation using retry packets
        pub retry: bool,
        /// Clock ticks per microsecond
        pub tick_per_us: f64,
        /// Idle timeout (nanoseconds)
        pub idle_timeout: u64,
        /// Keep connections alive with PING frames
        pub keep_alive: bool,
        /// ACK delay (nanoseconds)
        pub ack_delay: u64,
        /// ACK threshold (bytes)
        pub ack_threshold: u64,
        /// Retry token TTL (nanoseconds)
        pub retry_ttl: u64,
        /// TLS handshake TTL (nanoseconds)
        pub tls_hs_ttl: u64,
        /// Initial RX max stream data (bytes)
        pub initial_rx_max_stream_data: u64,
        /// Differentiated services code point
        pub dscp: u8,
    }

    impl Default for Config {
        fn default() -> Self {
            Self {
                role: Role::Client,
                retry: false,
                tick_per_us: 1000.0,         // 1 GHz clock
                idle_timeout: 1_000_000_000, // 1 second
                keep_alive: false,
                ack_delay: 50_000_000,             // 50ms
                ack_threshold: 65536,              // 64 KiB
                retry_ttl: 1_000_000_000,          // 1 second
                tls_hs_ttl: 3_000_000_000,         // 3 seconds
                initial_rx_max_stream_data: 65536, // 64 KiB
                dscp: 0,
            }
        }
    }
}

pub mod quic {
    use super::*;

    pub struct Quic {
        quic: *mut fd_quic_sys::fd_quic_t,
        mem: *mut u8,
        layout: Layout,
        _marker: PhantomData<*mut fd_quic_sys::fd_quic_t>,
    }

    unsafe impl Send for Quic {}
    unsafe impl Sync for Quic {}

    impl Quic {
        pub fn new(limits: config::Limits) -> Result<Self, &'static str> {
            let limits_sys: fd_quic_sys::fd_quic_limits_t = limits.into();

            unsafe {
                let align = fd_quic_sys::fd_quic_align() as usize;
                let footprint = fd_quic_sys::fd_quic_footprint(&limits_sys) as usize;

                if footprint == 0 {
                    return Err("invalid limits");
                }

                let layout =
                    Layout::from_size_align(footprint, align).map_err(|_| "invalid layout")?;

                let mem = alloc_zeroed(layout);
                if mem.is_null() {
                    return Err("memory allocation failed");
                }

                let quic = fd_quic_sys::fd_quic_new(mem as *mut _, &limits_sys);
                if quic.is_null() {
                    dealloc(mem, layout);
                    return Err("QUIC initialization failed");
                }

                Ok(Quic {
                    quic: quic as *mut fd_quic_sys::fd_quic_t,
                    mem,
                    layout,
                    _marker: PhantomData,
                })
            }
        }

        pub fn join(&mut self) -> Result<QuicHandle, &'static str> {
            unsafe {
                let handle = fd_quic_sys::fd_quic_join(self.quic as *mut _);
                if handle.is_null() {
                    return Err("failed to join QUIC instance");
                }
                Ok(QuicHandle {
                    quic: handle,
                    _marker: PhantomData,
                })
            }
        }

        /// # Safety: The caller must ensure that the returned pointer is not used after the
        /// Quic is dropped, and that any operations on it are thread-safe.
        pub unsafe fn as_raw(&self) -> *mut fd_quic_sys::fd_quic_t {
            self.quic
        }
    }

    impl Drop for Quic {
        fn drop(&mut self) {
            unsafe {
                let returned_mem = fd_quic_sys::fd_quic_delete(self.quic);
                if returned_mem != self.mem as *mut _ {
                    eprintln!("Warning: fd_quic_delete returned unexpected memory pointer");
                }
                dealloc(self.mem, self.layout);
            }
        }
    }

    pub struct QuicHandle {
        quic: *mut fd_quic_sys::fd_quic_t,
        _marker: PhantomData<*mut fd_quic_sys::fd_quic_t>,
    }

    impl QuicHandle {
        pub fn init(self) -> Result<ActiveQuic, &'static str> {
            unsafe {
                let quic = fd_quic_sys::fd_quic_init(self.quic);
                if quic.is_null() {
                    return Err("QUIC init failed");
                }
                Ok(ActiveQuic {
                    quic,
                    _marker: PhantomData,
                })
            }
        }

        /// # Safety: The caller must ensure that the returned pointer is not used after the
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

    pub struct ActiveQuic {
        quic: *mut fd_quic_sys::fd_quic_t,
        _marker: PhantomData<*mut fd_quic_sys::fd_quic_t>,
    }

    impl ActiveQuic {
        pub fn connect(
            &mut self,
            dst_addr: SocketAddrV4,
            src_addr: SocketAddrV4,
        ) -> Result<conn::Connection, &'static str> {
            unsafe {
                let dst_ip = u32::from(*dst_addr.ip()).to_be();
                let dst_port = dst_addr.port();
                let src_ip = u32::from(*src_addr.ip()).to_be();
                let src_port = src_addr.port();

                let conn =
                    fd_quic_sys::fd_quic_connect(self.quic, dst_ip, dst_port, src_ip, src_port);

                if conn.is_null() {
                    return Err("failed to create connection");
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
                fd_quic_sys::fd_quic_process_packet(
                    self.quic,
                    data.as_mut_ptr(),
                    data.len() as u64,
                );
            }
        }

        /// # Safety: The caller must ensure that the returned pointer is not used after the
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

    pub struct Connection {
        pub(crate) conn: *mut fd_quic_sys::fd_quic_conn_t,
        pub(crate) _marker: PhantomData<*mut fd_quic_sys::fd_quic_conn_t>,
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

        pub fn new_stream(&mut self) -> Result<stream::Stream, &'static str> {
            unsafe {
                let stream = fd_quic_sys::fd_quic_conn_new_stream(self.conn);
                if stream.is_null() {
                    return Err("failed to create stream");
                }
                Ok(stream::Stream {
                    stream,
                    _marker: PhantomData,
                })
            }
        }

        /// # Safety: The caller must ensure that the returned pointer is not used after the
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

    pub struct Stream {
        pub(crate) stream: *mut fd_quic_sys::fd_quic_stream_t,
        pub(crate) _marker: PhantomData<*mut fd_quic_sys::fd_quic_stream_t>,
    }

    impl Stream {
        pub fn send(&mut self, data: &[u8], fin: bool) -> Result<(), SendError> {
            unsafe {
                let result = fd_quic_sys::fd_quic_stream_send(
                    self.stream,
                    data.as_ptr() as *const _,
                    data.len() as u64,
                    if fin { 1 } else { 0 },
                );

                match result {
                    0 => Ok(()),
                    -1 => Err(SendError::InvalidStream),
                    -2 => Err(SendError::InvalidConnection),
                    -3 => Err(SendError::Again),
                    _ => Err(SendError::InvalidStream),
                }
            }
        }

        pub fn fin(&mut self) {
            unsafe {
                fd_quic_sys::fd_quic_stream_fin(self.stream);
            }
        }

        /// # Safety: The caller must ensure that the returned pointer is not used after the
        /// Stream is dropped, and that any operations on it are thread-safe.
        pub unsafe fn as_raw(&self) -> *mut fd_quic_sys::fd_quic_stream_t {
            self.stream
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{Config, Limits, Role};
    use std::net::{Ipv4Addr, SocketAddrV4};

    #[test]
    fn test_creation() {
        let limits = Limits::default();
        let result = quic::Quic::new(limits);
        assert!(result.is_ok());
    }

    #[test]
    fn test_role_conversion() {
        let client: i32 = Role::Client.into();
        let server: i32 = Role::Server.into();

        assert_eq!(client, fd_quic_sys::FD_QUIC_ROLE_CLIENT as i32);
        assert_eq!(server, fd_quic_sys::FD_QUIC_ROLE_SERVER as i32);
    }

    #[test]
    fn test_limits_conversion() {
        let limits = Limits {
            conn_cnt: 32,
            handshake_cnt: 16,
            log_depth: 2048,
            conn_id_cnt: 8,
            stream_id_cnt: 128,
            inflight_frame_cnt: 512,
            min_inflight_frame_cnt_conn: 32,
            tx_buf_sz: 131072,
            stream_pool_cnt: 256,
        };

        let sys_limits: fd_quic_sys::fd_quic_limits_t = limits.into();
        assert_eq!(sys_limits.conn_cnt, 32);
        assert_eq!(sys_limits.handshake_cnt, 16);
        assert_eq!(sys_limits.tx_buf_sz, 131072);
    }

    #[test]
    fn test_socketaddr_conversion() {
        let addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
        let ip_u32 = u32::from(*addr.ip()).to_be();
        let port = addr.port();

        assert_eq!(port, 8080);
        assert_ne!(ip_u32, 0);
    }
}
