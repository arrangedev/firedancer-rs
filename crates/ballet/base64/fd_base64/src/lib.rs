//! Safe Rust API for Firedancer Base64 utilities
//!
//! This crate provides safe abstractions over the raw FFI bindings in `fd_base64_sys`.
//! The Base64 implementation follows RFC 4648 standard with padding support and provides
//! high-performance encoding and decoding operations.
//!
//! ## Features
//!
//! - **RFC 4648 compliant**: Standard Base64 alphabet with padding
//! - **High performance**: Optimized C implementation with Rust safety
//! - **Zero-copy where possible**: Efficient memory usage
//! - **Safe API**: All unsafe operations are encapsulated
//! - **Comprehensive error handling**: Clear error messages for invalid input
//!
//! ## Usage
//!
//! ### Basic encoding and decoding
//!
//! ```rust
//! use fd_base64::{encode, decode};
//!
//! let data = b"Hello, World!";
//! let encoded = encode(data);
//! let decoded = decode(&encoded).unwrap();
//! assert_eq!(decoded, data);
//! ```
//!
//! ### Working with strings
//!
//! ```rust
//! use fd_base64::{encode_string, decode_string};
//!
//! let text = "Hello, World!";
//! let encoded = encode_string(text);
//! let decoded = decode_string(&encoded).unwrap();
//! assert_eq!(decoded, text);
//! ```
//!
//! ### In-place operations
//!
//! ```rust
//! use fd_base64::{encode_to_vec, decode_to_vec};
//!
//! let data = b"Hello, World!";
//! let mut encoded = Vec::new();
//! encode_to_vec(data, &mut encoded);
//!
//! let mut decoded = Vec::new();
//! decode_to_vec(&encoded, &mut decoded).unwrap();
//! assert_eq!(decoded, data);
//! ```

use fd_base64_sys as sys;

/// Error types that can occur during Base64 operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Base64Error {
    /// The input contains invalid Base64 characters
    InvalidCharacter,
    /// The input has invalid length or padding
    InvalidLength,
    /// The input contains invalid padding
    InvalidPadding,
    /// The output buffer is too small
    BufferTooSmall,
    /// Invalid input parameters
    InvalidInput,
}

impl std::fmt::Display for Base64Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Base64Error::InvalidCharacter => write!(f, "Invalid Base64 character"),
            Base64Error::InvalidLength => write!(f, "Invalid Base64 length"),
            Base64Error::InvalidPadding => write!(f, "Invalid Base64 padding"),
            Base64Error::BufferTooSmall => write!(f, "Output buffer too small"),
            Base64Error::InvalidInput => write!(f, "Invalid input parameters"),
        }
    }
}

impl std::error::Error for Base64Error {}

/// Calculate the encoded size for a given input length
///
/// This returns the exact number of characters needed to encode `input_len` bytes.
/// The result includes padding characters.
pub fn encoded_size(input_len: usize) -> usize {
    sys::FD_BASE64_ENC_SZ(input_len as sys::ulong) as usize
}

/// Calculate the maximum decoded size for a given encoded length
///
/// This returns the maximum number of bytes that could result from decoding
/// `encoded_len` Base64 characters. The actual decoded length may be smaller
/// due to padding.
pub fn decoded_size(encoded_len: usize) -> usize {
    sys::FD_BASE64_DEC_SZ(encoded_len as sys::ulong) as usize
}

/// Encode bytes to Base64
///
/// Returns a vector containing the Base64-encoded representation of the input.
/// The output uses the standard Base64 alphabet with padding.
///
/// # Examples
///
/// ```rust
/// use fd_base64::encode;
///
/// let data = b"Hello, World!";
/// let encoded = encode(data);
/// assert_eq!(encoded, b"SGVsbG8sIFdvcmxkIQ==");
/// ```
pub fn encode(input: &[u8]) -> Vec<u8> {
    if input.is_empty() {
        return Vec::new();
    }

    let output_len = encoded_size(input.len());
    let mut output = vec![0u8; output_len];

    let actual_len = unsafe {
        sys::fd_base64_encode(
            output.as_mut_ptr() as *mut i8,
            input.as_ptr() as *const std::ffi::c_void,
            input.len() as sys::ulong,
        )
    };

    output.truncate(actual_len as usize);
    output
}

/// Encode bytes to Base64 and append to an existing vector
///
/// This is more efficient than `encode` when you want to append to an existing buffer.
///
/// # Examples
///
/// ```rust
/// use fd_base64::encode_to_vec;
///
/// let data = b"Hello, World!";
/// let mut output = Vec::new();
/// encode_to_vec(data, &mut output);
/// assert_eq!(output, b"SGVsbG8sIFdvcmxkIQ==");
/// ```
pub fn encode_to_vec(input: &[u8], output: &mut Vec<u8>) {
    if input.is_empty() {
        return;
    }

    let start_len = output.len();
    let encoded_len = encoded_size(input.len());
    output.resize(start_len + encoded_len, 0);

    let actual_len = unsafe {
        sys::fd_base64_encode(
            output.as_mut_ptr().add(start_len) as *mut i8,
            input.as_ptr() as *const std::ffi::c_void,
            input.len() as sys::ulong,
        )
    };

    output.truncate(start_len + actual_len as usize);
}

