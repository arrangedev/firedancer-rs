use core::fmt;
use fd_rpc_sys as sys;

#[derive(Debug)]
pub enum XdpError {
    InitFailed,
    InstallFailed,
    ActivateFailed,
    TxFull,
    FrameTooLarge,
}

impl fmt::Display for XdpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XdpError::InitFailed => write!(f, "XSK initialization failed"),
            XdpError::InstallFailed => write!(f, "XDP program installation failed"),
            XdpError::ActivateFailed => write!(f, "XSK activation failed"),
            XdpError::TxFull => write!(f, "TX ring full"),
            XdpError::FrameTooLarge => write!(f, "packet exceeds frame size"),
        }
    }
}

impl core::error::Error for XdpError {}

const DEFAULT_FRAME_SZ: u64 = 2048;
const DEFAULT_RING_DEPTH: u64 = 1024;
const ETH_HDR_SZ: usize = 14;
const IP4_HDR_SZ: usize = 20;
const UDP_HDR_SZ: usize = 8;
const L2L3L4_HDR_SZ: usize = ETH_HDR_SZ + IP4_HDR_SZ + UDP_HDR_SZ;

pub struct XdpConfig {
    pub if_name: [u8; 16],
    pub if_queue: u32,
    pub src_ip: u32,
    pub src_port: u16,
    pub src_mac: [u8; 6],
    pub listen_ports: [u16; 8],
    pub listen_ports_cnt: usize,
    pub frame_sz: u64,
    pub ring_depth: u64,
}

impl Default for XdpConfig {
    fn default() -> Self {
        Self {
            if_name: [0u8; 16],
            if_queue: 0,
            src_ip: 0,
            src_port: 0,
            src_mac: [0u8; 6],
            listen_ports: [0u16; 8],
            listen_ports_cnt: 0,
            frame_sz: DEFAULT_FRAME_SZ,
            ring_depth: DEFAULT_RING_DEPTH,
        }
    }
}

impl XdpConfig {
    pub fn set_interface(&mut self, name: &str) {
        let len = name.len().min(15);
        self.if_name[..len].copy_from_slice(&name.as_bytes()[..len]);
        self.if_name[len] = 0;
    }
}

pub struct XdpConnection {
    xsk: sys::fd_xsk_t,
    xdp_fds: sys::fd_xdp_fds_t,
    _umem: Vec<u8>,
    umem_ptr: *mut u8,
    frame_sz: u64,
    src_ip: u32,
    src_port: u16,
    src_mac: [u8; 6],
    tx_frame_idx: u64,
    tx_frame_cnt: u64,
}

impl XdpConnection {
    pub fn new(config: &XdpConfig) -> Result<Self, XdpError> {
        let frame_sz = if config.frame_sz > 0 {
            config.frame_sz
        } else {
            DEFAULT_FRAME_SZ
        };
        let ring_depth = if config.ring_depth > 0 {
            config.ring_depth
        } else {
            DEFAULT_RING_DEPTH
        };

        let total_frames = ring_depth * 4;
        let umem_sz = (total_frames * frame_sz) as usize;
        let mut umem = vec![0u8; umem_sz + 4096];
        let align_offset = umem.as_ptr() as usize % 4096;
        let umem_ptr = if align_offset == 0 {
            umem.as_mut_ptr()
        } else {
            unsafe { umem.as_mut_ptr().add(4096 - align_offset) }
        };

        let if_idx =
            unsafe { libc::if_nametoindex(config.if_name.as_ptr() as *const libc::c_char) };
        if if_idx == 0 {
            return Err(XdpError::InitFailed);
        }

        let params = sys::fd_xsk_params {
            fr_depth: ring_depth,
            rx_depth: ring_depth,
            tx_depth: ring_depth,
            cr_depth: ring_depth,
            umem_addr: umem_ptr as *mut libc::c_void,
            frame_sz,
            umem_sz: umem_sz as u64,
            if_idx,
            if_queue_id: config.if_queue,
            bind_flags: 0,
        };

        let mut xsk: sys::fd_xsk_t = unsafe { core::mem::zeroed() };
        let result = unsafe { sys::fd_xsk_init(&mut xsk, &params) };
        if result.is_null() {
            return Err(XdpError::InitFailed);
        }

        let xdp_fds = unsafe {
            sys::fd_xdp_install(
                if_idx,
                config.src_ip,
                config.listen_ports_cnt as u64,
                config.listen_ports.as_ptr(),
                b"skb\0".as_ptr() as *const libc::c_char,
            )
        };
        if xdp_fds.xsk_map_fd < 0 {
            return Err(XdpError::InstallFailed);
        }

        let activated = unsafe { sys::fd_xsk_activate(&mut xsk, xdp_fds.xsk_map_fd) };
        if activated.is_null() {
            return Err(XdpError::ActivateFailed);
        }

        Ok(Self {
            xsk,
            xdp_fds,
            _umem: umem,
            umem_ptr,
            frame_sz,
            src_ip: config.src_ip,
            src_port: config.src_port,
            src_mac: config.src_mac,
            tx_frame_idx: 0,
            tx_frame_cnt: total_frames,
        })
    }

