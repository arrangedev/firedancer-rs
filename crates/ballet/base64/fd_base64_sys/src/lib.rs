//! Raw FFI bindings to Firedancer Base64 utilities
//!
//! This crate provides low-level, unsafe bindings to the Firedancer Base64 encoding/decoding utilities:
//! - High-performance Base64 encoding and decoding
//! - Standard Base64 alphabet with padding support (RFC 4648)
//! - Optimized for bulk data processing
//! - Zero-copy operations where possible
//!
//! For a safe Rust API, consider using the higher-level `fd_base64` wrapper crate.
//!
//! # Example
//!
//! ```rust,no_run
//! use fd_base64_sys::*;
//! use std::ffi::CString;
//!
//! unsafe {
//!     let input = b"Hello, World!";
//!     let encoded_size = FD_BASE64_ENC_SZ(input.len() as u64) as usize;
//!     let mut encoded = vec![0u8; encoded_size];
//!     
//!     let encoded_len = fd_base64_encode(
//!         encoded.as_mut_ptr() as *mut i8,
//!         input.as_ptr() as *const std::ffi::c_void,
//!         input.len() as u64,
//!     );
//!     
//!     encoded.truncate(encoded_len as usize);
//!     
//!     // Decode
//!     let decoded_size = FD_BASE64_DEC_SZ(encoded_len) as usize;
//!     let mut decoded = vec![0u8; decoded_size];
//!     
//!     let decoded_len = fd_base64_decode(
//!         decoded.as_mut_ptr(),
//!         encoded.as_ptr() as *const i8,
//!         encoded_len,
//!     );
//!     
//!     if decoded_len >= 0 {
//!         decoded.truncate(decoded_len as usize);
//!         assert_eq!(&decoded, input);
//!     }
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// Calculate the encoded size for a given byte count
/// Equivalent to the C macro: FD_BASE64_ENC_SZ(sz) ((((sz)+2UL)/3UL)*4UL)
pub const fn FD_BASE64_ENC_SZ(sz: ulong) -> ulong {
    ((sz + 2) / 3) * 4
}

/// Calculate the max decoded size for a given encoded character count
/// Equivalent to the C macro: FD_BASE64_DEC_SZ(sz) ((((sz)+3UL)/4UL)*3UL)
pub const fn FD_BASE64_DEC_SZ(sz: ulong) -> ulong {
    ((sz + 3) / 4) * 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_macros() {
        // Test encoding size calculation
        assert_eq!(FD_BASE64_ENC_SZ(0), 0);
        assert_eq!(FD_BASE64_ENC_SZ(1), 4);
        assert_eq!(FD_BASE64_ENC_SZ(2), 4);
        assert_eq!(FD_BASE64_ENC_SZ(3), 4);
        assert_eq!(FD_BASE64_ENC_SZ(4), 8);

        // Test decoding size calculation
        assert_eq!(FD_BASE64_DEC_SZ(0), 0);
        assert_eq!(FD_BASE64_DEC_SZ(4), 3);
        assert_eq!(FD_BASE64_DEC_SZ(8), 6);
    }

    #[test]
    fn test_base64_encode_decode_empty() {
        unsafe {
            let input = b"";
            let encoded_size = FD_BASE64_ENC_SZ(input.len() as u64) as usize;
            let mut encoded = vec![0u8; encoded_size.max(1)];

            let encoded_len = fd_base64_encode(
                encoded.as_mut_ptr() as *mut core::ffi::c_char,
                input.as_ptr() as *const core::ffi::c_void,
                input.len() as u64,
            );

            assert_eq!(encoded_len, 0);
        }
    }

    #[test]
    fn test_base64_encode_decode_basic() {
        unsafe {
            let input = b"Hello";
            let encoded_size = FD_BASE64_ENC_SZ(input.len() as u64) as usize;
            let mut encoded = vec![0u8; encoded_size];

            let encoded_len = fd_base64_encode(
                encoded.as_mut_ptr() as *mut core::ffi::c_char,
                input.as_ptr() as *const core::ffi::c_void,
                input.len() as u64,
            );

            assert!(encoded_len > 0);
            encoded.truncate(encoded_len as usize);

            // Expected: "Hello" -> "SGVsbG8="
            let expected_encoded = b"SGVsbG8=";
            assert_eq!(encoded, expected_encoded);

            // Now decode
            let decoded_size = FD_BASE64_DEC_SZ(encoded_len) as usize;
            let mut decoded = vec![0u8; decoded_size];

            let decoded_len = fd_base64_decode(
                decoded.as_mut_ptr(),
                encoded.as_ptr() as *const core::ffi::c_char,
                encoded_len,
            );

            assert!(decoded_len >= 0);
            decoded.truncate(decoded_len as usize);
            assert_eq!(&decoded, input);
        }
    }

    #[test]
    fn test_base64_encode_decode_various_sizes() {
        let test_cases: &[&[u8]] = &[
            b"",
            b"f",
            b"fo",
            b"foo",
            b"foob",
            b"fooba",
            b"foobar",
            b"Hello, World!",
            b"The quick brown fox jumps over the lazy dog",
        ];

        for &input in test_cases {
            unsafe {
                let encoded_size = FD_BASE64_ENC_SZ(input.len() as u64) as usize;
                let mut encoded = vec![0u8; encoded_size.max(1)];

                let encoded_len = fd_base64_encode(
                    encoded.as_mut_ptr() as *mut core::ffi::c_char,
                    input.as_ptr() as *const core::ffi::c_void,
                    input.len() as u64,
                );

                if input.is_empty() {
                    assert_eq!(encoded_len, 0);
                    continue;
                }

                assert!(encoded_len > 0);
                encoded.truncate(encoded_len as usize);

                let decoded_size = FD_BASE64_DEC_SZ(encoded_len) as usize;
                let mut decoded = vec![0u8; decoded_size];

                let decoded_len = fd_base64_decode(
                    decoded.as_mut_ptr(),
                    encoded.as_ptr() as *const core::ffi::c_char,
                    encoded_len,
                );

                assert!(
                    decoded_len >= 0,
                    "Failed to decode for input: {:?}",
                    core::str::from_utf8(input)
                );
                decoded.truncate(decoded_len as usize);
                assert_eq!(
                    &decoded,
                    input,
                    "Round-trip failed for input: {:?}",
                    core::str::from_utf8(input)
                );
            }
        }
    }

    #[test]
    fn test_base64_decode_invalid() {
        unsafe {
            let invalid_inputs: &[&[u8]] = &[
                b"SGVsbG8@", // Invalid character
                b"@@@",      // All invalid characters
            ];

            for &input in invalid_inputs {
                let decoded_size = FD_BASE64_DEC_SZ(input.len() as u64) as usize;
                let mut decoded = vec![0u8; decoded_size.max(1)];

                let decoded_len = fd_base64_decode(
                    decoded.as_mut_ptr(),
                    input.as_ptr() as *const core::ffi::c_char,
                    input.len() as u64,
                );

                // Should return -1 for invalid input
                assert_eq!(
                    decoded_len,
                    -1,
                    "Should fail for invalid input: {:?}",
                    core::str::from_utf8(input)
                );
            }
        }
    }

    #[test]
    fn test_bindings_exist() {
        // Just test that the functions exist and can be called
        let _enc_sz = FD_BASE64_ENC_SZ(10);
        let _dec_sz = FD_BASE64_DEC_SZ(12);
    }
}
