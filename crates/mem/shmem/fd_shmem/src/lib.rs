//! Safe Rust bindings for Firedancer shared memory utility
//!
//! This crate provides a safe, idiomatic Rust API for the Firedancer shared memory management system.
//! It wraps the unsafe FFI bindings provided by `libfd_shmem_sys`.
//!
//! The shared memory system enables NUMA-aware and page size-aware manipulation of complex
//! interprocess shared memory topologies with support for different page sizes and NUMA nodes.

use core::ffi::CStr;
use core::ptr;
use std::ffi::CString;

/// Result type for shared memory operations
pub type ShmemResult<T> = Result<T, ShmemError>;

/// Errors that can occur during shared memory operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShmemError {
    /// Invalid input arguments
    InvalidArgs(String),
    /// Region not found
    NotFound(String),
    /// Permission denied
    PermissionDenied(String),
    /// Region already exists
    AlreadyExists(String),
    /// I/O error occurred
    IoError(String),
    /// Memory allocation failed
    MemoryError(String),
    /// System error with errno
    SystemError(String, i32),
    /// Unknown error
    Unknown(String),
}

impl core::fmt::Display for ShmemError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ShmemError::InvalidArgs(msg) => write!(f, "Invalid arguments: {}", msg),
            ShmemError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ShmemError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            ShmemError::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            ShmemError::IoError(msg) => write!(f, "I/O error: {}", msg),
            ShmemError::MemoryError(msg) => write!(f, "Memory error: {}", msg),
            ShmemError::SystemError(msg, errno) => {
                write!(f, "System error: {} (errno: {})", msg, errno)
            }
            ShmemError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl core::error::Error for ShmemError {}

impl From<i32> for ShmemError {
    fn from(errno: i32) -> Self {
        match errno {
            libc::EINVAL => ShmemError::InvalidArgs("Invalid arguments".to_string()),
            libc::ENOENT => ShmemError::NotFound("Region not found".to_string()),
            libc::EACCES => ShmemError::PermissionDenied("Permission denied".to_string()),
            libc::EEXIST => ShmemError::AlreadyExists("Region already exists".to_string()),
            libc::EIO => ShmemError::IoError("I/O error".to_string()),
            libc::ENOMEM => ShmemError::MemoryError("Out of memory".to_string()),
            _ => ShmemError::SystemError("System error".to_string(), errno),
        }
    }
}

/// Page sizes supported by the shared memory system
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageSize {
    /// Normal 4KB pages
    Normal = libfd_shmem_sys::FD_SHMEM_NORMAL_PAGE_SZ as isize,
    /// Huge 2MB pages
    Huge = libfd_shmem_sys::FD_SHMEM_HUGE_PAGE_SZ as isize,
    /// Gigantic 1GB pages
    Gigantic = libfd_shmem_sys::FD_SHMEM_GIGANTIC_PAGE_SZ as isize,
}

impl PageSize {
    /// Get the size in bytes
    pub fn size_bytes(self) -> usize {
        self as usize
    }

    /// Get the log2 of the page size
    pub fn log2_size(self) -> i32 {
        match self {
            PageSize::Normal => libfd_shmem_sys::FD_SHMEM_NORMAL_LG_PAGE_SZ as i32,
            PageSize::Huge => libfd_shmem_sys::FD_SHMEM_HUGE_LG_PAGE_SZ as i32,
            PageSize::Gigantic => libfd_shmem_sys::FD_SHMEM_GIGANTIC_LG_PAGE_SZ as i32,
        }
    }

    /// Convert from raw page size value
    pub fn from_raw(page_sz: u64) -> Option<Self> {
        match page_sz {
            x if x == libfd_shmem_sys::FD_SHMEM_NORMAL_PAGE_SZ as u64 => Some(PageSize::Normal),
            x if x == libfd_shmem_sys::FD_SHMEM_HUGE_PAGE_SZ as u64 => Some(PageSize::Huge),
            x if x == libfd_shmem_sys::FD_SHMEM_GIGANTIC_PAGE_SZ as u64 => Some(PageSize::Gigantic),
            _ => None,
        }
    }