    pub fn send_udp(
        &mut self,
        dst_ip: u32,
        dst_port: u16,
        dst_mac: &[u8; 6],
        payload: &[u8],
    ) -> Result<(), XdpError> {
        let total_len = L2L3L4_HDR_SZ + payload.len();
        if total_len > self.frame_sz as usize {
            return Err(XdpError::FrameTooLarge);
        }

        let frame_offset = self.tx_frame_idx * self.frame_sz;
        let frame = unsafe {
            core::slice::from_raw_parts_mut(self.umem_ptr.add(frame_offset as usize), total_len)
        };

        build_eth_header(&mut frame[..ETH_HDR_SZ], dst_mac, &self.src_mac);
        build_ip4_header(
            &mut frame[ETH_HDR_SZ..ETH_HDR_SZ + IP4_HDR_SZ],
            self.src_ip,
            dst_ip,
            (IP4_HDR_SZ + UDP_HDR_SZ + payload.len()) as u16,
        );
        build_udp_header(
            &mut frame[ETH_HDR_SZ + IP4_HDR_SZ..L2L3L4_HDR_SZ],
            self.src_port,
            dst_port,
            (UDP_HDR_SZ + payload.len()) as u16,
        );
        frame[L2L3L4_HDR_SZ..].copy_from_slice(payload);

        let ring_tx = &mut self.xsk.ring_tx;
        let prod = unsafe { *ring_tx.prod };
        let cons = unsafe { *ring_tx.cons };
        if prod.wrapping_sub(cons) >= ring_tx.depth {
            return Err(XdpError::TxFull);
        }

        let idx = (prod & (ring_tx.depth - 1)) as isize;
        let desc = unsafe { &mut *ring_tx.packet_ring.offset(idx) };
        desc.addr = frame_offset;
        desc.len = total_len as u32;
        desc.options = 0;

        core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
        unsafe { *ring_tx.prod = prod.wrapping_add(1) };

        self.tx_frame_idx = (self.tx_frame_idx + 1) % self.tx_frame_cnt;

        if unsafe { sys::fd_xsk_tx_need_wakeup(&mut self.xsk) } != 0 {
            unsafe {
                libc::sendto(
                    self.xsk.xsk_fd,
                    core::ptr::null(),
                    0,
                    libc::MSG_DONTWAIT,
                    core::ptr::null(),
                    0,
                );
            }
        }

        Ok(())
    }
}

impl Drop for XdpConnection {
    fn drop(&mut self) {
        unsafe {
            sys::fd_xsk_deactivate(&mut self.xsk, self.xdp_fds.xsk_map_fd);
            if self.xdp_fds.prog_link_fd >= 0 {
                libc::close(self.xdp_fds.prog_link_fd);
            }
            if self.xdp_fds.xsk_map_fd >= 0 {
                libc::close(self.xdp_fds.xsk_map_fd);
            }
            sys::fd_xsk_delete(&mut self.xsk as *mut _ as *mut libc::c_void);
        }
    }
}

#[inline]
fn build_eth_header(buf: &mut [u8], dst_mac: &[u8; 6], src_mac: &[u8; 6]) {
    buf[0..6].copy_from_slice(dst_mac);
    buf[6..12].copy_from_slice(src_mac);
    buf[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
}

#[inline]
fn build_ip4_header(buf: &mut [u8], src_ip: u32, dst_ip: u32, total_len: u16) {
    buf[0] = 0x45;
    buf[1] = 0;
    buf[2..4].copy_from_slice(&total_len.to_be_bytes());
    buf[4..6].copy_from_slice(&[0, 0]);
    buf[6..8].copy_from_slice(&[0x40, 0x00]);
    buf[8] = 64;
    buf[9] = 17;
    buf[10..12].copy_from_slice(&[0, 0]);
    buf[12..16].copy_from_slice(&src_ip.to_be_bytes());
    buf[16..20].copy_from_slice(&dst_ip.to_be_bytes());

    let mut checksum: u32 = 0;
    for i in (0..20).step_by(2) {
        checksum += u16::from_be_bytes([buf[i], buf[i + 1]]) as u32;
    }
    while checksum > 0xFFFF {
        checksum = (checksum & 0xFFFF) + (checksum >> 16);
    }
    let checksum = !(checksum as u16);
    buf[10..12].copy_from_slice(&checksum.to_be_bytes());
}

#[inline]
fn build_udp_header(buf: &mut [u8], src_port: u16, dst_port: u16, total_len: u16) {
    buf[0..2].copy_from_slice(&src_port.to_be_bytes());
    buf[2..4].copy_from_slice(&dst_port.to_be_bytes());
    buf[4..6].copy_from_slice(&total_len.to_be_bytes());
    buf[6..8].copy_from_slice(&[0, 0]);
}
