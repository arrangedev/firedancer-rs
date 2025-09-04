//! Safe Rust API for Firedancer nanopb implementation
//!
//! This crate provides safe abstractions over the raw FFI bindings in `fd_nanopb_sys`.
//! nanopb is a lightweight Protocol Buffers implementation designed for embedded systems
//! and resource-constrained environments.
//!
//! ## Features
//!
//! - **Memory safe**: All unsafe operations are encapsulated with proper error handling
//! - **Zero-copy**: Efficient encoding/decoding with minimal allocations
//! - **Buffer-only mode**: Simplified API that works with byte buffers
//! - **Field iteration**: Safe iteration over protocol buffer message fields
//! - **Error handling**: Comprehensive error types with descriptive messages
//! - **No std support**: Can be used in no_std environments
//!
//! ## Usage
//!
//! ### Encoding
//!
//! ```rust,no_run
//! use fd_nanopb::{OutputStream, Error};
//!
//! let mut buffer = [0u8; 1024];
//! let mut stream = OutputStream::from_buffer(&mut buffer)?;
//!
//! // Encode data using pb_encode_* functions
//! stream.write_varint(42)?;
//! stream.write_string(b"hello")?;
//!
//! let encoded_size = stream.bytes_written();
//! println!("Encoded {} bytes", encoded_size);
//! ```
//!
//! ### Decoding
//!
//! ```rust,no_run
//! use fd_nanopb::{InputStream, Error};
//!
//! let data = &[/* encoded protobuf data */];
//! let mut stream = InputStream::from_buffer(data)?;
//!
//! // Decode data using pb_decode_* functions
//! let value = stream.read_varint()?;
//! let string_data = stream.read_string()?;
//! ```
//!
//! ### Field Iteration
//!
//! ```rust,no_run
//! use fd_nanopb::{FieldIter, MessageDescriptor};
//!
//! // Iterate over fields in a message descriptor
//! let mut iter = FieldIter::begin(&descriptor, &message)?;
//! while let Some(field) = iter.next() {
//!     println!("Field tag: {}, type: {:?}", field.tag(), field.field_type());
//! }
//! ```

#![no_std]

extern crate alloc;

use alloc::{format, string::String};
use core::{fmt, mem::MaybeUninit};
use fd_nanopb_sys as sys;

pub const MAX_MESSAGE_SIZE: usize = 64 * 1024;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Buffer is too small for the operation
    BufferTooSmall,
    /// Invalid data format or structure
    InvalidData(String),
    /// End of stream reached unexpectedly
    UnexpectedEof,
    /// Invalid field type or wire type
    InvalidField(String),
    /// Invalid string encoding (not UTF-8)
    InvalidString,
    /// Invalid varint encoding
    InvalidVarint,
    /// Message is too large
    MessageTooLarge,
    /// Internal nanopb error
    InternalError(String),
    /// Null pointer or invalid memory access
    NullPointer,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BufferTooSmall => write!(f, "Buffer too small"),
            Error::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            Error::UnexpectedEof => write!(f, "Unexpected end of stream"),
            Error::InvalidField(msg) => write!(f, "Invalid field: {}", msg),
            Error::InvalidString => write!(f, "Invalid string encoding"),
            Error::InvalidVarint => write!(f, "Invalid varint encoding"),
            Error::MessageTooLarge => write!(f, "Message too large"),
            Error::InternalError(msg) => write!(f, "Internal error: {}", msg),
            Error::NullPointer => write!(f, "Null pointer"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireType {
    /// Variable-length integer
    Varint = sys::pb_wire_type_t_PB_WT_VARINT as isize,
    /// 64-bit fixed-length value
    Fixed64 = sys::pb_wire_type_t_PB_WT_64BIT as isize,
    /// Length-delimited value (strings, bytes, messages)
    LengthDelimited = sys::pb_wire_type_t_PB_WT_STRING as isize,
    /// 32-bit fixed-length value
    Fixed32 = sys::pb_wire_type_t_PB_WT_32BIT as isize,
}

impl WireType {
    /// Convert from raw wire type value
    pub fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            sys::pb_wire_type_t_PB_WT_VARINT => Some(WireType::Varint),
            sys::pb_wire_type_t_PB_WT_64BIT => Some(WireType::Fixed64),
            sys::pb_wire_type_t_PB_WT_STRING => Some(WireType::LengthDelimited),
            sys::pb_wire_type_t_PB_WT_32BIT => Some(WireType::Fixed32),
            _ => None,
        }
    }
}

