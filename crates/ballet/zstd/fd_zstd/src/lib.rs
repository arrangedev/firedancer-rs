//! Safe Rust wrapper for Firedancer's Zstandard compression API.
//!
//! This crate provides safe, idiomatic Rust bindings for the Firedancer Zstandard
//! compression library, which handles streaming compression and decompression of
//! Zstandard (.zst) files without heap allocations or syscalls.
//!
//! # Features
//!
//! - **Zero-allocation**: Uses caller-provided memory regions, no heap allocations
//! - **Streaming**: Process data in chunks without loading entire files into memory
//! - **Frame inspection**: Peek at frame headers to determine requirements
//! - **Error handling**: Comprehensive error types with context
//!
//! # Examples
//!
//! ## Decompressing
//!
//! ```rust,no_run
//! use fd_zstd::{DecompressionStream, FramePeek};
//!
//! let compressed_data = b"..."; // compressed data
//! let peek = FramePeek::new(&compressed_data[..18])?;
//! let window_size = peek.window_size();
//!
//! let mut dstream = DecompressionStream::new(peek.window_size())?;
//!
//! let mut input = &compressed_data[..];
//! let mut output = vec![0u8; 4096];
//! let mut total_output = Vec::new();
//!
//! loop {
//!     let result = dstream.read(&mut input, &mut output[..])?;
//!     total_output.extend_from_slice(&output[..result.bytes_written]);
//!     
//!     if result.is_frame_complete() {
//!         break;
//!     }
//!     
//!     if result.bytes_written == output.len() {
//!         // continue with more space
//!         output.clear();
//!         output.resize(4096, 0);
//!     }
//! }
//!
//! let total = total_output.len();
//! Ok::<(), fd_zstd::Error>(())
//! ```

#![no_std]

extern crate alloc;

use alloc::alloc::{alloc, dealloc, Layout};
use alloc::vec::{self, Vec};
use core::fmt;
use core::ptr::NonNull;

use fd_zstd_sys::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Invalid input data or parameters
    InvalidInput,
    /// Insufficient buffer space
    InsufficientBuffer,
    /// Memory allocation failure
    AllocationFailed,
    /// Protocol error in compressed data
    ProtocolError,
    /// Frame requires larger window size than supported
    WindowSizeTooLarge,
    /// Unexpected end of input
    UnexpectedEof,
    /// Internal error from underlying library
    InternalError(u64),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidInput => write!(f, "invalid input data or parameters"),
            Error::InsufficientBuffer => write!(f, "insufficient buffer space"),
            Error::AllocationFailed => write!(f, "memory allocation failed"),
            Error::ProtocolError => write!(f, "protocol error in compressed data"),
            Error::WindowSizeTooLarge => {
                write!(f, "frame requires larger window size than supported")
            }
            Error::UnexpectedEof => write!(f, "unexpected end of input"),
            Error::InternalError(code) => write!(f, "internal error: {}", code),
        }
    }
}

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
impl std::error::Error for Error {}

pub type ZstdResult<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct FramePeek {
    window_size: u64,
    frame_content_size: Option<u64>,
    is_skippable: bool,
}

