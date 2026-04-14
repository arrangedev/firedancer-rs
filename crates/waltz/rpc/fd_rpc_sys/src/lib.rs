#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]
#![allow(clippy::all)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub type uchar = ::std::os::raw::c_uchar;
pub type ushort = ::std::os::raw::c_ushort;
pub type uint = ::std::os::raw::c_uint;
pub type ulong = ::std::os::raw::c_ulong;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct fd_h2_rbuf {
    pub buf0: *mut u8,
    pub buf1: *mut u8,
    pub lo: *mut u8,
    pub hi: *mut u8,
    pub lo_off: u64,
    pub hi_off: u64,
    pub bufsz: u64,
}

pub type fd_h2_rbuf_t = fd_h2_rbuf;

#[repr(C)]
pub struct fd_rpc_io {
    pub sock_fd: ::std::os::raw::c_int,
    pub state: ::std::os::raw::c_int,
    pub err: ::std::os::raw::c_int,
    _pad0: ::std::os::raw::c_int,
    pub ssl_ctx: *mut ::std::os::raw::c_void,
    pub ssl: *mut ::std::os::raw::c_void,
    pub ssl_hs_done: ::std::os::raw::c_int,
    _pad1: ::std::os::raw::c_int,
    pub rbuf_rx: [fd_h2_rbuf_t; 1],
    pub rbuf_tx: [fd_h2_rbuf_t; 1],
}

pub type fd_rpc_io_t = fd_rpc_io;

const _: () = assert!(::std::mem::size_of::<fd_h2_rbuf>() == 56);
const _: () = assert!(::std::mem::align_of::<fd_h2_rbuf>() == 8);
const _: () = assert!(::std::mem::size_of::<fd_rpc_io>() == 152);
const _: () = assert!(::std::mem::align_of::<fd_rpc_io>() == 8);
