//! Safe Rust API for Firedancer virtual memory allocator (valloc)
//!
//! This crate provides safe abstractions over the raw FFI bindings in `libfd_valloc_sys`.
//! The valloc system provides a virtual table-based approach to memory allocation,
//! allowing different allocation strategies to be swapped at runtime.
//!
//! ## Features
//!
//! - **LibcAllocator**: Standard libc-based allocator using `aligned_alloc`
//! - **NullAllocator**: Special allocator that always returns null (for testing)
//! - **BacktracingAllocator**: Debug allocator that tracks allocations for leak detection
//! - **Safe memory management**: Automatic cleanup and memory safety guarantees
//! - **Alignment support**: Proper handling of memory alignment requirements
//!
//! ## Example
//!
//! ```rust
//! use fd_valloc::{LibcAllocator, VirtualAllocator};
//!
//! let allocator = LibcAllocator::new();
//! let memory = allocator.allocate(1024, 64).unwrap();
//! // Memory is automatically freed when `memory` is dropped
//! ```

use core::ptr::NonNull;
use fd_valloc_sys::{
    self as sys, fd_backtracing_alloc_virtual, fd_is_null_alloc_virtual, fd_libc_alloc_virtual,
    fd_null_alloc_virtual, fd_valloc_free, fd_valloc_malloc, ulong,
};

#[derive(Debug, Clone, PartialEq)]
pub enum VAllocError {
    /// Allocation failed (out of memory or invalid parameters)
    AllocationFailed,
    /// Invalid alignment (must be power of 2)
    InvalidAlignment,
    /// Size is zero or too large
    InvalidSize,
    /// Allocator is null or invalid
    InvalidAllocator,
}

impl core::fmt::Display for VAllocError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VAllocError::AllocationFailed => write!(f, "Memory allocation failed"),
            VAllocError::InvalidAlignment => write!(f, "Invalid alignment - must be power of 2"),
            VAllocError::InvalidSize => write!(f, "Invalid size - must be > 0"),
            VAllocError::InvalidAllocator => write!(f, "Invalid or null allocator"),
        }
    }
}

impl core::error::Error for VAllocError {}

pub struct AllocatedMemory<'a> {
    ptr: NonNull<u8>,
    size: usize,
    allocator: &'a dyn VirtualAllocator,
}

impl<'a> AllocatedMemory<'a> {
    fn new(ptr: NonNull<u8>, size: usize, allocator: &'a dyn VirtualAllocator) -> Self {
        Self {
            ptr,
            size,
            allocator,
        }
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size) }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }
}

impl<'a> Drop for AllocatedMemory<'a> {
    fn drop(&mut self) {
        self.allocator.deallocate_raw(self.ptr.as_ptr());
    }
}

pub trait VirtualAllocator {
    fn allocate(&self, size: usize, alignment: usize) -> Result<AllocatedMemory<'_>, VAllocError>;
    fn deallocate_raw(&self, ptr: *mut u8);
}

pub struct LibcAllocator {
    valloc: sys::fd_valloc_t,
}

impl LibcAllocator {
    pub fn new() -> Self {
        Self {
            valloc: unsafe { fd_libc_alloc_virtual() },
        }
    }
}

impl Default for LibcAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualAllocator for LibcAllocator {
    fn allocate(&self, size: usize, alignment: usize) -> Result<AllocatedMemory<'_>, VAllocError> {
        if size == 0 {
            return Err(VAllocError::InvalidSize);
        }

        if !alignment.is_power_of_two() {
            return Err(VAllocError::InvalidAlignment);
        }

        unsafe {
            let ptr = fd_valloc_malloc(self.valloc, alignment as ulong, size as ulong);
            if ptr.is_null() {
                Err(VAllocError::AllocationFailed)
            } else {
                Ok(AllocatedMemory::new(
                    NonNull::new_unchecked(ptr as *mut u8),
                    size,
                    self,
                ))
            }
        }
    }

    fn deallocate_raw(&self, ptr: *mut u8) {
        unsafe {
            fd_valloc_free(self.valloc, ptr as *mut core::ffi::c_void);
        }
    }
}

pub struct NullAllocator {
    valloc: sys::fd_valloc_t,
}

impl NullAllocator {
    pub fn new() -> Self {
        unsafe {
            Self {
                valloc: fd_null_alloc_virtual(),
            }
        }
    }

    pub fn is_null(&self) -> bool {
        unsafe { fd_is_null_alloc_virtual(self.valloc) != 0 }
    }
}

impl Default for NullAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualAllocator for NullAllocator {
    fn allocate(
        &self,
        _size: usize,
        _alignment: usize,
    ) -> Result<AllocatedMemory<'_>, VAllocError> {
        Err(VAllocError::InvalidAllocator)
    }

    fn deallocate_raw(&self, _ptr: *mut u8) {
        // null allocator doesn't allocate, so nothing to free
    }
}