/// Output stream for encoding Protocol Buffer data
pub struct OutputStream<'a> {
    stream: sys::pb_ostream_t,
    _buffer: &'a mut [u8],
}

impl<'a> OutputStream<'a> {
    /// Create an output stream from a mutable buffer
    pub fn from_buffer(buffer: &'a mut [u8]) -> Result<Self> {
        if buffer.is_empty() {
            return Err(Error::BufferTooSmall);
        }

        let stream = unsafe { sys::pb_ostream_from_buffer(buffer.as_mut_ptr(), buffer.len()) };

        Ok(Self {
            stream,
            _buffer: buffer,
        })
    }

    /// Get the number of bytes written to the stream
    pub fn bytes_written(&self) -> usize {
        self.stream.bytes_written
    }

    /// Get the maximum size of the stream
    pub fn max_size(&self) -> usize {
        self.stream.max_size
    }

    /// Get the remaining space in the stream
    pub fn bytes_remaining(&self) -> usize {
        self.stream
            .max_size
            .saturating_sub(self.stream.bytes_written)
    }

    /// Write raw bytes to the stream
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        let success = unsafe { sys::pb_write(&mut self.stream, data.as_ptr(), data.len()) };

        if success {
            Ok(())
        } else {
            Err(self.get_error())
        }
    }

    /// Write a varint to the stream
    pub fn write_varint(&mut self, value: u64) -> Result<()> {
        let success = unsafe { sys::pb_encode_varint(&mut self.stream, value) };

        if success {
            Ok(())
        } else {
            Err(self.get_error())
        }
    }

    /// Write a signed varint to the stream
    pub fn write_svarint(&mut self, value: i64) -> Result<()> {
        let success = unsafe { sys::pb_encode_svarint(&mut self.stream, value) };

        if success {
            Ok(())
        } else {
            Err(self.get_error())
        }
    }

    /// Write a string to the stream
    pub fn write_string(&mut self, data: &[u8]) -> Result<()> {
        let success = unsafe { sys::pb_encode_string(&mut self.stream, data.as_ptr(), data.len()) };

        if success {
            Ok(())
        } else {
            Err(self.get_error())
        }
    }

    /// Write a fixed32 value to the stream
    pub fn write_fixed32(&mut self, value: &[u8; 4]) -> Result<()> {
        let success = unsafe {
            sys::pb_encode_fixed32(&mut self.stream, value.as_ptr() as *const core::ffi::c_void)
        };

        if success {
            Ok(())
        } else {
            Err(self.get_error())
        }
    }

    /// Write a fixed64 value to the stream
    pub fn write_fixed64(&mut self, value: &[u8; 8]) -> Result<()> {
        let success = unsafe {
            sys::pb_encode_fixed64(&mut self.stream, value.as_ptr() as *const core::ffi::c_void)
        };

        if success {
            Ok(())
        } else {
            Err(self.get_error())
        }
    }

    /// Write a field tag to the stream
    pub fn write_tag(&mut self, wire_type: WireType, field_number: u32) -> Result<()> {
        let success =
            unsafe { sys::pb_encode_tag(&mut self.stream, wire_type as u32, field_number) };

        if success {
            Ok(())
        } else {
            Err(self.get_error())
        }
    }

    /// Get the current error from the stream
    fn get_error(&self) -> Error {
        let errmsg = unsafe {
            if self.stream.errmsg.is_null() {
                "Unknown error"
            } else {
                let cstr = core::ffi::CStr::from_ptr(self.stream.errmsg);
                cstr.to_str().unwrap_or("Invalid error message")
            }
        };
        Error::InternalError(String::from(errmsg))
    }

    /// Get the encoded data written to the stream
    pub fn encoded_data(&self) -> &[u8] {
        &self._buffer[..self.bytes_written()]
    }
}

/// Input stream for decoding Protocol Buffer data
pub struct InputStream<'a> {
    stream: sys::pb_istream_t,
    _data: &'a [u8],
}

impl<'a> InputStream<'a> {
    /// Create an input stream from a buffer
    pub fn from_buffer(data: &'a [u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::BufferTooSmall);
        }

        let stream = unsafe { sys::pb_istream_from_buffer(data.as_ptr(), data.len()) };

