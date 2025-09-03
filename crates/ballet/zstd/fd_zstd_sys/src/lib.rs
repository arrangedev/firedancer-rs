//! Low-level FFI bindings to Firedancer's fd_zstd module.
//!
//! This crate provides raw, unsafe bindings to the Firedancer Zstandard compression API,
//! which provides streaming compression and decompression for Zstandard (.zst) files.
//!
//! For safe, idiomatic Rust wrappers, see the `fd_zstd` crate.
//!
//! # Safety
//!
//! All functions in this crate are unsafe and require careful handling of:
//! - Memory management and lifetime guarantees
//! - Proper initialization of dstream objects with correct memory alignment
//! - Buffer size requirements and bounds checking
//! - Window size limits for decompression contexts
//! - Thread safety considerations
//!
//! # Zstandard Operations
//!
//! The main Zstandard operations available:
//! - `fd_zstd_peek`: Peek at frame header to determine window size and content size
//! - `fd_zstd_dstream_new`: Create a new decompression stream
//! - `fd_zstd_dstream_read`: Decompress data in streaming fashion
//! - `fd_zstd_dstream_reset`: Reset decompression stream for new frame
//! - `fd_zstd_dstream_delete`: Clean up decompression stream
//!
//! # Memory Management
//!
//! fd_zstd uses static memory allocation with no heap allocations or syscalls.
//! Each `fd_zstd_dstream_t` object requires a contiguous memory region sized
//! according to the maximum window size it needs to handle.
//!
//! Use `fd_zstd_dstream_align()` and `fd_zstd_dstream_footprint()` to determine
//! the memory requirements before calling `fd_zstd_dstream_new()`.
//!
//! # Example
//!
//! ```rust,no_run
//! use fd_zstd_sys::*;
//! use std::alloc::{alloc, dealloc, Layout};
//! use std::ptr;
//!
//! unsafe {
//!     // Determine memory requirements for 128KB window
//!     let max_window_sz = 128 * 1024;
//!     let align = fd_zstd_dstream_align();
//!     let footprint = fd_zstd_dstream_footprint(max_window_sz);
//!     
//!     // Allocate memory for dstream
//!     let layout = Layout::from_size_align(footprint as usize, align as usize).unwrap();
//!     let mem = alloc(layout);
//!     if mem.is_null() {
//!         panic!("Failed to allocate memory");
//!     }
//!     
//!     // Create decompression stream
//!     let dstream = fd_zstd_dstream_new(mem as *mut _, max_window_sz);
//!     if dstream.is_null() {
//!         dealloc(mem, layout);
//!         panic!("Failed to create dstream");
//!     }
//!     
//!     // Use dstream for decompression...
//!     // (see fd_zstd_dstream_read documentation)
//!     
//!     // Clean up
//!     let returned_mem = fd_zstd_dstream_delete(dstream);
//!     assert_eq!(returned_mem, mem as *mut _);
//!     dealloc(mem, layout);
//! }
//! ```
//!
//! # Constants
//!
//! - `FD_ZSTD_MAX_HDR_SZ`: Maximum size of frame header (18 bytes)
//! - `FD_ZSTD_CSTREAM_ALIGN`: Alignment requirement for compression streams (64 bytes)

#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_static_lifetimes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(FD_ZSTD_MAX_HDR_SZ, 18);
        assert_eq!(FD_ZSTD_CSTREAM_ALIGN, 64);
    }

    #[test]
    fn test_dstream_align() {
        unsafe {
            let align = fd_zstd_dstream_align();
            assert!(align > 0);
            assert_eq!(align & (align - 1), 0);
        }
    }

    #[test]
    fn test_dstream_footprint() {
        unsafe {
            let footprint_64k = fd_zstd_dstream_footprint(64 * 1024);
            let footprint_128k = fd_zstd_dstream_footprint(128 * 1024);

            assert!(footprint_128k >= footprint_64k);
            assert!(footprint_64k > 1000);
            assert!(footprint_64k < 100 * 1024 * 1024);
        }
    }
}
