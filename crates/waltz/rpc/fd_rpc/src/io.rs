use core::fmt;
use fd_rpc_sys as sys;

const DEFAULT_BUF_SZ: usize = 65536;

#[derive(Debug)]
pub enum IoError {
    ConnectFailed(i32),
    AlreadyConnected,
    NotConnected,
    Closed,
    Ssl(i32),
    Os(i32),
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IoError::ConnectFailed(e) => write!(f, "connect failed (errno {})", e),
            IoError::AlreadyConnected => write!(f, "already connected"),
            IoError::NotConnected => write!(f, "not connected"),
            IoError::Closed => write!(f, "connection closed by peer"),
            IoError::Ssl(e) => write!(f, "TLS error ({})", e),
            IoError::Os(e) => write!(f, "OS error (errno {})", e),
        }
    }
}

impl core::error::Error for IoError {}

#[derive(Debug, Default)]
pub struct PumpResult {
    pub connected: bool,
    pub rx_data: bool,
    pub tx_drain: bool,
    pub error: bool,
    pub closed: bool,
}

impl PumpResult {
    fn from_flags(flags: u32) -> Self {
        Self {
            connected: flags & sys::FD_RPC_IO_PUMP_CONNECTED != 0,
            rx_data: flags & sys::FD_RPC_IO_PUMP_RX_DATA != 0,
            tx_drain: flags & sys::FD_RPC_IO_PUMP_TX_DRAIN != 0,
            error: flags & sys::FD_RPC_IO_PUMP_ERROR != 0,
            closed: flags & sys::FD_RPC_IO_PUMP_CLOSED != 0,
        }
    }
}

pub struct Connection {
    io: *mut sys::fd_rpc_io_t,
    _rx_buf: Box<[u8]>,
    _tx_buf: Box<[u8]>,
    _io_mem: Box<[u8]>,
}

unsafe impl Send for Connection {}

impl Connection {
    pub fn new() -> Self {
        Self::with_buf_sz(DEFAULT_BUF_SZ, DEFAULT_BUF_SZ)
    }

    pub fn with_buf_sz(rx_bufsz: usize, tx_bufsz: usize) -> Self {
        let footprint = unsafe { sys::fd_rpc_io_footprint() } as usize;
        let align = unsafe { sys::fd_rpc_io_align() } as usize;
        let mut io_mem = vec![0u8; footprint + align].into_boxed_slice();
        let mut rx_buf = vec![0u8; rx_bufsz].into_boxed_slice();
        let mut tx_buf = vec![0u8; tx_bufsz].into_boxed_slice();

        let aligned = {
            let ptr = io_mem.as_mut_ptr() as usize;
            ((ptr + align - 1) & !(align - 1)) as *mut u8
        };

        let io = unsafe {
            sys::fd_rpc_io_new(
                aligned as *mut core::ffi::c_void,
                rx_buf.as_mut_ptr() as *mut core::ffi::c_void,
                rx_bufsz as u64,
                tx_buf.as_mut_ptr() as *mut core::ffi::c_void,
                tx_bufsz as u64,
            )
        };
        assert!(!io.is_null());

        Self {
            io,
            _rx_buf: rx_buf,
            _tx_buf: tx_buf,
            _io_mem: io_mem,
        }
    }

    pub fn connect(
        &mut self,
        addr: u32,
        port: u16,
        use_tls: bool,
        hostname: Option<&str>,
    ) -> Result<(), IoError> {
        let host_cstr;
        let host_ptr = match hostname {
            Some(h) => {
                host_cstr = std::ffi::CString::new(h).map_err(|_| IoError::ConnectFailed(0))?;
                host_cstr.as_ptr()
            }
            None => core::ptr::null(),
        };

        let rc = unsafe {
            sys::fd_rpc_io_connect(self.io, addr, port, if use_tls { 1 } else { 0 }, host_ptr)
        };

        if rc < 0 {
            let err = unsafe { (*self.io).err };
            return Err(IoError::ConnectFailed(err));
        }
        Ok(())
    }

    pub fn pump(&mut self) -> PumpResult {
        let flags = unsafe { sys::fd_rpc_io_pump(self.io) };
        PumpResult::from_flags(flags)
    }

    pub fn state(&self) -> i32 {
        unsafe { sys::fd_rpc_io_state(self.io) }
    }

    pub fn is_ready(&self) -> bool {
        self.state() == sys::FD_RPC_IO_STATE_READY as i32
    }

    pub fn rbuf_rx(&mut self) -> &mut sys::fd_h2_rbuf_t {
        unsafe { &mut *sys::fd_rpc_io_rbuf_rx(self.io) }
    }

    pub fn rbuf_tx(&mut self) -> &mut sys::fd_h2_rbuf_t {
        unsafe { &mut *sys::fd_rpc_io_rbuf_tx(self.io) }
    }

    pub fn tx_push(&mut self, data: &[u8]) {
        let rbuf = self.rbuf_tx() as *mut sys::fd_h2_rbuf_t;
        unsafe {
            sys::fd_h2_rbuf_push(
                rbuf,
                data.as_ptr() as *const core::ffi::c_void,
                data.len() as u64,
            );
        }
    }

    pub fn rx_used(&self) -> usize {
        unsafe { sys::fd_h2_rbuf_used_sz((*self.io).rbuf_rx.as_ptr()) as usize }
    }

    /// Copy up to `buf.len()` bytes from the RX ring buffer into `buf`.
    /// Returns number of bytes copied. Consumes the bytes from the ring buffer.
    pub fn rx_pop(&mut self, buf: &mut [u8]) -> usize {
        let rbuf = self.rbuf_rx() as *mut sys::fd_h2_rbuf_t;
        let avail = unsafe { sys::fd_h2_rbuf_used_sz(rbuf) } as usize;
        let n = avail.min(buf.len());
        if n > 0 {
            unsafe {
                sys::fd_h2_rbuf_pop_copy(
                    rbuf,
                    buf.as_mut_ptr() as *mut core::ffi::c_void,
                    n as u64,
                );
            }
        }
        n
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe { sys::fd_rpc_io_close(self.io) };
    }
}