        Ok(Self {
            stream,
            _data: data,
        })
    }

    /// Get the number of bytes remaining in the stream
    pub fn bytes_remaining(&self) -> usize {
        self.stream.bytes_left
    }

    /// Check if the stream is at the end
    pub fn is_eof(&self) -> bool {
        self.stream.bytes_left == 0
    }

    /// Read raw bytes from the stream
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<()> {
        let success = unsafe { sys::pb_read(&mut self.stream, buffer.as_mut_ptr(), buffer.len()) };

        if success {
            Ok(())
        } else {
            Err(self.get_error())
        }
    }

    /// Read a varint from the stream
    pub fn read_varint(&mut self) -> Result<u64> {
        let mut value = 0u64;
        let success = unsafe { sys::pb_decode_varint(&mut self.stream, &mut value) };

        if success {
            Ok(value)
        } else {
            Err(self.get_error())
        }
    }

    /// Read a 32-bit varint from the stream
    pub fn read_varint32(&mut self) -> Result<u32> {
        let mut value = 0u32;
        let success = unsafe { sys::pb_decode_varint32(&mut self.stream, &mut value) };

        if success {
            Ok(value)
        } else {
            Err(self.get_error())
        }
    }

    /// Read a signed varint from the stream
    pub fn read_svarint(&mut self) -> Result<i64> {
        let mut value = 0i64;
        let success = unsafe { sys::pb_decode_svarint(&mut self.stream, &mut value) };

        if success {
            Ok(value)
        } else {
            Err(self.get_error())
        }
    }

    /// Read a boolean from the stream
    pub fn read_bool(&mut self) -> Result<bool> {
        let mut value = false;
        let success = unsafe { sys::pb_decode_bool(&mut self.stream, &mut value) };

        if success {
            Ok(value)
        } else {
            Err(self.get_error())
        }
    }

    /// Read a fixed32 value from the stream
    pub fn read_fixed32(&mut self) -> Result<[u8; 4]> {
        let mut value = [0u8; 4];
        let success = unsafe {
            sys::pb_decode_fixed32(
                &mut self.stream,
                value.as_mut_ptr() as *mut core::ffi::c_void,
            )
        };

        if success {
            Ok(value)
        } else {
            Err(self.get_error())
        }
    }

    /// Read a fixed64 value from the stream
    pub fn read_fixed64(&mut self) -> Result<[u8; 8]> {
        let mut value = [0u8; 8];
        let success = unsafe {
            sys::pb_decode_fixed64(
                &mut self.stream,
                value.as_mut_ptr() as *mut core::ffi::c_void,
            )
        };

        if success {
            Ok(value)
        } else {
            Err(self.get_error())
        }
    }

    /// Decode a field tag from the stream
    pub fn read_tag(&mut self) -> Result<Option<(WireType, u32)>> {
        let mut wire_type = 0u32;
        let mut tag = 0u32;
        let mut eof = false;

        let success =
            unsafe { sys::pb_decode_tag(&mut self.stream, &mut wire_type, &mut tag, &mut eof) };

        if success {
            if eof {
                Ok(None)
            } else {
                let wt = WireType::from_raw(wire_type).ok_or_else(|| {
                    Error::InvalidField(format!("Invalid wire type: {}", wire_type))
                })?;
                Ok(Some((wt, tag)))
            }
        } else {
            Err(self.get_error())
        }
    }

    /// Skip a field based on its wire type
    pub fn skip_field(&mut self, wire_type: WireType) -> Result<()> {
        let success = unsafe { sys::pb_skip_field(&mut self.stream, wire_type as u32) };

        if success {
            Ok(())
        } else {
            Err(self.get_error())
        }
    }

    /// Create a substream for reading a length-delimited field
    pub fn make_string_substream(&mut self) -> Result<InputStream<'_>> {
        let mut substream = MaybeUninit::<sys::pb_istream_t>::uninit();

        let success =
            unsafe { sys::pb_make_string_substream(&mut self.stream, substream.as_mut_ptr()) };

        if success {
            let substream = unsafe { substream.assume_init() };
            Ok(InputStream {
                stream: substream,
                _data: &[], // Substream manages its own data reference
            })
        } else {
            Err(self.get_error())
        }
    }

    /// Close a string substream
    pub fn close_string_substream(&mut self, substream: InputStream<'_>) -> Result<()> {
        let success = unsafe {
            sys::pb_close_string_substream(
                &mut self.stream,
                &substream.stream as *const _ as *mut _,
            )
        };

        if success {
            Ok(())
        } else {
            Err(self.get_error())
        }
    }

    /// Get the current error from the stream
    fn get_error(&self) -> Error {
        let errmsg = unsafe {
            if self.stream.errmsg.is_null() {
                "Unknown error"
            } else {
                let cstr = core::ffi::CStr::from_ptr(self.stream.errmsg);
                cstr.to_str().unwrap_or("Invalid error message")
            }
        };
        Error::InternalError(String::from(errmsg))
    }
}

