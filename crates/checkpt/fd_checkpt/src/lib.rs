//! Safe Rust bindings for Firedancer checkpoint utility
//!
//! This crate provides a safe, idiomatic Rust API for the Firedancer checkpoint and restore system.
//! It wraps the unsafe FFI bindings provided by `libfd_checkpt_sys`.
//!
//! The checkpoint system enables fast parallel compressed checkpoint and restore operations
//! with support for both raw and LZ4-compressed frames.

use core::ffi::CStr;
use std::{fs::File, os::fd::IntoRawFd};

/// Result type for checkpoint operations
pub type CheckptResult<T> = Result<T, CheckptError>;

/// Errors that can occur during checkpoint operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckptError {
    /// Invalid input arguments
    InvalidArgs(String),
    /// Unsupported operation on this target
    Unsupported(String),
    /// I/O error occurred
    IoError(String),
    /// Compression/decompression error
    CompressionError(String),
    /// Unknown error
    Unknown(String),
}

impl core::fmt::Display for CheckptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CheckptError::InvalidArgs(msg) => write!(f, "Invalid arguments: {}", msg),
            CheckptError::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            CheckptError::IoError(msg) => write!(f, "I/O error: {}", msg),
            CheckptError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            CheckptError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl core::error::Error for CheckptError {}

impl From<i32> for CheckptError {
    fn from(err_code: i32) -> Self {
        let msg = unsafe {
            let c_str = libfd_checkpt_sys::fd_checkpt_strerror(err_code);
            CStr::from_ptr(c_str).to_string_lossy().into_owned()
        };

        match err_code {
            libfd_checkpt_sys::FD_CHECKPT_ERR_INVAL => CheckptError::InvalidArgs(msg),
            libfd_checkpt_sys::FD_CHECKPT_ERR_UNSUP => CheckptError::Unsupported(msg),
            libfd_checkpt_sys::FD_CHECKPT_ERR_IO => CheckptError::IoError(msg),
            libfd_checkpt_sys::FD_CHECKPT_ERR_COMP => CheckptError::CompressionError(msg),
            _ => CheckptError::Unknown(msg),
        }
    }
}

/// Frame styles for checkpoint compression
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameStyle {
    /// Uncompressed frame
    Raw = libfd_checkpt_sys::FD_CHECKPT_FRAME_STYLE_RAW as isize,
    /// LZ4 compressed frame
    Lz4 = libfd_checkpt_sys::FD_CHECKPT_FRAME_STYLE_LZ4 as isize,
}

impl FrameStyle {
    /// Check if this frame style is supported on the current target
    pub fn is_supported(self) -> bool {
        unsafe { libfd_checkpt_sys::fd_checkpt_frame_style_is_supported(self as i32) != 0 }
    }
}

impl Default for FrameStyle {
    fn default() -> Self {
        FrameStyle::Raw
    }
}

/// A checkpoint handle for writing checkpoint data
pub struct FdCheckpt {
    inner: Box<libfd_checkpt_sys::fd_checkpt_private>,
    _write_buffer: Option<Vec<u8>>,
    _mmio_buffer: Option<Vec<u8>>,
}

impl FdCheckpt {
    pub fn new_stream(file: File, write_buffer_size: Option<usize>) -> CheckptResult<Self> {
        let wbuf_sz = write_buffer_size.unwrap_or(libfd_checkpt_sys::FD_CHECKPT_WBUF_MIN as usize);
        if wbuf_sz < libfd_checkpt_sys::FD_CHECKPT_WBUF_MIN as usize {
            return Err(CheckptError::InvalidArgs(format!(
                "Write buffer size {} is less than minimum {}",
                wbuf_sz,
                libfd_checkpt_sys::FD_CHECKPT_WBUF_MIN
            )));
        }

        let mut write_buffer = vec![0u8; wbuf_sz];
        let mut checkpt_mem =
            Box::new(unsafe { core::mem::zeroed::<libfd_checkpt_sys::fd_checkpt_private>() });

        let fd = file.into_raw_fd();
        let checkpt_ptr = unsafe {
            libfd_checkpt_sys::fd_checkpt_init_stream(
                checkpt_mem.as_mut() as *mut _ as *mut core::ffi::c_void,
                fd,
                write_buffer.as_mut_ptr() as *mut core::ffi::c_void,
                wbuf_sz as u64,
            )
        };

        if checkpt_ptr.is_null() {
            // If initialization failed, we need to close the file descriptor
            unsafe { libc::close(fd) };
            return Err(CheckptError::InvalidArgs(
                "Failed to initialize checkpoint".to_string(),
            ));
        }

        Ok(FdCheckpt {
            inner: checkpt_mem,
            _write_buffer: Some(write_buffer),
            _mmio_buffer: None,
        })
    }

