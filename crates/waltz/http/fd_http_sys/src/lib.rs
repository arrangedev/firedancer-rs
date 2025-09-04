//! Raw FFI bindings to Firedancer's HTTP server implementation
//!
//! This crate provides unsafe, low-level bindings to the HTTP server implementation
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
//! For a safe, idiomatic Rust API, use the `fd_http` crate instead.
//!
//! ## Features
//!
//! This implementation provides:
//! - HTTP/1.1 server functionality with WebSocket support
//! - High-performance request handling with ring buffer architecture
//! - URL parsing and handling utilities
//! - picohttpparser integration for fast HTTP parsing
//! - Connection management and metrics
//! - Broadcasting capabilities for WebSocket clients
//!
//! ## Example
//!
//! ```rust,no_run
//! use fd_http_sys::*;
//! use std::mem::MaybeUninit;
//!
//! unsafe {
//!     // Create HTTP server parameters
//!     let params = fd_http_server_params_t {
//!         max_connection_cnt: 100,
//!         max_ws_connection_cnt: 50,
//!         max_request_len: 8192,
//!         max_ws_recv_frame_len: 8192,
//!         max_ws_send_frame_cnt: 1000,
//!         outgoing_buffer_sz: 1024 * 1024,
//!     };
//!     
//!     let align = fd_http_server_align();
//!     let footprint = fd_http_server_footprint(params);
//!     
//!     // Allocate memory and initialize server...
//!     // (This is a simplified example - real usage requires proper setup)
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_server_constants() {
        assert_eq!(FD_HTTP_SERVER_METHOD_GET, 0);
        assert_eq!(FD_HTTP_SERVER_METHOD_POST, 1);
        assert_eq!(FD_HTTP_SERVER_METHOD_OPTIONS, 2);
        assert_eq!(FD_HTTP_SERVER_METHOD_PUT, 3);
    }

    #[test]
    fn test_http_connection_close_constants() {
        assert_eq!(FD_HTTP_SERVER_CONNECTION_CLOSE_OK, -1);
        assert_eq!(FD_HTTP_SERVER_CONNECTION_CLOSE_EVICTED, -2);
        assert_eq!(FD_HTTP_SERVER_CONNECTION_CLOSE_TOO_SLOW, -3);
        assert_eq!(FD_HTTP_SERVER_CONNECTION_CLOSE_EXPECTED_EOF, -4);
    }

    #[test]
    fn test_url_constants() {
        assert_eq!(FD_URL_SUCCESS, 0);
        assert_eq!(FD_URL_ERR_SCHEME, 1);
        assert_eq!(FD_URL_ERR_HOST_OVERSZ, 2);
        assert_eq!(FD_URL_ERR_USERINFO, 3);
    }

    #[test]
    fn test_http_server_functions_exist() {
        // Test that key functions are available (we can't call them without proper setup)
        let align_fn: unsafe extern "C" fn() -> ulong = fd_http_server_align;
        let footprint_fn: unsafe extern "C" fn(fd_http_server_params_t) -> ulong =
            fd_http_server_footprint;

        // Just check that the function pointers are not null
        assert!(!(align_fn as *const u8).is_null());
        assert!(!(footprint_fn as *const u8).is_null());
    }

    #[test]
    fn test_http_server_sizing() {
        unsafe {
            let align = fd_http_server_align();
            assert!(align > 0);
            assert!(align.is_power_of_two());

            let params = fd_http_server_params_t {
                max_connection_cnt: 10,
                max_ws_connection_cnt: 5,
                max_request_len: 4096,
                max_ws_recv_frame_len: 4096,
                max_ws_send_frame_cnt: 100,
                outgoing_buffer_sz: 65536,
            };

            let footprint = fd_http_server_footprint(params);
            assert!(footprint > 0);
        }
    }

    #[test]
    fn test_url_functions_exist() {
        let parse_fn: unsafe extern "C" fn(
            *mut fd_url_t,
            *const i8,
            ulong,
            *mut i32,
        ) -> *mut fd_url_t = fd_url_parse_cstr;
        let unescape_fn: unsafe extern "C" fn(*mut i8, ulong) -> ulong = fd_url_unescape;

        // Just check that the function pointers are not null
        assert!(!(parse_fn as *const u8).is_null());
        assert!(!(unescape_fn as *const u8).is_null());
    }

    #[test]
    fn test_method_string_functions() {
        unsafe {
            let method_str = fd_http_server_method_str(FD_HTTP_SERVER_METHOD_GET as u8);
            assert!(!method_str.is_null());

            let close_reason_str = fd_http_server_connection_close_reason_str(
                FD_HTTP_SERVER_CONNECTION_CLOSE_OK as i32,
            );
            assert!(!close_reason_str.is_null());
        }
    }
}