    /// Get a string representation of the page size
    pub fn as_str(self) -> &'static str {
        unsafe {
            let c_str = libfd_shmem_sys::fd_shmem_page_sz_to_cstr(self as u64);
            CStr::from_ptr(c_str).to_str().unwrap_or("unknown")
        }
    }

    /// Parse page size from string
    pub fn from_str(s: &str) -> Option<Self> {
        let c_str = CString::new(s).ok()?;
        let page_sz = unsafe { libfd_shmem_sys::fd_cstr_to_shmem_page_sz(c_str.as_ptr()) };
        Self::from_raw(page_sz)
    }
}

impl Default for PageSize {
    fn default() -> Self {
        PageSize::Normal
    }
}

/// Join mode for shared memory regions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinMode {
    /// Read-only access
    ReadOnly = libfd_shmem_sys::FD_SHMEM_JOIN_MODE_READ_ONLY as isize,
    /// Read-write access
    ReadWrite = libfd_shmem_sys::FD_SHMEM_JOIN_MODE_READ_WRITE as isize,
}

impl Default for JoinMode {
    fn default() -> Self {
        JoinMode::ReadWrite
    }
}

/// Information about a shared memory region
#[derive(Debug, Clone)]
pub struct ShmemInfo {
    /// Page size used for the region
    pub page_size: PageSize,
    /// Number of pages in the region
    pub page_count: u64,
}

impl ShmemInfo {
    /// Get the total size of the region in bytes
    pub fn total_size(&self) -> u64 {
        self.page_size.size_bytes() as u64 * self.page_count
    }
}

/// Information about a joined shared memory region
#[derive(Debug, Clone)]
pub struct JoinInfo {
    /// Reference count for the join
    pub ref_count: i64,
    /// Local address of the shared memory region
    pub address: *mut u8,
    /// Page size used for the region
    pub page_size: PageSize,
    /// Number of pages in the region
    pub page_count: u64,
    /// Join mode (read-only or read-write)
    pub mode: JoinMode,
    /// Name of the region
    pub name: String,
}

impl JoinInfo {
    /// Get the total size of the region in bytes
    pub fn total_size(&self) -> u64 {
        self.page_size.size_bytes() as u64 * self.page_count
    }

    /// Get the region as a slice (unsafe - caller must ensure validity)
    pub unsafe fn as_slice(&self) -> &[u8] {
        core::slice::from_raw_parts(self.address, self.total_size() as usize)
    }

    /// Get the region as a mutable slice (unsafe - caller must ensure validity and write access)
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        if matches!(self.mode, JoinMode::ReadOnly) {
            panic!("Cannot get mutable slice for read-only region");
        }
        core::slice::from_raw_parts_mut(self.address, self.total_size() as usize)
    }
}

unsafe impl Send for JoinInfo {}
unsafe impl Sync for JoinInfo {}

/// A handle to a joined shared memory region
pub struct ShmemJoin {
    join_handle: *mut core::ffi::c_void,
    info: JoinInfo,
}

impl ShmemJoin {
    /// Get information about this join
    pub fn info(&self) -> &JoinInfo {
        &self.info
    }

    /// Get the region as a slice
    pub fn as_slice(&self) -> &[u8] {
        unsafe { self.info.as_slice() }
    }

    /// Get the region as a mutable slice (only for read-write regions)
    pub fn as_mut_slice(&mut self) -> ShmemResult<&mut [u8]> {
        if matches!(self.info.mode, JoinMode::ReadOnly) {
            return Err(ShmemError::PermissionDenied(
                "Cannot get mutable access to read-only region".to_string(),
            ));
        }
        Ok(unsafe { self.info.as_mut_slice() })
    }

    /// Get a pointer to the start of the region
    pub fn as_ptr(&self) -> *const u8 {
        self.info.address
    }