pub struct MessageDescriptor {
    descriptor: *const sys::pb_msgdesc_t,
}

impl MessageDescriptor {
    /// Create a message descriptor from a raw pointer
    ///
    /// # Safety
    /// The caller must ensure the descriptor pointer is valid and lives
    /// as long as this MessageDescriptor instance.
    pub unsafe fn from_raw(descriptor: *const sys::pb_msgdesc_t) -> Result<Self> {
        if descriptor.is_null() {
            return Err(Error::NullPointer);
        }
        Ok(Self { descriptor })
    }

    /// Get the number of fields in the message
    pub fn field_count(&self) -> usize {
        unsafe { (*self.descriptor).field_count as usize }
    }

    /// Get the number of required fields in the message
    pub fn required_field_count(&self) -> usize {
        unsafe { (*self.descriptor).required_field_count as usize }
    }

    /// Get the largest tag number in the message
    pub fn largest_tag(&self) -> u32 {
        unsafe { (*self.descriptor).largest_tag }
    }

    /// Get the raw descriptor pointer
    ///
    /// # Safety
    /// The caller must ensure the returned pointer is not used after
    /// this MessageDescriptor is dropped.
    pub unsafe fn as_raw(&self) -> *const sys::pb_msgdesc_t {
        self.descriptor
    }
}

unsafe impl Send for MessageDescriptor {}
unsafe impl Sync for MessageDescriptor {}

pub struct FieldIter {
    iter: sys::pb_field_iter_t,
    _phantom: core::marker::PhantomData<*const ()>,
}

impl FieldIter {
    /// Begin iteration over fields in a message
    ///
    /// # Safety
    /// The caller must ensure the descriptor and message pointers are valid
    /// and live as long as this iterator.
    pub unsafe fn begin(
        descriptor: &MessageDescriptor,
        message: *mut core::ffi::c_void,
    ) -> Result<Self> {
        let mut iter = MaybeUninit::<sys::pb_field_iter_t>::uninit();

        let success = sys::pb_field_iter_begin(iter.as_mut_ptr(), descriptor.descriptor, message);

        if success {
            Ok(Self {
                iter: iter.assume_init(),
                _phantom: core::marker::PhantomData,
            })
        } else {
            Err(Error::InvalidData(
                "Failed to initialize field iterator".into(),
            ))
        }
    }

    /// Move to the next field
    pub fn next(&mut self) -> bool {
        unsafe { sys::pb_field_iter_next(&mut self.iter) }
    }

    /// Find a field with the given tag
    pub fn find(&mut self, tag: u32) -> bool {
        unsafe { sys::pb_field_iter_find(&mut self.iter, tag) }
    }

    /// Get the current field's tag
    pub fn tag(&self) -> u32 {
        self.iter.tag as u32
    }

    /// Get the current field's data size
    pub fn data_size(&self) -> usize {
        self.iter.data_size as usize
    }

    /// Get the current field's array size
    pub fn array_size(&self) -> usize {
        self.iter.array_size as usize
    }

    /// Get the current field's type
    pub fn field_type(&self) -> u8 {
        self.iter.type_
    }
}

pub mod utils {
    use super::*;

    /// Calculate the size needed to encode a message
    ///
    /// # Safety
    /// The caller must ensure the descriptor and message pointers are valid.
    pub unsafe fn get_encoded_size(
        descriptor: &MessageDescriptor,
        message: *const core::ffi::c_void,
    ) -> Result<usize> {
        let mut size = 0usize;
        let success = sys::pb_get_encoded_size(&mut size, descriptor.descriptor, message);

        if success {
            Ok(size)
        } else {
            Err(Error::InternalError(
                "Failed to calculate encoded size".into(),
            ))
        }
    }

    /// Encode a complete message to a buffer
    ///
    /// # Safety
    /// The caller must ensure the descriptor and message pointers are valid.
    pub unsafe fn encode_message(
        buffer: &mut [u8],
        descriptor: &MessageDescriptor,
        message: *const core::ffi::c_void,
    ) -> Result<usize> {
        let mut stream = OutputStream::from_buffer(buffer)?;
        let success = sys::pb_encode(&mut stream.stream, descriptor.descriptor, message);

        if success {
            Ok(stream.bytes_written())
        } else {
            Err(stream.get_error())
        }
    }