#[cfg(feature = "hosted")]
pub struct BacktracingAllocator {
    valloc: sys::fd_valloc_t,
    inner: LibcAllocator,
}

#[cfg(feature = "hosted")]
impl BacktracingAllocator {
    pub fn new() -> Self {
        let inner = LibcAllocator::new();
        unsafe {
            Self {
                valloc: fd_backtracing_alloc_virtual(&inner.valloc as *const _ as *mut _),
                inner,
            }
        }
    }

    pub fn wrap(inner: LibcAllocator) -> Self {
        unsafe {
            Self {
                valloc: fd_backtracing_alloc_virtual(&inner.valloc as *const _ as *mut _),
                inner,
            }
        }
    }
}

#[cfg(feature = "hosted")]
impl Default for BacktracingAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "hosted")]
impl VirtualAllocator for BacktracingAllocator {
    fn allocate(&self, size: usize, alignment: usize) -> Result<AllocatedMemory, VAllocError> {
        if size == 0 {
            return Err(VAllocError::InvalidSize);
        }

        if !alignment.is_power_of_two() {
            return Err(VAllocError::InvalidAlignment);
        }

        unsafe {
            let ptr = fd_valloc_malloc(self.valloc, alignment as ulong, size as ulong);
            if ptr.is_null() {
                Err(VAllocError::AllocationFailed)
            } else {
                Ok(AllocatedMemory::new(
                    NonNull::new_unchecked(ptr as *mut u8),
                    size,
                    self,
                ))
            }
        }
    }

    fn deallocate_raw(&self, ptr: *mut u8) {
        unsafe {
            sys::fd_valloc_free(self.valloc, ptr as *mut core::ffi::c_void);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_libc_allocator_basic() {
        let allocator = LibcAllocator::new();
        let memory = allocator.allocate(1024, 64).unwrap();

        assert_eq!(memory.size(), 1024);
        assert_eq!(memory.as_ptr() as usize % 64, 0);
    }

    #[test]
    fn test_libc_allocator_various_sizes() {
        let allocator = LibcAllocator::new();

        for size in [1, 64, 1024, 4096, 65536] {
            let memory = allocator.allocate(size, 64).unwrap();
            assert_eq!(memory.size(), size);
            assert_eq!(memory.as_ptr() as usize % 64, 0);
        }
    }

    #[test]
    fn test_libc_allocator_various_alignments() {
        let allocator = LibcAllocator::new();

        for align_exp in 3..=12 {
            // 8 bytes to 4KB
            let alignment = 1 << align_exp;
            let memory = allocator.allocate(alignment * 2, alignment).unwrap();
            assert_eq!(memory.as_ptr() as usize % alignment, 0);
        }
    }

    #[test]
    fn test_invalid_parameters() {
        let allocator = LibcAllocator::new();
        assert!(matches!(
            allocator.allocate(0, 64),
            Err(VAllocError::InvalidSize)
        ));

        assert!(matches!(
            allocator.allocate(1024, 63),
            Err(VAllocError::InvalidAlignment)
        ));
    }

    #[test]
    fn test_null_allocator() {
        let allocator = NullAllocator::new();
        assert!(allocator.is_null());

        assert!(matches!(
            allocator.allocate(1024, 64),
            Err(VAllocError::InvalidAllocator)
        ));
    }

    #[test]
    fn test_allocated_memory_slice_access() {
        let allocator = LibcAllocator::new();
        let mut memory = allocator.allocate(1024, 64).unwrap();

        let slice = memory.as_mut_slice();
        assert_eq!(slice.len(), 1024);

        slice[0] = 0x42;
        slice[1023] = 0x24;

        let slice = memory.as_slice();
        assert_eq!(slice[0], 0x42);
        assert_eq!(slice[1023], 0x24);
    }

    #[test]
    fn test_multiple_allocations() {
        let allocator = LibcAllocator::new();
        let mut allocations = Vec::new();

        for i in 1..=10 {
            let memory = allocator.allocate(i * 64, 64).unwrap();
            allocations.push(memory);
        }

        for (i, memory) in allocations.iter().enumerate() {
            assert_eq!(memory.size(), (i + 1) * 64);
            assert_eq!(memory.as_ptr() as usize % 64, 0);
        }
    }

    #[cfg(feature = "hosted")]
    #[test]
    fn test_backtracing_allocator() {
        let allocator = BacktracingAllocator::new();
        let memory = allocator.allocate(1024, 64).unwrap();

        assert_eq!(memory.size(), 1024);
        assert_eq!(memory.as_ptr() as usize % 64, 0);

        // should output tracing to stdout
    }
}