    /// Get a mutable pointer to the start of the region (only for read-write regions)
    pub fn as_mut_ptr(&mut self) -> ShmemResult<*mut u8> {
        if matches!(self.info.mode, JoinMode::ReadOnly) {
            return Err(ShmemError::PermissionDenied(
                "Cannot get mutable access to read-only region".to_string(),
            ));
        }
        Ok(self.info.address)
    }
}

impl Drop for ShmemJoin {
    fn drop(&mut self) {
        let result =
            unsafe { libfd_shmem_sys::fd_shmem_leave(self.join_handle, None, ptr::null_mut()) };
        if result != 0 {
            eprintln!(
                "Warning: Failed to leave shared memory region: {}",
                ShmemError::from(result)
            );
        }
    }
}

unsafe impl Send for ShmemJoin {}
unsafe impl Sync for ShmemJoin {}

/// Main interface to the shared memory system
pub struct FdShmem;

impl FdShmem {
    /// Get the number of NUMA nodes in the system
    pub fn numa_count() -> u64 {
        unsafe { libfd_shmem_sys::fd_shmem_numa_cnt() }
    }

    /// Get the number of logical CPUs in the system
    pub fn cpu_count() -> u64 {
        unsafe { libfd_shmem_sys::fd_shmem_cpu_cnt() }
    }

    /// Get the NUMA node index for a given CPU
    pub fn numa_idx_for_cpu(cpu_idx: u64) -> Option<u64> {
        let numa_idx = unsafe { libfd_shmem_sys::fd_shmem_numa_idx(cpu_idx) };
        if numa_idx == u64::MAX {
            None
        } else {
            Some(numa_idx)
        }
    }

    /// Get a CPU index for a given NUMA node
    pub fn cpu_idx_for_numa(numa_idx: u64) -> Option<u64> {
        let cpu_idx = unsafe { libfd_shmem_sys::fd_shmem_cpu_idx(numa_idx) };
        if cpu_idx == u64::MAX {
            None
        } else {
            Some(cpu_idx)
        }
    }