    /// Decode a complete message from a buffer
    ///
    /// # Safety
    /// The caller must ensure the descriptor and message pointers are valid.
    pub unsafe fn decode_message(
        data: &[u8],
        descriptor: &MessageDescriptor,
        message: *mut core::ffi::c_void,
    ) -> Result<()> {
        let mut stream = InputStream::from_buffer(data)?;
        let success = sys::pb_decode(&mut stream.stream, descriptor.descriptor, message);

        if success {
            Ok(())
        } else {
            Err(stream.get_error())
        }
    }

    /// Release any allocated memory in a decoded message
    ///
    /// # Safety
    /// The caller must ensure the descriptor and message pointers are valid.
    pub unsafe fn release_message(descriptor: &MessageDescriptor, message: *mut core::ffi::c_void) {
        sys::pb_release(descriptor.descriptor, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_output_stream() {
        let mut buffer = [0u8; 64];
        let stream = OutputStream::from_buffer(&mut buffer).unwrap();

        assert_eq!(stream.bytes_written(), 0);
        assert_eq!(stream.max_size(), 64);
        assert_eq!(stream.bytes_remaining(), 64);
    }

    #[test]
    fn test_input_stream() {
        let data = [0u8; 64];
        let stream = InputStream::from_buffer(&data).unwrap();

        assert_eq!(stream.bytes_remaining(), 64);
        assert!(!stream.is_eof());
    }

    #[test]
    fn test_empty_buffer() {
        let mut empty_buffer = [];
        assert!(OutputStream::from_buffer(&mut empty_buffer).is_err());

        let empty_data = [];
        assert!(InputStream::from_buffer(&empty_data).is_err());
    }

    #[test]
    fn test_wire_type() {
        assert_eq!(
            WireType::from_raw(sys::pb_wire_type_t_PB_WT_VARINT),
            Some(WireType::Varint)
        );
        assert_eq!(
            WireType::from_raw(sys::pb_wire_type_t_PB_WT_64BIT),
            Some(WireType::Fixed64)
        );
        assert_eq!(
            WireType::from_raw(sys::pb_wire_type_t_PB_WT_STRING),
            Some(WireType::LengthDelimited)
        );
        assert_eq!(
            WireType::from_raw(sys::pb_wire_type_t_PB_WT_32BIT),
            Some(WireType::Fixed32)
        );
        assert_eq!(WireType::from_raw(999), None);
    }

    #[test]
    fn test_write_raw() {
        let mut buffer = [0u8; 64];
        let mut stream = OutputStream::from_buffer(&mut buffer).unwrap();

        let data = b"hello world";
        stream.write(data).unwrap();

        assert_eq!(stream.bytes_written(), data.len());
        assert_eq!(stream.encoded_data(), data);
    }

    #[test]
    fn test_varint() {
        let mut buffer = [0u8; 64];
        let mut out_stream = OutputStream::from_buffer(&mut buffer).unwrap();

        let test_values = [0u64, 1, 127, 128, 16383, 16384, 2097151, 2097152];

        for &value in &test_values {
            out_stream.write_varint(value).unwrap();
        }

        let encoded_data = out_stream.encoded_data().to_vec();
        let mut in_stream = InputStream::from_buffer(&encoded_data).unwrap();

        for &expected in &test_values {
            let decoded = in_stream.read_varint().unwrap();
            assert_eq!(decoded, expected);
        }
    }

    #[test]
    fn test_string() {
        let mut buffer = [0u8; 64];
        let mut stream = OutputStream::from_buffer(&mut buffer).unwrap();

        let test_string = b"test string";
        stream.write_string(test_string).unwrap();
        assert!(stream.bytes_written() > test_string.len());
    }

    #[test]
    fn test_fixed() {
        let mut buffer = [0u8; 64];
        let mut out_stream = OutputStream::from_buffer(&mut buffer).unwrap();

        let fixed32_val = [0x12, 0x34, 0x56, 0x78];
        let fixed64_val = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];

        out_stream.write_fixed32(&fixed32_val).unwrap();
        out_stream.write_fixed64(&fixed64_val).unwrap();

        let encoded_data = out_stream.encoded_data().to_vec();
        let mut in_stream = InputStream::from_buffer(&encoded_data).unwrap();

        let decoded_32 = in_stream.read_fixed32().unwrap();
        let decoded_64 = in_stream.read_fixed64().unwrap();

        assert_eq!(decoded_32, fixed32_val);
        assert_eq!(decoded_64, fixed64_val);
    }
}