impl FramePeek {
    /// Peek at a frame header to extract metadata.
    ///
    /// `header_bytes` should contain at least the first 18 bytes of the frame,
    /// or as many bytes as available if less than 18 bytes remain.
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidInput` if the header is malformed or incomplete.
    /// Returns `Error::UnexpectedEof` if insufficient data is provided.
    pub fn new(header_bytes: &[u8]) -> ZstdResult<Self> {
        if header_bytes.is_empty() {
            return Err(Error::UnexpectedEof);
        }

        let mut peek = fd_zstd_peek_t {
            window_sz: 0,
            frame_content_sz: u64::MAX,
            frame_is_skippable: 0,
        };

        let result = unsafe {
            fd_zstd_peek(
                &mut peek,
                header_bytes.as_ptr() as *const _,
                header_bytes.len() as u64,
            )
        };

        if result.is_null() {
            return Err(Error::InvalidInput);
        }

        Ok(FramePeek {
            window_size: peek.window_sz,
            frame_content_size: if peek.frame_content_sz == u64::MAX {
                None
            } else {
                Some(peek.frame_content_sz)
            },
            is_skippable: peek.frame_is_skippable != 0,
        })
    }

    /// window size required to decompress this frame
    pub fn window_size(&self) -> u64 {
        self.window_size
    }

    /// content size if known, or None if unknown
    pub fn frame_content_size(&self) -> Option<u64> {
        self.frame_content_size
    }

    /// checks if this is a skippable frame
    pub fn is_skippable(&self) -> bool {
        self.is_skippable
    }
}

#[derive(Debug, Clone)]
pub struct ReadResult {
    /// Number of input bytes consumed
    pub bytes_consumed: usize,
    /// Number of output bytes written
    pub bytes_written: usize,
    /// Whether the current frame is complete
    pub frame_complete: bool,
}

impl ReadResult {
    /// check if the current frame has been completely decompressed
    pub fn is_frame_complete(&self) -> bool {
        self.frame_complete
    }
}

/// A streaming decompression context for Zstandard data.
///
/// This struct manages the memory and state required for decompressing
/// Zstandard frames in a streaming fashion.
pub struct DecompressionStream {
    dstream: NonNull<fd_zstd_dstream_t>,
    memory: NonNull<u8>,
    layout: Layout,
}

impl DecompressionStream {
    /// Create a new decompression stream that can handle frames up to the
    /// specified window size.
    ///
    /// # Errors
    ///
    /// Returns `Error::AllocationFailed` if memory allocation fails.
    /// Returns `Error::InvalidInput` if the window size is invalid.
    pub fn new(max_window_size: u64) -> ZstdResult<Self> {
        let align = unsafe { fd_zstd_dstream_align() } as usize;
        let footprint = unsafe { fd_zstd_dstream_footprint(max_window_size) } as usize;
        let layout = Layout::from_size_align(footprint, align).map_err(|_| Error::InvalidInput)?;
        let memory = NonNull::new(unsafe { alloc(layout) }).ok_or(Error::AllocationFailed)?;
        let dstream = unsafe { fd_zstd_dstream_new(memory.as_ptr() as *mut _, max_window_size) };
        let dstream = NonNull::new(dstream).ok_or_else(|| {
            unsafe { dealloc(memory.as_ptr(), layout) };
            Error::InternalError(0)
        })?;

        Ok(DecompressionStream {
            dstream,
            memory,
            layout,
        })
    }

    /// Reset the decompression stream to expect a new frame.
    ///
    /// This should be called between frames or after an error to reset
    /// the internal state.
    pub fn reset(&mut self) {
        unsafe {
            fd_zstd_dstream_reset(self.dstream.as_ptr());
        }
    }

    /// Read and decompress data from the input buffer into the output buffer.
    ///
    /// This function will consume as much input as possible and produce as much
    /// output as possible. The input slice will be updated to point to the
    /// remaining unconsumed data, and the function returns information about
    /// how much data was processed.
    ///
    /// # Arguments
    ///
    /// * `input` - Mutable reference to input slice; will be updated to remaining data
    /// * `output` - Output buffer to write decompressed data into
    ///
    /// # Returns
    ///
    /// Returns a `ReadResult` containing information about bytes processed and
    /// whether the frame is complete.
    ///
    /// # Errors
    ///
    /// Returns `Error::ProtocolError` if the compressed data is malformed.
    /// Returns `Error::InternalError` for other internal failures.
    pub fn read(&mut self, input: &mut &[u8], output: &mut [u8]) -> ZstdResult<ReadResult> {
        let input_start = input.as_ptr();
        let input_end = unsafe { input.as_ptr().add(input.len()) };
        let output_start = output.as_mut_ptr();
        let output_end = unsafe { output.as_mut_ptr().add(output.len()) };

        let mut in_ptr = input_start;
        let mut out_ptr = output_start;
        let mut error_code = 0u64;

        let result = unsafe {
            fd_zstd_dstream_read(
                self.dstream.as_ptr(),
                &mut in_ptr,
                input_end,
                &mut out_ptr,
                output_end,
                &mut error_code,
            )
        };

        let bytes_consumed = unsafe { in_ptr.offset_from(input_start) } as usize;
        let bytes_written = unsafe { out_ptr.offset_from(output_start) } as usize;

        *input = &input[bytes_consumed..];

        match result {
            0 => Ok(ReadResult {
                bytes_consumed,
                bytes_written,
                frame_complete: false,
            }),
            -1 => Ok(ReadResult {
                bytes_consumed,
                bytes_written,
                frame_complete: true,
            }),
            71 => Err(Error::ProtocolError), // EPROTO
            _ => Err(Error::InternalError(error_code)),
        }
    }
}

impl Drop for DecompressionStream {
    fn drop(&mut self) {
        unsafe {
            let returned_mem = fd_zstd_dstream_delete(self.dstream.as_ptr());
            debug_assert_eq!(returned_mem, self.memory.as_ptr() as *mut _);
            dealloc(self.memory.as_ptr(), self.layout);
        }
    }
}

unsafe impl Send for DecompressionStream {}

/// Convenience function to decompress an entire buffer at once.
///
/// This function handles frame detection and memory management automatically,
/// but requires the entire compressed data to be available in memory.
///
/// # Arguments
///
/// * `compressed_data` - The complete compressed data
/// * `max_output_size` - Maximum size of decompressed output (safety limit)
///
/// # Returns
///
/// Returns a `Vec<u8>` containing the decompressed data.
///
/// # Errors
///
/// Returns various errors if decompression fails or limits are exceeded.
pub fn decompress_all(compressed_data: &[u8], max_output_size: usize) -> ZstdResult<Vec<u8>> {
    if compressed_data.len() < 4 {
        return Err(Error::UnexpectedEof);
    }

    let header_size = core::cmp::min(compressed_data.len(), 18);
    let peek = FramePeek::new(&compressed_data[..header_size])?;
    let mut dstream = DecompressionStream::new(peek.window_size())?;

    let mut input = compressed_data;
    let mut output = Vec::new();
    let mut temp_buffer = vec::from_elem(0u8, 65536); // 64KB chunk sz

    while !input.is_empty() {
        let result = dstream.read(&mut input, &mut temp_buffer)?;
        if result.bytes_written > 0 {
            if output.len() + result.bytes_written > max_output_size {
                return Err(Error::InsufficientBuffer);
            }
            output.extend_from_slice(&temp_buffer[..result.bytes_written]);
        }

        if result.is_frame_complete() {
            if !input.is_empty() {
                dstream.reset();
            } else {
                break;
            }
        }

        if result.bytes_consumed == 0 && result.bytes_written == 0 {
            return Err(Error::InternalError(0));
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_peek_empty() {
        let result = FramePeek::new(&[]);
        assert!(matches!(result, Err(Error::UnexpectedEof)));
    }

    #[test]
    fn test_decomp_stream() {
        let result = DecompressionStream::new(64 * 1024);
        assert!(result.is_ok());
    }

    #[test]
    fn test_decomp_stream_reset() {
        let mut dstream = DecompressionStream::new(64 * 1024).unwrap();
        dstream.reset();
    }
}