    /// Validate that memory pages are on the expected NUMA node
    pub fn validate_numa_placement(
        memory: &[u8],
        page_size: PageSize,
        cpu_idx: u64,
    ) -> ShmemResult<()> {
        let page_count = (memory.len() + page_size.size_bytes() - 1) / page_size.size_bytes();
        let result = unsafe {
            libfd_shmem_sys::fd_shmem_numa_validate(
                memory.as_ptr() as *const core::ffi::c_void,
                page_size as u64,
                page_count as u64,
                cpu_idx,
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(ShmemError::from(result))
        }
    }

    /// Join (map) a named shared memory region
    pub fn join(name: &str, mode: JoinMode, lock_pages: bool) -> ShmemResult<ShmemJoin> {
        let c_name = CString::new(name).map_err(|_| {
            ShmemError::InvalidArgs("Invalid name: contains null bytes".to_string())
        })?;

        let mut join_info = unsafe { core::mem::zeroed::<libfd_shmem_sys::fd_shmem_join_info>() };

        let join_handle = unsafe {
            libfd_shmem_sys::fd_shmem_join(
                c_name.as_ptr(),
                mode as i32,
                None, // No custom join function
                ptr::null_mut(),
                &mut join_info,
                if lock_pages { 1 } else { 0 },
            )
        };

        if join_handle.is_null() {
            return Err(ShmemError::Unknown(
                "Failed to join shared memory region".to_string(),
            ));
        }

        let page_size = PageSize::from_raw(join_info.page_sz)
            .ok_or_else(|| ShmemError::InvalidArgs("Invalid page size".to_string()))?;

        let join_mode = match join_info.mode {
            x if x == libfd_shmem_sys::FD_SHMEM_JOIN_MODE_READ_ONLY as i32 => JoinMode::ReadOnly,
            x if x == libfd_shmem_sys::FD_SHMEM_JOIN_MODE_READ_WRITE as i32 => JoinMode::ReadWrite,
            _ => return Err(ShmemError::InvalidArgs("Invalid join mode".to_string())),
        };

        let region_name = unsafe {
            CStr::from_ptr(join_info.__bindgen_anon_1.name.as_ptr())
                .to_string_lossy()
                .into_owned()
        };

        let info = JoinInfo {
            ref_count: join_info.ref_cnt,
            address: join_info.shmem as *mut u8,
            page_size,
            page_count: join_info.page_cnt,
            mode: join_mode,
            name: region_name,
        };

        Ok(ShmemJoin { join_handle, info })
    }

    /// Query information about a joined region by name
    pub fn query_by_name(name: &str) -> ShmemResult<JoinInfo> {
        let c_name = CString::new(name).map_err(|_| {
            ShmemError::InvalidArgs("Invalid name: contains null bytes".to_string())
        })?;

        let mut join_info = unsafe { core::mem::zeroed::<libfd_shmem_sys::fd_shmem_join_info>() };

        let result = unsafe {
            libfd_shmem_sys::fd_shmem_join_query_by_name(c_name.as_ptr(), &mut join_info)
        };

        if result != 0 {
            return Err(ShmemError::from(result));
        }

        let page_size = PageSize::from_raw(join_info.page_sz)
            .ok_or_else(|| ShmemError::InvalidArgs("Invalid page size".to_string()))?;

        let join_mode = match join_info.mode {
            x if x == libfd_shmem_sys::FD_SHMEM_JOIN_MODE_READ_ONLY as i32 => JoinMode::ReadOnly,
            x if x == libfd_shmem_sys::FD_SHMEM_JOIN_MODE_READ_WRITE as i32 => JoinMode::ReadWrite,
            _ => return Err(ShmemError::InvalidArgs("Invalid join mode".to_string())),
        };

        let region_name = unsafe {
            CStr::from_ptr(join_info.__bindgen_anon_1.name.as_ptr())
                .to_string_lossy()
                .into_owned()
        };

        Ok(JoinInfo {
            ref_count: join_info.ref_cnt,
            address: join_info.shmem as *mut u8,
            page_size,
            page_count: join_info.page_cnt,
            mode: join_mode,
            name: region_name,
        })
    }

    /// Create a new shared memory region
    pub fn create(
        name: &str,
        page_size: PageSize,
        page_count: u64,
        cpu_idx: u64,
        mode: u64,
    ) -> ShmemResult<()> {
        // fd_shmem_create is an inline function, so we call fd_shmem_create_multi directly
        Self::create_multi(name, page_size, &[(page_count, cpu_idx)], mode)
    }

    /// Create a multi-region shared memory area
    pub fn create_multi(
        name: &str,
        page_size: PageSize,
        sub_regions: &[(u64, u64)], // (page_count, cpu_idx) pairs
        mode: u64,
    ) -> ShmemResult<()> {
        let c_name = CString::new(name).map_err(|_| {
            ShmemError::InvalidArgs("Invalid name: contains null bytes".to_string())
        })?;

        let (page_counts, cpu_indices): (Vec<u64>, Vec<u64>) = sub_regions.iter().cloned().unzip();

        let result = unsafe {
            libfd_shmem_sys::fd_shmem_create_multi(
                c_name.as_ptr(),
                page_size as u64,
                sub_regions.len() as u64,
                page_counts.as_ptr(),
                cpu_indices.as_ptr(),
                mode,
            )
        };

        if result == 0 {
            Ok(())
        } else {
            Err(ShmemError::from(result))
        }
    }

    /// Unlink (delete) a shared memory region
    pub fn unlink(name: &str, page_size: PageSize) -> ShmemResult<()> {
        let c_name = CString::new(name).map_err(|_| {
            ShmemError::InvalidArgs("Invalid name: contains null bytes".to_string())
        })?;

        let result = unsafe { libfd_shmem_sys::fd_shmem_unlink(c_name.as_ptr(), page_size as u64) };

        if result == 0 {
            Ok(())
        } else {
            Err(ShmemError::from(result))
        }
    }

    /// Get information about a shared memory region
    pub fn info(name: &str, page_size: Option<PageSize>) -> ShmemResult<ShmemInfo> {
        let c_name = CString::new(name).map_err(|_| {
            ShmemError::InvalidArgs("Invalid name: contains null bytes".to_string())
        })?;

        let page_sz = page_size.map(|ps| ps as u64).unwrap_or(0);
        let mut info = unsafe { core::mem::zeroed::<libfd_shmem_sys::fd_shmem_info>() };

        let result = unsafe { libfd_shmem_sys::fd_shmem_info(c_name.as_ptr(), page_sz, &mut info) };

        if result != 0 {
            return Err(ShmemError::from(result));
        }

        let page_size = PageSize::from_raw(info.page_sz)
            .ok_or_else(|| ShmemError::InvalidArgs("Invalid page size".to_string()))?;

        Ok(ShmemInfo {
            page_size,
            page_count: info.page_cnt,
        })
    }

    /// Acquire private memory pages
    pub fn acquire(page_size: PageSize, page_count: u64, cpu_idx: u64) -> ShmemResult<*mut u8> {
        // fd_shmem_acquire is an inline function, so we call fd_shmem_acquire_multi directly
        let ptr = unsafe {
            libfd_shmem_sys::fd_shmem_acquire_multi(page_size as u64, 1, &page_count, &cpu_idx)
        };

        if ptr.is_null() {
            Err(ShmemError::MemoryError(
                "Failed to acquire memory pages".to_string(),
            ))
        } else {
            Ok(ptr as *mut u8)
        }
    }

    /// Release private memory pages
    pub fn release(ptr: *mut u8, page_size: PageSize, page_count: u64) -> ShmemResult<()> {
        let result = unsafe {
            libfd_shmem_sys::fd_shmem_release(
                ptr as *mut core::ffi::c_void,
                page_size as u64,
                page_count,
            )
        };

        if result == 0 {
            Ok(())
        } else {
            Err(ShmemError::SystemError(
                "Failed to release memory pages".to_string(),
                result,
            ))
        }
    }

    /// Validate a region name
    pub fn validate_name(name: &str) -> bool {
        let c_name = match CString::new(name) {
            Ok(c_name) => c_name,
            Err(_) => return false,
        };
        let len = unsafe { libfd_shmem_sys::fd_shmem_name_len(c_name.as_ptr()) };
        len > 0
    }
}

/// Iterator over all joined shared memory regions
pub struct ShmemIterator {
    current: *const libfd_shmem_sys::fd_shmem_join_info,
}

impl ShmemIterator {
    /// Create a new iterator over joined regions
    pub fn new() -> Self {
        let current = unsafe { libfd_shmem_sys::fd_shmem_iter_begin() };
        Self { current }
    }
}

impl Iterator for ShmemIterator {
    type Item = JoinInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }

