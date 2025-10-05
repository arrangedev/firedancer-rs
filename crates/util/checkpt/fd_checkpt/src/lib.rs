//! Wrapper for `fd_checkpt_sys`
//!
//! This provided a safe API for the checkpoint system, which enables fast parallel compressed
//! checkpoint and restore operations. Both raw and lz4-compressed frames are supported.

use core::ffi::CStr;
use std::{fs::File, os::fd::IntoRawFd};

pub type CheckptResult<T> = Result<T, CheckptError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckptError {
    InvalidArgs(&'static str),
    Unsupported(&'static str),
    IoError(&'static str),
    CompressionError(&'static str),
    Unknown(&'static str),
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
            let c_str = fd_checkpt_sys::fd_checkpt_strerror(err_code);
            CStr::from_ptr(c_str).to_str().unwrap()
        };

        match err_code {
            fd_checkpt_sys::FD_CHECKPT_ERR_INVAL => CheckptError::InvalidArgs(msg),
            fd_checkpt_sys::FD_CHECKPT_ERR_UNSUP => CheckptError::Unsupported(msg),
            fd_checkpt_sys::FD_CHECKPT_ERR_IO => CheckptError::IoError(msg),
            fd_checkpt_sys::FD_CHECKPT_ERR_COMP => CheckptError::CompressionError(msg),
            _ => CheckptError::Unknown(msg),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameStyle {
    Raw = fd_checkpt_sys::FD_CHECKPT_FRAME_STYLE_RAW as isize,
    Lz4 = fd_checkpt_sys::FD_CHECKPT_FRAME_STYLE_LZ4 as isize,
}

impl FrameStyle {
    pub fn is_supported(self) -> bool {
        unsafe { fd_checkpt_sys::fd_checkpt_frame_style_is_supported(self as i32) != 0 }
    }
}

impl Default for FrameStyle {
    fn default() -> Self {
        FrameStyle::Raw
    }
}

#[repr(C)]
pub struct Checkpt {
    inner: Box<fd_checkpt_sys::fd_checkpt_private>,
    _write_buffer: Option<Vec<u8>>,
    _mmio_buffer: Option<Vec<u8>>,
}

impl Checkpt {
    pub fn new_stream(file: File, write_buffer_size: Option<usize>) -> CheckptResult<Self> {
        let wbuf_sz = write_buffer_size.unwrap_or(fd_checkpt_sys::FD_CHECKPT_WBUF_MIN as usize);
        if wbuf_sz < fd_checkpt_sys::FD_CHECKPT_WBUF_MIN as usize {
            return Err(CheckptError::InvalidArgs(
                "Write buffer size is less than minimum",
            ));
        }

        let mut write_buffer = vec![0u8; wbuf_sz];
        let mut checkpt_mem =
            Box::new(unsafe { core::mem::zeroed::<fd_checkpt_sys::fd_checkpt_private>() });

        let fd = file.into_raw_fd();
        let checkpt_ptr = unsafe {
            fd_checkpt_sys::fd_checkpt_init_stream(
                checkpt_mem.as_mut() as *mut _ as *mut core::ffi::c_void,
                fd,
                write_buffer.as_mut_ptr() as *mut core::ffi::c_void,
                wbuf_sz as u64,
            )
        };

        if checkpt_ptr.is_null() {
            unsafe { libc::close(fd) };
            return Err(CheckptError::InvalidArgs("Failed to initialize checkpoint"));
        }

        Ok(Checkpt {
            inner: checkpt_mem,
            _write_buffer: Some(write_buffer),
            _mmio_buffer: None,
        })
    }

    #[inline]
    pub fn new_mmio(buffer_size: usize) -> CheckptResult<Self> {
        let mut mmio_buffer = vec![0u8; buffer_size];
        let mut checkpt_mem =
            Box::new(unsafe { core::mem::zeroed::<fd_checkpt_sys::fd_checkpt_private>() });

        let checkpt_ptr = unsafe {
            fd_checkpt_sys::fd_checkpt_init_mmio(
                checkpt_mem.as_mut() as *mut _ as *mut core::ffi::c_void,
                mmio_buffer.as_mut_ptr() as *mut core::ffi::c_void,
                buffer_size as u64,
            )
        };

        if checkpt_ptr.is_null() {
            return Err(CheckptError::InvalidArgs("Failed to initialize checkpoint"));
        }

        Ok(Checkpt {
            inner: checkpt_mem,
            _write_buffer: None,
            _mmio_buffer: Some(mmio_buffer),
        })
    }

    #[inline]
    pub fn is_mmio(&self) -> bool {
        // checkpt->fd<0
        self.inner.fd < 0
    }

    #[inline]
    pub fn can_open_frame(&self) -> bool {
        //!checkpt->frame_style
        self.inner.frame_style == 0
    }

    #[inline]
    pub fn in_frame(&self) -> bool {
        // checkpt->frame_style>0
        self.inner.frame_style > 0
    }

    #[inline]
    pub fn open_frame(&mut self, style: FrameStyle) -> CheckptResult<u64> {
        if !style.is_supported() {
            return Err(CheckptError::Unsupported("Frame style is not supported"));
        }

        let mut offset = 0u64;
        let result = unsafe {
            fd_checkpt_sys::fd_checkpt_open_advanced(self.inner.as_mut(), style as i32, &mut offset)
        };

        if result == 0 {
            Ok(offset)
        } else {
            Err(CheckptError::from(result))
        }
    }

    #[inline]
    pub fn close_frame(&mut self) -> CheckptResult<u64> {
        let mut offset = 0u64;
        let result =
            unsafe { fd_checkpt_sys::fd_checkpt_close_advanced(self.inner.as_mut(), &mut offset) };

        if result == 0 {
            Ok(offset)
        } else {
            Err(CheckptError::from(result))
        }
    }

    #[inline]
    pub fn checkpoint_meta(&mut self, data: &[u8]) -> CheckptResult<()> {
        if data.len() > fd_checkpt_sys::FD_CHECKPT_META_MAX as usize {
            return Err(CheckptError::InvalidArgs("Metadata size exceeds maximum"));
        }

        let result = unsafe {
            fd_checkpt_sys::fd_checkpt_meta(
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

    #[inline]
    pub fn checkpoint_data(&mut self, data: &[u8]) -> CheckptResult<()> {
        let result = unsafe {
            fd_checkpt_sys::fd_checkpt_data(
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

    #[inline]
    pub fn mmio_buffer(&self) -> Option<&[u8]> {
        self._mmio_buffer.as_ref().map(|buf| buf.as_slice())
    }
}

impl Drop for Checkpt {
    fn drop(&mut self) {
        unsafe {
            let result = fd_checkpt_sys::fd_checkpt_fini(self.inner.as_mut());
            if result.is_null() {
                // can't panic in drop
            }
        }
    }
}

#[repr(C)]
pub struct Restore {
    inner: Box<fd_checkpt_sys::fd_restore_private>,
    _read_buffer: Option<Vec<u8>>,
    _mmio_buffer: Option<Vec<u8>>,
}

impl Restore {
    #[inline]
    pub fn new_stream(file: File, read_buffer_size: Option<usize>) -> CheckptResult<Self> {
        let rbuf_sz = read_buffer_size.unwrap_or(fd_checkpt_sys::FD_RESTORE_RBUF_MIN as usize);
        if rbuf_sz < fd_checkpt_sys::FD_RESTORE_RBUF_MIN as usize {
            return Err(CheckptError::InvalidArgs(
                "Read buffer size is less than minimum",
            ));
        }

        let mut read_buffer = vec![0u8; rbuf_sz];
        let mut restore_mem =
            Box::new(unsafe { core::mem::zeroed::<fd_checkpt_sys::fd_restore_private>() });

        let fd = file.into_raw_fd();
        let restore_ptr = unsafe {
            fd_checkpt_sys::fd_restore_init_stream(
                restore_mem.as_mut() as *mut _ as *mut core::ffi::c_void,
                fd,
                read_buffer.as_mut_ptr() as *mut core::ffi::c_void,
                rbuf_sz as u64,
            )
        };

        if restore_ptr.is_null() {
            unsafe { libc::close(fd) };
            return Err(CheckptError::InvalidArgs("Failed to initialize restore"));
        }

        Ok(Restore {
            inner: restore_mem,
            _read_buffer: Some(read_buffer),
            _mmio_buffer: None,
        })
    }

    #[inline]
    pub fn new_mmio(data: Vec<u8>) -> CheckptResult<Self> {
        let mut restore_mem =
            Box::new(unsafe { core::mem::zeroed::<fd_checkpt_sys::fd_restore_private>() });

        let restore_ptr = unsafe {
            fd_checkpt_sys::fd_restore_init_mmio(
                restore_mem.as_mut() as *mut _ as *mut core::ffi::c_void,
                data.as_ptr() as *const core::ffi::c_void,
                data.len() as u64,
            )
        };

        if restore_ptr.is_null() {
            return Err(CheckptError::InvalidArgs("Failed to initialize restore"));
        }

        Ok(Restore {
            inner: restore_mem,
            _read_buffer: None,
            _mmio_buffer: Some(data),
        })
    }

    #[inline]
    pub fn is_mmio(&self) -> bool {
        // restore->fd<0
        self.inner.fd < 0
    }

    #[inline]
    pub fn can_open_frame(&self) -> bool {
        // !restore->frame_style
        self.inner.frame_style == 0
    }

    #[inline]
    pub fn in_frame(&self) -> bool {
        // restore->frame_style>0
        self.inner.frame_style > 0
    }

    #[inline]
    pub fn size(&self) -> u64 {
        // restore->sz
        self.inner.sz
    }

    #[inline]
    pub fn seek(&mut self, offset: u64) -> CheckptResult<()> {
        let result = unsafe { fd_checkpt_sys::fd_restore_seek(self.inner.as_mut(), offset) };

        if result == 0 {
            Ok(())
        } else {
            Err(CheckptError::from(result))
        }
    }

    #[inline]
    pub fn open_frame(&mut self, style: FrameStyle) -> CheckptResult<u64> {
        let mut offset = 0u64;
        let result = unsafe {
            fd_checkpt_sys::fd_restore_open_advanced(self.inner.as_mut(), style as i32, &mut offset)
        };

        if result == 0 {
            Ok(offset)
        } else {
            Err(CheckptError::from(result))
        }
    }

    #[inline]
    pub fn close_frame(&mut self) -> CheckptResult<u64> {
        let mut offset = 0u64;
        let result =
            unsafe { fd_checkpt_sys::fd_restore_close_advanced(self.inner.as_mut(), &mut offset) };

        if result == 0 {
            Ok(offset)
        } else {
            Err(CheckptError::from(result))
        }
    }

    #[inline]
    pub fn restore_meta(&mut self, buffer: &mut [u8]) -> CheckptResult<()> {
        if buffer.len() > fd_checkpt_sys::FD_RESTORE_META_MAX as usize {
            return Err(CheckptError::InvalidArgs(
                "Metadata buffer size exceeds maximum",
            ));
        }

        let result = unsafe {
            fd_checkpt_sys::fd_restore_meta(
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

    #[inline]
    pub fn restore_data(&mut self, buffer: &mut [u8]) -> CheckptResult<()> {
        let result = unsafe {
            fd_checkpt_sys::fd_restore_data(
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

impl Drop for Restore {
    fn drop(&mut self) {
        unsafe {
            let result = fd_checkpt_sys::fd_restore_fini(self.inner.as_mut());
            if result.is_null() {
                // can't panic in drop
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
        let err = CheckptError::from(fd_checkpt_sys::FD_CHECKPT_ERR_INVAL);
        assert!(matches!(err, CheckptError::InvalidArgs(_)));

        let err = CheckptError::from(fd_checkpt_sys::FD_CHECKPT_ERR_UNSUP);
        assert!(matches!(err, CheckptError::Unsupported(_)));

        let err = CheckptError::from(fd_checkpt_sys::FD_CHECKPT_ERR_IO);
        assert!(matches!(err, CheckptError::IoError(_)));

        let err = CheckptError::from(fd_checkpt_sys::FD_CHECKPT_ERR_COMP);
        assert!(matches!(err, CheckptError::CompressionError(_)));
    }

    #[test]
    fn test_mmio_chkpt() {
        let mut checkpt = Checkpt::new_mmio(1024 * 1024).expect("Failed to create MMIO checkpoint");

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
        let mut checkpt = Checkpt::new_mmio(1024 * 1024).expect("Failed to create MMIO checkpoint");

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
            Restore::new_mmio(checkpoint_data).expect("Failed to create MMIO restore");

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

        let mut checkpt = Checkpt::new_mmio(1024 * 1024).expect("Failed to create MMIO checkpoint");

        checkpt.open_frame(style).expect("Failed to open frame");

        let test_data = b"Frame style test data";
        checkpt
            .checkpoint_meta(test_data)
            .expect("Failed to checkpoint metadata");

        let end_offset = checkpt.close_frame().expect("Failed to close frame");

        let checkpoint_data = checkpt.mmio_buffer().expect("No MMIO buffer").to_vec();
        let checkpoint_data = checkpoint_data[..end_offset as usize].to_vec();

        let mut restore =
            Restore::new_mmio(checkpoint_data).expect("Failed to create MMIO restore");
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
            Checkpt::new_mmio(2 * 1024 * 1024).expect("Failed to create MMIO checkpoint");

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
            Restore::new_mmio(checkpoint_data).expect("Failed to create MMIO restore");
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
    fn test_stream_chkpt() {
        let mut checkpt = Checkpt::new_stream(
            File::create("test_stream_chkpt.bin").expect("Failed to create file"),
            Some(1024 * 1024),
        )
        .expect("Failed to create stream checkpoint");
        checkpt
            .open_frame(FrameStyle::Raw)
            .expect("Failed to open frame");

        let test_data = b"Test data for stream checkpoint";
        checkpt
            .checkpoint_meta(test_data)
            .expect("Failed to checkpoint metadata");

        let end_offset = checkpt.close_frame().expect("Failed to close frame");

        println!("Checkpoint data size: {}", end_offset);

        let mut restore = Restore::new_stream(
            File::open("test_stream_chkpt.bin").expect("Failed to open file"),
            Some(1024 * 1024),
        )
        .expect("Failed to create stream restore");

        restore
            .open_frame(FrameStyle::Raw)
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
    fn test_error_cases() {
        let mut checkpt = Checkpt::new_mmio(1024 * 1024).expect("Failed to create MMIO checkpoint");
        checkpt
            .open_frame(FrameStyle::Raw)
            .expect("Failed to open frame");

        let too_large_meta = vec![0u8; (fd_checkpt_sys::FD_CHECKPT_META_MAX + 1) as usize];
        let result = checkpt.checkpoint_meta(&too_large_meta);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CheckptError::InvalidArgs(_)));

        let checkpoint_data = vec![0u8; 1024];
        let mut restore =
            Restore::new_mmio(checkpoint_data).expect("Failed to create MMIO restore");

        let too_large_buffer = vec![0u8; (fd_checkpt_sys::FD_RESTORE_META_MAX + 1) as usize];
        let result = restore.restore_meta(&mut too_large_buffer.clone());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CheckptError::InvalidArgs(_)));
    }
}