    /// Create a new checkpoint in memory-mapped I/O mode
    pub fn new_mmio(buffer_size: usize) -> CheckptResult<Self> {
        let mut mmio_buffer = vec![0u8; buffer_size];
        let mut checkpt_mem =
            Box::new(unsafe { core::mem::zeroed::<libfd_checkpt_sys::fd_checkpt_private>() });

        let checkpt_ptr = unsafe {
            libfd_checkpt_sys::fd_checkpt_init_mmio(
                checkpt_mem.as_mut() as *mut _ as *mut core::ffi::c_void,
                mmio_buffer.as_mut_ptr() as *mut core::ffi::c_void,
                buffer_size as u64,
            )
        };

        if checkpt_ptr.is_null() {
            return Err(CheckptError::InvalidArgs(
                "Failed to initialize checkpoint".to_string(),
            ));
        }

        Ok(FdCheckpt {
            inner: checkpt_mem,
            _write_buffer: None,
            _mmio_buffer: Some(mmio_buffer),
        })
    }

    /// Check if this checkpoint is in memory-mapped I/O mode
    pub fn is_mmio(&self) -> bool {
        // Inline implementation: return checkpt->fd<0
        self.inner.fd < 0
    }

    /// Check if a frame can be opened
    pub fn can_open_frame(&self) -> bool {
        // Inline implementation: return !checkpt->frame_style
        self.inner.frame_style == 0
    }

    /// Check if currently in a frame
    pub fn in_frame(&self) -> bool {
        // Inline implementation: return checkpt->frame_style>0
        self.inner.frame_style > 0
    }

    /// Open a new frame with the specified style
    pub fn open_frame(&mut self, style: FrameStyle) -> CheckptResult<u64> {
        if !style.is_supported() {
            return Err(CheckptError::Unsupported(format!(
                "{:?} frame style is not supported",
                style
            )));
        }

        let mut offset = 0u64;
        let result = unsafe {
            libfd_checkpt_sys::fd_checkpt_open_advanced(
                self.inner.as_mut(),
                style as i32,
                &mut offset,
            )
        };

        if result == 0 {
            Ok(offset)
        } else {
            Err(CheckptError::from(result))
        }
    }

    /// Close the current frame
    pub fn close_frame(&mut self) -> CheckptResult<u64> {
        let mut offset = 0u64;
        let result = unsafe {
            libfd_checkpt_sys::fd_checkpt_close_advanced(self.inner.as_mut(), &mut offset)
        };

        if result == 0 {
            Ok(offset)
        } else {
            Err(CheckptError::from(result))
        }
    }

    /// Checkpoint metadata (small buffers, copied immediately)
    pub fn checkpoint_meta(&mut self, data: &[u8]) -> CheckptResult<()> {
        if data.len() > libfd_checkpt_sys::FD_CHECKPT_META_MAX as usize {
            return Err(CheckptError::InvalidArgs(format!(
                "Metadata size {} exceeds maximum {}",
                data.len(),
                libfd_checkpt_sys::FD_CHECKPT_META_MAX
            )));
        }

        let result = unsafe {
            libfd_checkpt_sys::fd_checkpt_meta(
                self.inner.as_mut(),
                data.as_ptr() as *const core::ffi::c_void,
                data.len() as u64,
            )
        };

        if result == 0 {
            Ok(())
        } else {
            Err(CheckptError::from(result))
        }
    }

    /// Checkpoint data (large buffers, must remain valid until frame is closed)
    pub fn checkpoint_data(&mut self, data: &[u8]) -> CheckptResult<()> {
        let result = unsafe {
            libfd_checkpt_sys::fd_checkpt_data(
                self.inner.as_mut(),
                data.as_ptr() as *const core::ffi::c_void,
                data.len() as u64,
            )
        };

        if result == 0 {
            Ok(())
        } else {
            Err(CheckptError::from(result))
        }
    }

    /// Get the memory-mapped buffer (if in MMIO mode)
    pub fn mmio_buffer(&self) -> Option<&[u8]> {
        self._mmio_buffer.as_ref().map(|buf| buf.as_slice())
    }
}

