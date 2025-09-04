//! Raw FFI bindings to Firedancer's nanopb implementation
//!
//! This crate provides unsafe, low-level bindings to the nanopb Protocol Buffers
//! implementation used by Firedancer. These bindings are generated automatically
//! from the C headers using bindgen.
//!
//! ## Safety
//!
//! All functions in this crate are `unsafe` and require careful handling of:
//! - Memory management (allocation/deallocation)
//! - Pointer validity and lifetimes
//! - Thread safety
//! - Proper initialization of structures
//!
//! For a safe, idiomatic Rust API, use the `fd_nanopb` crate instead.
//!
//! ## Configuration
//!
//! This implementation is configured with:
//! - `PB_FIELD_32BIT=1`: Support for large messages and field numbers > 65536
//! - `PB_ENABLE_MALLOC=1`: Dynamic allocation support
//! - `PB_BUFFER_ONLY=1`: Buffer-only mode (no custom streams)
//!
//! ## Example
//!
//! ```rust,no_run
//! use fd_nanopb_sys::*;
//! use std::mem::MaybeUninit;
//!
//! unsafe {
//!     // Create a buffer for encoding
//!     let mut buffer = [0u8; 1024];
//!     let mut stream = pb_ostream_from_buffer(buffer.as_mut_ptr(), buffer.len());
//!     
//!     // Encoding would happen here with pb_encode()...
//!     
//!     println!("Encoded {} bytes", (*stream).bytes_written);
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
    fn test_stream() {
        let mut buffer = [0u8; 64];
        let stream = unsafe { pb_ostream_from_buffer(buffer.as_mut_ptr(), buffer.len()) };
        assert_eq!(stream.max_size, 64);
        assert_eq!(stream.bytes_written, 0);
    }

    #[test]
    fn test_istream() {
        let buffer = [0u8; 64];
        let stream = unsafe { pb_istream_from_buffer(buffer.as_ptr(), buffer.len()) };
        assert_eq!(stream.bytes_left, 64);
    }

    #[test]
    fn test_wiretypes() {
        assert_eq!(pb_wire_type_t_PB_WT_VARINT, 0);
        assert_eq!(pb_wire_type_t_PB_WT_64BIT, 1);
        assert_eq!(pb_wire_type_t_PB_WT_STRING, 2);
        assert_eq!(pb_wire_type_t_PB_WT_32BIT, 5);
    }
}