        let join_info = unsafe { &*self.current };

        let page_size = PageSize::from_raw(join_info.page_sz)?;
        let join_mode = match join_info.mode {
            x if x == libfd_shmem_sys::FD_SHMEM_JOIN_MODE_READ_ONLY as i32 => JoinMode::ReadOnly,
            x if x == libfd_shmem_sys::FD_SHMEM_JOIN_MODE_READ_WRITE as i32 => JoinMode::ReadWrite,
            _ => return None,
        };

        let region_name = unsafe {
            CStr::from_ptr(join_info.__bindgen_anon_1.name.as_ptr())
                .to_string_lossy()
                .into_owned()
        };

        let info = JoinInfo {
            ref_count: join_info.ref_cnt,
            address: join_info.shmem as *mut u8,
            page_size,
            page_count: join_info.page_cnt,
            mode: join_mode,
            name: region_name,
        };

        // Advance to next
        self.current = unsafe { libfd_shmem_sys::fd_shmem_iter_next(self.current) };

        Some(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn test_page_size() {
        assert_eq!(PageSize::Normal.size_bytes(), 4096);
        assert_eq!(PageSize::Huge.size_bytes(), 2097152);
        assert_eq!(PageSize::Gigantic.size_bytes(), 1073741824);

        assert_eq!(PageSize::Normal.log2_size(), 12);
        assert_eq!(PageSize::Huge.log2_size(), 21);
        assert_eq!(PageSize::Gigantic.log2_size(), 30);
    }

    #[test]
    fn test_page_size_conversions() {
        assert_eq!(PageSize::from_raw(4096), Some(PageSize::Normal));
        assert_eq!(PageSize::from_raw(2097152), Some(PageSize::Huge));
        assert_eq!(PageSize::from_raw(1073741824), Some(PageSize::Gigantic));
        assert_eq!(PageSize::from_raw(123), None);

        assert_eq!(PageSize::Normal.as_str(), "normal");
        assert_eq!(PageSize::Huge.as_str(), "huge");
        assert_eq!(PageSize::Gigantic.as_str(), "gigantic");

        assert_eq!(PageSize::from_str("normal"), Some(PageSize::Normal));
        assert_eq!(PageSize::from_str("huge"), Some(PageSize::Huge));
        assert_eq!(PageSize::from_str("gigantic"), Some(PageSize::Gigantic));
        assert_eq!(PageSize::from_str("invalid"), None);
    }

    #[test]
    fn test_shmem_info() {
        let info = ShmemInfo {
            page_size: PageSize::Normal,
            page_count: 10,
        };
        assert_eq!(info.total_size(), 40960); // 10 * 4096
    }

    #[test]
    fn test_system_info() {
        let numa_count = FdShmem::numa_count();
        let cpu_count = FdShmem::cpu_count();

        // On macOS with stub implementation, these might be 0
        if numa_count == 0 && cpu_count == 0 {
            println!("Using stub NUMA implementation (likely macOS)");
        } else {
            assert!(numa_count > 0);
            assert!(cpu_count > 0);
            assert!(cpu_count >= numa_count);
        }

        println!("NUMA nodes: {}, CPUs: {}", numa_count, cpu_count);
    }

    #[test]
    fn test_name_validation() {
        assert!(FdShmem::validate_name("test_region"));
        assert!(FdShmem::validate_name("region123"));
        assert!(FdShmem::validate_name("my-region.test"));
        assert!(!FdShmem::validate_name(""));
        assert!(!FdShmem::validate_name("invalid\0name"));
    }

    #[test_case(JoinMode::ReadOnly; "read_only")]
    #[test_case(JoinMode::ReadWrite; "read_write")]
    fn test_join_modes(mode: JoinMode) {
        // Just test that the enum values are correct
        match mode {
            JoinMode::ReadOnly => assert_eq!(mode as i32, 0),
            JoinMode::ReadWrite => assert_eq!(mode as i32, 1),
        }
    }

    #[test]
    fn test_error_types() {
        let err1 = ShmemError::InvalidArgs("test".to_string());
        let err2 = ShmemError::NotFound("test".to_string());
        let err3 = ShmemError::SystemError("test".to_string(), 42);

        assert!(matches!(err1, ShmemError::InvalidArgs(_)));
        assert!(matches!(err2, ShmemError::NotFound(_)));
        assert!(matches!(err3, ShmemError::SystemError(_, 42)));
    }

    #[test]
    fn test_error_from_errno() {
        let err1 = ShmemError::from(libc::EINVAL);
        let err2 = ShmemError::from(libc::ENOENT);
        let err3 = ShmemError::from(libc::EACCES);

        assert!(matches!(err1, ShmemError::InvalidArgs(_)));
        assert!(matches!(err2, ShmemError::NotFound(_)));
        assert!(matches!(err3, ShmemError::PermissionDenied(_)));
    }

    #[test]
    fn test_iterator() {
        // Test that the iterator can be created (may be empty)
        let iter = ShmemIterator::new();
        let regions: Vec<JoinInfo> = iter.collect();
        println!("Found {} joined regions", regions.len());
    }
}