impl Drop for FdCheckpt {
    fn drop(&mut self) {
        unsafe {
            let result = libfd_checkpt_sys::fd_checkpt_fini(self.inner.as_mut());
            if result.is_null() {
                // Log warning but can't panic in drop
                eprintln!("Warning: Failed to properly finalize checkpoint");
            }
        }
    }
}

/// A restore handle for reading checkpoint data
pub struct FdRestore {
    inner: Box<libfd_checkpt_sys::fd_restore_private>,
    _read_buffer: Option<Vec<u8>>,
    _mmio_buffer: Option<Vec<u8>>,
}

impl FdRestore {
    /// Create a new restore in streaming mode
    pub fn new_stream(file: File, read_buffer_size: Option<usize>) -> CheckptResult<Self> {
        let rbuf_sz = read_buffer_size.unwrap_or(libfd_checkpt_sys::FD_RESTORE_RBUF_MIN as usize);
        if rbuf_sz < libfd_checkpt_sys::FD_RESTORE_RBUF_MIN as usize {
            return Err(CheckptError::InvalidArgs(format!(
                "Read buffer size {} is less than minimum {}",
                rbuf_sz,
                libfd_checkpt_sys::FD_RESTORE_RBUF_MIN
            )));
        }

        let mut read_buffer = vec![0u8; rbuf_sz];
        let mut restore_mem =
            Box::new(unsafe { core::mem::zeroed::<libfd_checkpt_sys::fd_restore_private>() });

        let fd = file.into_raw_fd();
        let restore_ptr = unsafe {
            libfd_checkpt_sys::fd_restore_init_stream(
                restore_mem.as_mut() as *mut _ as *mut core::ffi::c_void,
                fd,
                read_buffer.as_mut_ptr() as *mut core::ffi::c_void,
                rbuf_sz as u64,
            )
        };

        if restore_ptr.is_null() {
            // If initialization failed, we need to close the file descriptor
            unsafe { libc::close(fd) };
            return Err(CheckptError::InvalidArgs(
                "Failed to initialize restore".to_string(),
            ));
        }

        Ok(FdRestore {
            inner: restore_mem,
            _read_buffer: Some(read_buffer),
            _mmio_buffer: None,
        })
    }

    /// Create a new restore from memory-mapped data
    pub fn new_mmio(data: Vec<u8>) -> CheckptResult<Self> {
        let mut restore_mem =
            Box::new(unsafe { core::mem::zeroed::<libfd_checkpt_sys::fd_restore_private>() });

        let restore_ptr = unsafe {
            libfd_checkpt_sys::fd_restore_init_mmio(
                restore_mem.as_mut() as *mut _ as *mut core::ffi::c_void,
                data.as_ptr() as *const core::ffi::c_void,
                data.len() as u64,
            )
        };

        if restore_ptr.is_null() {
            return Err(CheckptError::InvalidArgs(
                "Failed to initialize restore".to_string(),
            ));
        }

        Ok(FdRestore {
            inner: restore_mem,
            _read_buffer: None,
            _mmio_buffer: Some(data),
        })
    }

    /// Check if this restore is in memory-mapped I/O mode
    pub fn is_mmio(&self) -> bool {
        // Inline implementation: return restore->fd<0
        self.inner.fd < 0
    }

    /// Check if a frame can be opened
    pub fn can_open_frame(&self) -> bool {
        // Inline implementation: return !restore->frame_style
        self.inner.frame_style == 0
    }

    /// Check if currently in a frame
    pub fn in_frame(&self) -> bool {
        // Inline implementation: return restore->frame_style>0
        self.inner.frame_style > 0
    }

    /// Get the size of the checkpoint data
    pub fn size(&self) -> u64 {
        // Inline implementation: return restore->sz
        self.inner.sz
    }

    /// Seek to a specific offset in the checkpoint
    pub fn seek(&mut self, offset: u64) -> CheckptResult<()> {
        let result = unsafe { libfd_checkpt_sys::fd_restore_seek(self.inner.as_mut(), offset) };

        if result == 0 {
            Ok(())
        } else {
            Err(CheckptError::from(result))
        }
    }

    /// Open a frame for restoration
    pub fn open_frame(&mut self, style: FrameStyle) -> CheckptResult<u64> {
        let mut offset = 0u64;
        let result = unsafe {
            libfd_checkpt_sys::fd_restore_open_advanced(
                self.inner.as_mut(),
                style as i32,
                &mut offset,
            )
        };

        if result == 0 {
            Ok(offset)
        } else {
            Err(CheckptError::from(result))
        }
    }