/// Encode a string to Base64
///
/// This is a convenience function that encodes a string's UTF-8 bytes.
///
/// # Examples
///
/// ```rust
/// use fd_base64::encode_string;
///
/// let text = "Hello, World!";
/// let encoded = encode_string(text);
/// assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
/// ```
pub fn encode_string(input: &str) -> String {
    let encoded_bytes = encode(input.as_bytes());
    // SAFETY: Base64 output is always valid ASCII/UTF-8
    unsafe { String::from_utf8_unchecked(encoded_bytes) }
}

/// Decode Base64 bytes
///
/// Returns a vector containing the decoded bytes, or an error if the input is invalid.
///
/// # Examples
///
/// ```rust
/// use fd_base64::decode;
///
/// let encoded = b"SGVsbG8sIFdvcmxkIQ==";
/// let decoded = decode(encoded).unwrap();
/// assert_eq!(decoded, b"Hello, World!");
/// ```
pub fn decode(input: &[u8]) -> Result<Vec<u8>, Base64Error> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let max_output_len = decoded_size(input.len());
    let mut output = vec![0u8; max_output_len];

    let decoded_len = unsafe {
        sys::fd_base64_decode(
            output.as_mut_ptr(),
            input.as_ptr() as *const i8,
            input.len() as sys::ulong,
        )
    };

    if decoded_len < 0 {
        // Determine the type of error based on the input
        if input.len() % 4 != 0 {
            return Err(Base64Error::InvalidLength);
        }

        // Check for invalid characters
        for &byte in input {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=' => {}
                _ => return Err(Base64Error::InvalidCharacter),
            }
        }

        // If we get here, it's likely a padding issue
        return Err(Base64Error::InvalidPadding);
    }

    output.truncate(decoded_len as usize);
    Ok(output)
}

/// Decode Base64 bytes and append to an existing vector
///
/// This is more efficient than `decode` when you want to append to an existing buffer.
///
/// # Examples
///
/// ```rust
/// use fd_base64::decode_to_vec;
///
/// let encoded = b"SGVsbG8sIFdvcmxkIQ==";
/// let mut output = Vec::new();
/// decode_to_vec(encoded, &mut output).unwrap();
/// assert_eq!(output, b"Hello, World!");
/// ```
pub fn decode_to_vec(input: &[u8], output: &mut Vec<u8>) -> Result<(), Base64Error> {
    if input.is_empty() {
        return Ok(());
    }

    let start_len = output.len();
    let max_decoded_len = decoded_size(input.len());
    output.resize(start_len + max_decoded_len, 0);

    let decoded_len = unsafe {
        sys::fd_base64_decode(
            output.as_mut_ptr().add(start_len),
            input.as_ptr() as *const i8,
            input.len() as sys::ulong,
        )
    };

    if decoded_len < 0 {
        output.truncate(start_len); // Restore original length

        if input.len() % 4 != 0 {
            return Err(Base64Error::InvalidLength);
        }

        for &byte in input {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=' => {}
                _ => return Err(Base64Error::InvalidCharacter),
            }
        }

        return Err(Base64Error::InvalidPadding);
    }

    output.truncate(start_len + decoded_len as usize);
    Ok(())
}

/// Decode a Base64 string
///
/// This is a convenience function that decodes a Base64 string and returns the result as a UTF-8 string.
///
/// # Examples
///
/// ```rust
/// use fd_base64::decode_string;
///
/// let encoded = "SGVsbG8sIFdvcmxkIQ==";
/// let decoded = decode_string(encoded).unwrap();
/// assert_eq!(decoded, "Hello, World!");
/// ```
pub fn decode_string(input: &str) -> Result<String, Base64Error> {
    let decoded_bytes = decode(input.as_bytes())?;
    String::from_utf8(decoded_bytes).map_err(|_| Base64Error::InvalidInput)
}

/// In-place encoding utilities
pub mod inplace {
    use super::*;

    /// Encode data in-place into a pre-allocated buffer
    ///
    /// Returns the number of bytes written to the output buffer.
    /// The output buffer must be at least `encoded_size(input.len())` bytes long.
    ///
    /// # Errors
    ///
    /// Returns `Base64Error::BufferTooSmall` if the output buffer is too small.
    pub fn encode(input: &[u8], output: &mut [u8]) -> Result<usize, Base64Error> {
        if input.is_empty() {
            return Ok(0);
        }

        let required_len = encoded_size(input.len());
        if output.len() < required_len {
            return Err(Base64Error::BufferTooSmall);
        }

        let actual_len = unsafe {
            sys::fd_base64_encode(
                output.as_mut_ptr() as *mut i8,
                input.as_ptr() as *const std::ffi::c_void,
                input.len() as sys::ulong,
            )
        };

        Ok(actual_len as usize)
    }

