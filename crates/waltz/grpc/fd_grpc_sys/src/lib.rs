//! Raw FFI bindings to Firedancer's gRPC implementation
//!
//! This crate provides unsafe, low-level bindings to the gRPC client implementation
//! used by Firedancer. These bindings are generated automatically from the C headers
//! using bindgen.
//!
//! ## Safety
//!
//! All functions in this crate are `unsafe` and require careful handling of:
//! - Memory management (allocation/deallocation)
//! - Pointer validity and lifetimes
//! - Thread safety
//! - Proper initialization of structures
//! - Connection lifecycle management
//!
//! For a safe, idiomatic Rust API, use the `fd_grpc` crate instead.
//!
//! ## Features
//!
//! This implementation provides:
//! - gRPC client functionality over HTTP/2+TLS
//! - Unary and server-streaming request support
//! - Protocol Buffer message encoding/decoding integration
//! - Connection management and metrics
//! - SSL/TLS support via OpenSSL
//!
//! ## Example
//!
//! ```rust,no_run
//! use fd_grpc_sys::*;
//! use std::mem::MaybeUninit;
//!
//! unsafe {
//!     // Create a gRPC client
//!     let align = fd_grpc_client_align();
//!     let footprint = fd_grpc_client_footprint(1024);
//!     
//!     // Allocate memory and initialize client...
//!     // (This is a simplified example - real usage requires proper setup)
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_status_constants() {
        assert_eq!(FD_GRPC_STATUS_OK, 0);
        assert_eq!(FD_GRPC_STATUS_CANCELLED, 1);
        assert_eq!(FD_GRPC_STATUS_UNKNOWN, 2);
        assert_eq!(FD_GRPC_STATUS_INVALID_ARGUMENT, 3);
        assert_eq!(FD_GRPC_STATUS_DEADLINE_EXCEEDED, 4);
        assert_eq!(FD_GRPC_STATUS_NOT_FOUND, 5);
    }

    #[test]
    fn test_grpc_client_constants() {
        assert_eq!(FD_GRPC_CLIENT_MAX_STREAMS, 8);
        assert!(FD_GRPC_CLIENT_VERSION_LEN_MAX > 0);
    }

    #[test]
    fn test_grpc_deadline_constants() {
        assert_eq!(FD_GRPC_DEADLINE_HEADER, 1);
        assert_eq!(FD_GRPC_DEADLINE_RX_END, 2);
    }

    #[test]
    fn test_grpc_client_functions_exist() {
        let align_fn: unsafe extern "C" fn() -> ulong = fd_grpc_client_align;
        let footprint_fn: unsafe extern "C" fn(ulong) -> ulong = fd_grpc_client_footprint;
        assert!(!(align_fn as *const u8).is_null());
        assert!(!(footprint_fn as *const u8).is_null());
    }

    #[test]
    fn test_grpc_client_sizing() {
        unsafe {
            let align = fd_grpc_client_align();
            assert!(align > 0);
            assert!(align.is_power_of_two());

            let footprint = fd_grpc_client_footprint(1024);
            assert!(footprint > 0);
        }
    }
}