    /// Close the current frame
    pub fn close_frame(&mut self) -> CheckptResult<u64> {
        let mut offset = 0u64;
        let result = unsafe {
            libfd_checkpt_sys::fd_restore_close_advanced(self.inner.as_mut(), &mut offset)
        };

        if result == 0 {
            Ok(offset)
        } else {
            Err(CheckptError::from(result))
        }
    }

    /// Restore metadata (small buffers, copied immediately)
    pub fn restore_meta(&mut self, buffer: &mut [u8]) -> CheckptResult<()> {
        if buffer.len() > libfd_checkpt_sys::FD_RESTORE_META_MAX as usize {
            return Err(CheckptError::InvalidArgs(format!(
                "Metadata buffer size {} exceeds maximum {}",
                buffer.len(),
                libfd_checkpt_sys::FD_RESTORE_META_MAX
            )));
        }

        let result = unsafe {
            libfd_checkpt_sys::fd_restore_meta(
                self.inner.as_mut(),
                buffer.as_mut_ptr() as *mut core::ffi::c_void,
                buffer.len() as u64,
            )
        };

        if result == 0 {
            Ok(())
        } else {
            Err(CheckptError::from(result))
        }
    }

    /// Restore data (large buffers, must remain valid until frame is closed)
    pub fn restore_data(&mut self, buffer: &mut [u8]) -> CheckptResult<()> {
        let result = unsafe {
            libfd_checkpt_sys::fd_restore_data(
                self.inner.as_mut(),
                buffer.as_mut_ptr() as *mut core::ffi::c_void,
                buffer.len() as u64,
            )
        };

        if result == 0 {
            Ok(())
        } else {
            Err(CheckptError::from(result))
        }
    }
}