    /// Decode data in-place into a pre-allocated buffer
    ///
    /// Returns the number of bytes written to the output buffer.
    /// The output buffer must be at least `decoded_size(input.len())` bytes long.
    ///
    /// # Errors
    ///
    /// Returns `Base64Error::BufferTooSmall` if the output buffer is too small,
    /// or other `Base64Error` variants for invalid input.
    pub fn decode(input: &[u8], output: &mut [u8]) -> Result<usize, Base64Error> {
        if input.is_empty() {
            return Ok(0);
        }

        let max_required_len = decoded_size(input.len());
        if output.len() < max_required_len {
            return Err(Base64Error::BufferTooSmall);
        }

        let decoded_len = unsafe {
            sys::fd_base64_decode(
                output.as_mut_ptr(),
                input.as_ptr() as *const i8,
                input.len() as sys::ulong,
            )
        };

        if decoded_len < 0 {
            if input.len() % 4 != 0 {
                return Err(Base64Error::InvalidLength);
            }

            for &byte in input {
                match byte {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=' => {}
                    _ => return Err(Base64Error::InvalidCharacter),
                }
            }

            return Err(Base64Error::InvalidPadding);
        }

        Ok(decoded_len as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_calculations() {
        assert_eq!(encoded_size(0), 0);
        assert_eq!(encoded_size(1), 4);
        assert_eq!(encoded_size(2), 4);
        assert_eq!(encoded_size(3), 4);
        assert_eq!(encoded_size(4), 8);

        assert_eq!(decoded_size(0), 0);
        assert_eq!(decoded_size(4), 3);
        assert_eq!(decoded_size(8), 6);
    }

    #[test]
    fn test_encode_decode_empty() {
        let empty: &[u8] = &[];
        let encoded = encode(empty);
        assert!(encoded.is_empty());

        let decoded = decode(&encoded).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_encode_decode_basic() {
        let test_cases = [
            (b"" as &[u8], b"" as &[u8]),
            (b"f", b"Zg=="),
            (b"fo", b"Zm8="),
            (b"foo", b"Zm9v"),
            (b"foob", b"Zm9vYg=="),
            (b"fooba", b"Zm9vYmE="),
            (b"foobar", b"Zm9vYmFy"),
            (b"Hello, World!", b"SGVsbG8sIFdvcmxkIQ=="),
        ];

        for (input, expected) in test_cases {
            let encoded = encode(input);
            assert_eq!(
                encoded,
                expected,
                "Encoding failed for: {:?}",
                std::str::from_utf8(input)
            );

            let decoded = decode(&encoded).unwrap();
            assert_eq!(
                decoded,
                input,
                "Decoding failed for: {:?}",
                std::str::from_utf8(input)
            );
        }
    }

    #[test]
    fn test_string_functions() {
        let text = "Hello, World!";
        let encoded = encode_string(text);
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");

        let decoded = decode_string(&encoded).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_to_vec_functions() {
        let input = b"Hello, World!";

        let mut encoded = Vec::new();
        encode_to_vec(input, &mut encoded);
        assert_eq!(encoded, b"SGVsbG8sIFdvcmxkIQ==");

        let mut decoded = Vec::new();
        decode_to_vec(&encoded, &mut decoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_decode_errors() {
        let invalid_cases: &[(&[u8], Base64Error)] = &[
            (b"SGVs@G8=", Base64Error::InvalidCharacter), // Invalid character
            (b"@@@=", Base64Error::InvalidCharacter),     // All invalid characters
        ];

        for (input, expected_error) in invalid_cases {
            let result = decode(input);
            assert!(
                result.is_err(),
                "Should fail for: {:?}",
                std::str::from_utf8(input)
            );

            let error = result.unwrap_err();
            assert_eq!(
                error,
                *expected_error,
                "Wrong error type for: {:?}",
                std::str::from_utf8(input)
            );
        }
    }

    #[test]
    fn test_inplace_encode() {
        let input = b"Hello, World!";
        let mut output = vec![0u8; encoded_size(input.len())];

        let len = inplace::encode(input, &mut output).unwrap();
        output.truncate(len);
        assert_eq!(output, b"SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn test_inplace_decode() {
        let input = b"SGVsbG8sIFdvcmxkIQ==";
        let mut output = vec![0u8; decoded_size(input.len())];

        let len = inplace::decode(input, &mut output).unwrap();
        output.truncate(len);
        assert_eq!(output, b"Hello, World!");
    }

    #[test]
    fn test_inplace_buffer_too_small() {
        let input = b"Hello, World!";
        let mut small_output = vec![0u8; 1];

        let result = inplace::encode(input, &mut small_output);
        assert_eq!(result.unwrap_err(), Base64Error::BufferTooSmall);
    }

    #[test]
    fn test_round_trip_various_sizes() {
        for size in 0..100 {
            let input: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

            let encoded = encode(&input);
            let decoded = decode(&encoded).unwrap();

            assert_eq!(decoded, input, "Round-trip failed for size {}", size);
        }
    }

    #[test]
    fn test_binary_data() {
        let binary_data = [0u8, 1, 2, 3, 255, 254, 253, 128, 127];

        let encoded = encode(&binary_data);
        let decoded = decode(&encoded).unwrap();

        assert_eq!(decoded, binary_data);
    }
}
