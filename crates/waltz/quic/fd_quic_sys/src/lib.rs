//! Raw FFI bindings to Firedancer QUIC implementation
//!
//! This crate provides low-level, unsafe bindings to the Firedancer QUIC library:
//! - QUIC protocol implementation (RFC 9000, RFC 9001)
//! - Connection management and lifecycle
//! - Stream multiplexing and flow control
//! - Packet processing and encryption/decryption
//! - TLS 1.3 handshake integration
//! - High-performance networking primitives
//!
//! For a safe Rust API, consider using the higher-level wrapper crate.
//!
//! # Safety
//!
//! All functions in this crate are unsafe and require careful handling of:
//! - Memory management and lifetime guarantees
//! - Thread safety and concurrency
//! - Proper initialization and cleanup
//! - Network buffer management
//!
//! # Example
//!
//! ```rust,no_run
//! use fd_quic_sys::*;
//! use std::ptr;
//!
//! unsafe {
//!     // Set up QUIC limits
//!     let limits = fd_quic_limits_t {
//!         conn_cnt: 16,
//!         handshake_cnt: 8,
//!         log_depth: 1024,
//!         conn_id_cnt: 4,
//!         stream_id_cnt: 64,
//!         inflight_frame_cnt: 256,
//!         min_inflight_frame_cnt_conn: 16,
//!         tx_buf_sz: 65536,
//!         stream_pool_cnt: 128,
//!     };
//!
//!     // Calculate memory requirements
//!     let align = fd_quic_align();
//!     let footprint = fd_quic_footprint(&limits);
//!     
//!     if footprint > 0 {
//!         // Allocate memory and create QUIC instance
//!         let layout = std::alloc::Layout::from_size_align(footprint as usize, align as usize).unwrap();
//!         let mem = std::alloc::alloc_zeroed(layout);
//!         if !mem.is_null() {
//!             let quic = fd_quic_new(mem as *mut _, &limits);
//!             if !quic.is_null() {
//!                 // Initialize and use QUIC...
//!                 fd_quic_delete(quic as *mut _);
//!             }
//!             std::alloc::dealloc(mem, layout);
//!         }
//!     }
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_footprint() {
        unsafe {
            let align = fd_quic_align();
            assert!(align > 0);
            assert!(align.is_power_of_two());

            let limits = fd_quic_limits_t {
                conn_cnt: 1,
                handshake_cnt: 1,
                log_depth: 64,
                conn_id_cnt: 4,
                stream_id_cnt: 16,
                inflight_frame_cnt: 32,
                min_inflight_frame_cnt_conn: 8,
                tx_buf_sz: 4096,
                stream_pool_cnt: 16,
            };

            let footprint = fd_quic_footprint(&limits);
            assert!(footprint > 0);
        }
    }

    #[test]
    fn test_new_delete() {
        unsafe {
            let limits = fd_quic_limits_t {
                conn_cnt: 1,
                handshake_cnt: 1,
                log_depth: 64,
                conn_id_cnt: 4,
                stream_id_cnt: 16,
                inflight_frame_cnt: 32,
                min_inflight_frame_cnt_conn: 8,
                tx_buf_sz: 4096,
                stream_pool_cnt: 16,
            };

            let align = fd_quic_align();
            let footprint = fd_quic_footprint(&limits);

            if footprint == 0 {
                return;
            }

            let layout =
                std::alloc::Layout::from_size_align(footprint as usize, align as usize).unwrap();
            let mem = std::alloc::alloc_zeroed(layout);
            if mem.is_null() {
                panic!("Failed to allocate memory");
            }

            let quic = fd_quic_new(mem as *mut _, &limits);

            if !quic.is_null() {
                let returned_mem = fd_quic_delete(quic as *mut _);
                assert_eq!(returned_mem, mem as *mut _);
            }

            std::alloc::dealloc(mem, layout);
        }
    }

    #[test]
    fn test_bindings_exist() {
        unsafe {
            let _align = fd_quic_align();
            let limits = fd_quic_limits_t {
                conn_cnt: 1,
                handshake_cnt: 1,
                log_depth: 64,
                conn_id_cnt: 4,
                stream_id_cnt: 16,
                inflight_frame_cnt: 32,
                min_inflight_frame_cnt_conn: 8,
                tx_buf_sz: 4096,
                stream_pool_cnt: 16,
            };

            let _footprint = fd_quic_footprint(&limits);
        }
    }
}