impl Drop for FdRestore {
    fn drop(&mut self) {
        unsafe {
            let result = libfd_checkpt_sys::fd_restore_fini(self.inner.as_mut());
            if result.is_null() {
                // Log warning but can't panic in drop
                eprintln!("Warning: Failed to properly finalize restore");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_case::test_case;

    #[test]
    fn test_framestyle_support() {
        assert!(FrameStyle::Raw.is_supported());
        let lz4_supported = FrameStyle::Lz4.is_supported();
        println!("LZ4 support: {}", lz4_supported);
    }

    #[test]
    fn test_error_conv() {
        let err = CheckptError::from(libfd_checkpt_sys::FD_CHECKPT_ERR_INVAL);
        assert!(matches!(err, CheckptError::InvalidArgs(_)));

        let err = CheckptError::from(libfd_checkpt_sys::FD_CHECKPT_ERR_UNSUP);
        assert!(matches!(err, CheckptError::Unsupported(_)));

        let err = CheckptError::from(libfd_checkpt_sys::FD_CHECKPT_ERR_IO);
        assert!(matches!(err, CheckptError::IoError(_)));

        let err = CheckptError::from(libfd_checkpt_sys::FD_CHECKPT_ERR_COMP);
        assert!(matches!(err, CheckptError::CompressionError(_)));
    }

    #[test]
    fn test_mmio_chkpt() {
        let mut checkpt =
            FdCheckpt::new_mmio(1024 * 1024).expect("Failed to create MMIO checkpoint");

        assert!(checkpt.is_mmio());
        assert!(checkpt.can_open_frame());
        assert!(!checkpt.in_frame());

        let offset = checkpt
            .open_frame(FrameStyle::Raw)
            .expect("Failed to open frame");

        assert_eq!(offset, 0);
        assert!(checkpt.in_frame());
        assert!(!checkpt.can_open_frame());

        let test_data = b"Hello, checkpoint!";
        checkpt
            .checkpoint_meta(test_data)
            .expect("Failed to checkpoint metadata");

        let end_offset = checkpt.close_frame().expect("Failed to close frame");
        assert!(end_offset > 0);
        assert!(!checkpt.in_frame());
        assert!(checkpt.can_open_frame());
    }

    #[test]
    fn test_mmio_roundtrip() {
        let mut checkpt =
            FdCheckpt::new_mmio(1024 * 1024).expect("Failed to create MMIO checkpoint");

        checkpt
            .open_frame(FrameStyle::Raw)
            .expect("Failed to open frame");

        let test_data = b"Test data for round trip";
        checkpt
            .checkpoint_meta(test_data)
            .expect("Failed to checkpoint metadata");

        let end_offset = checkpt.close_frame().expect("Failed to close frame");

        let checkpoint_data = checkpt.mmio_buffer().expect("No MMIO buffer").to_vec();
        let checkpoint_data = checkpoint_data[..end_offset as usize].to_vec();

        let mut restore =
            FdRestore::new_mmio(checkpoint_data).expect("Failed to create MMIO restore");

        assert!(restore.is_mmio());
        assert!(restore.can_open_frame());

        let restore_offset = restore
            .open_frame(FrameStyle::Raw)
            .expect("Failed to open restore frame");
        assert_eq!(restore_offset, 0);

        let mut restored_data = vec![0u8; test_data.len()];
        restore
            .restore_meta(&mut restored_data)
            .expect("Failed to restore metadata");

        assert_eq!(&restored_data, test_data);

        let restore_end_offset = restore
            .close_frame()
            .expect("Failed to close restore frame");
        assert_eq!(restore_end_offset, end_offset);
    }

    #[test_case(FrameStyle::Raw; "raw_frame")]
    #[test_case(FrameStyle::Lz4; "lz4_frame")]
    fn test_framestyles(style: FrameStyle) {
        if !style.is_supported() {
            println!("Skipping test for unsupported frame style: {:?}", style);
            return;
        }

        let mut checkpt =
            FdCheckpt::new_mmio(1024 * 1024).expect("Failed to create MMIO checkpoint");

        checkpt.open_frame(style).expect("Failed to open frame");

        let test_data = b"Frame style test data";
        checkpt
            .checkpoint_meta(test_data)
            .expect("Failed to checkpoint metadata");

        let end_offset = checkpt.close_frame().expect("Failed to close frame");

        let checkpoint_data = checkpt.mmio_buffer().expect("No MMIO buffer").to_vec();
        let checkpoint_data = checkpoint_data[..end_offset as usize].to_vec();

        let mut restore =
            FdRestore::new_mmio(checkpoint_data).expect("Failed to create MMIO restore");
        restore
            .open_frame(style)
            .expect("Failed to open restore frame");

        let mut restored_data = vec![0u8; test_data.len()];
        restore
            .restore_meta(&mut restored_data)
            .expect("Failed to restore metadata");

        assert_eq!(&restored_data, test_data);

        restore
            .close_frame()
            .expect("Failed to close restore frame");
    }

    #[test]
    fn test_large_chkpt() {
        let mut checkpt =
            FdCheckpt::new_mmio(2 * 1024 * 1024).expect("Failed to create MMIO checkpoint");

        checkpt
            .open_frame(FrameStyle::Raw)
            .expect("Failed to open frame");

        let large_data = vec![0x42u8; 100 * 1024]; // 100KB
        checkpt
            .checkpoint_data(&large_data)
            .expect("Failed to checkpoint data");

        let end_offset = checkpt.close_frame().expect("Failed to close frame");

        let checkpoint_data = checkpt.mmio_buffer().expect("No MMIO buffer").to_vec();
        let checkpoint_data = checkpoint_data[..end_offset as usize].to_vec();

        let mut restore =
            FdRestore::new_mmio(checkpoint_data).expect("Failed to create MMIO restore");
        restore
            .open_frame(FrameStyle::Raw)
            .expect("Failed to open restore frame");

        let mut restored_data = vec![0u8; large_data.len()];
        restore
            .restore_data(&mut restored_data)
            .expect("Failed to restore data");

        restore
            .close_frame()
            .expect("Failed to close restore frame");

        assert_eq!(restored_data, large_data);
    }

    #[test]
    fn test_error_cases() {
        let mut checkpt =
            FdCheckpt::new_mmio(1024 * 1024).expect("Failed to create MMIO checkpoint");
        checkpt
            .open_frame(FrameStyle::Raw)
            .expect("Failed to open frame");

        let too_large_meta = vec![0u8; (libfd_checkpt_sys::FD_CHECKPT_META_MAX + 1) as usize];
        let result = checkpt.checkpoint_meta(&too_large_meta);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CheckptError::InvalidArgs(_)));

        let checkpoint_data = vec![0u8; 1024];
        let mut restore =
            FdRestore::new_mmio(checkpoint_data).expect("Failed to create MMIO restore");

        let too_large_buffer = vec![0u8; (libfd_checkpt_sys::FD_RESTORE_META_MAX + 1) as usize];
        let result = restore.restore_meta(&mut too_large_buffer.clone());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CheckptError::InvalidArgs(_)));
    }
}
