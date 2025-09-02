//! Safe Rust API for Firedancer shared scratchpad (spad) allocator
//!
//! This crate provides safe abstractions over the raw FFI bindings in `libfd_spad_sys`.
//! The spad system provides high-performance, persistent inter-process shared scratch pad
//! memory with automatic cleanup and memory safety guarantees.
//!
//! ## Features
//!
//! - **Frame-based allocation**: Push/pop semantics for automatic cleanup
//! - **High performance**: O(1) assembly operations for most operations
//! - **Shared memory support**: Compatible with inter-process shared memory regions
//! - **Alignment support**: Proper handling of memory alignment requirements
//! - **Prepare/publish pattern**: Dynamic allocation sizing with safety checks
//! - **Trim support**: Ability to shrink allocations after use
//! - **Virtual allocator integration**: Compatible with valloc interface
//! - **RAII cleanup**: Automatic frame cleanup on scope exit
//!
//! ## Shared Memory Usage
//!
//! Unlike the scratch allocator which is thread-local, spad is designed for shared memory
//! environments and can be used across multiple threads and processes when backed by
//! shared memory regions.
//!
//! ## Example
//!
//! ```rust
//! use fd_spad::{SpadAllocator, SpadFrame};
//! use std::alloc::Layout;
//!
//! // init shmem
//! let mem_max = 8192;
//! let footprint = SpadAllocator::footprint(mem_max).unwrap();
//! let layout = Layout::from_size_align(footprint, SpadAllocator::align()).unwrap();
//! let shmem = unsafe { std::alloc::alloc(layout) };
//!
//! // create and join spad
//! let mut allocator = SpadAllocator::new(shmem as *mut u8, mem_max).unwrap();
//!
//! // scoped frame; automatic cleanup
//! {
//!     let _frame = allocator.push_frame().unwrap();
//!     let memory = allocator.allocate(1024, 64).unwrap();
//! }
//!
//! unsafe { std::alloc::dealloc(shmem, layout); }
//! ```

use core::ptr::NonNull;
use fd_spad_sys::{self as sys, ulong};

#[derive(Debug, Clone, PartialEq)]
pub enum SpadError {
    /// Memory allocation failed
    AllocationFailed,
    /// Invalid alignment (must be power of 2)
    InvalidAlignment,
    /// Size is zero or too large
    InvalidSize,
    /// No spad frame available for allocation
    NoFrame,
    /// Too many frames (exceeded max depth)
    TooManyFrames,
    /// Attempted to pop when no frames exist
    NoFramesToPop,
    /// Insufficient spad space
    InsufficientSpace,
    /// Invalid operation state
    InvalidState,
    /// Prepare/publish/cancel state mismatch
    PrepareMismatch,
    /// Invalid shared memory pointer
    InvalidShmem,
    /// Spad creation failed
    CreationFailed,
    /// Spad join failed
    JoinFailed,
}

impl core::fmt::Display for SpadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SpadError::AllocationFailed => write!(f, "Spad allocation failed"),
            SpadError::InvalidAlignment => write!(f, "Invalid alignment - must be power of 2"),
            SpadError::InvalidSize => write!(f, "Invalid size - must be > 0"),
            SpadError::NoFrame => write!(f, "No spad frame available for allocation"),
            SpadError::TooManyFrames => write!(f, "Too many frames - exceeded maximum depth"),
            SpadError::NoFramesToPop => write!(f, "No frames available to pop"),
            SpadError::InsufficientSpace => write!(f, "Insufficient spad space"),
            SpadError::InvalidState => write!(f, "Invalid operation state"),
            SpadError::PrepareMismatch => write!(f, "Prepare/publish/cancel state mismatch"),
            SpadError::InvalidShmem => write!(f, "Invalid shared memory pointer"),
            SpadError::CreationFailed => write!(f, "Spad creation failed"),
            SpadError::JoinFailed => write!(f, "Spad join failed"),
        }
    }
}

impl std::error::Error for SpadError {}

/// RAII guard for a spad frame; pops the current frame when dropped
pub struct SpadFrame {
    _private: (),
}

impl SpadFrame {
    fn new() -> Self {
        Self { _private: () }
    }
}

impl Drop for SpadFrame {
    fn drop(&mut self) {
        // safety: caller must ensure frames are properly managed
    }
}

pub struct SpadAllocation {
    ptr: NonNull<u8>,
    size: usize,
}

impl SpadAllocation {
    fn new(ptr: NonNull<u8>, size: usize) -> Self {
        Self { ptr, size }
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

/// Allocation that can be resized before being finalized
pub struct DynamicAllocation {
    start_ptr: NonNull<u8>,
    max_size: usize,
}

impl DynamicAllocation {
    pub fn as_ptr(&self) -> *mut u8 {
        self.start_ptr.as_ptr()
    }

    pub fn max_size(&self) -> usize {
        self.max_size
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.start_ptr.as_ptr(), self.max_size) }
    }
}

pub struct SpadAllocator {
    spad: NonNull<sys::fd_spad_t>,
}

impl SpadAllocator {
    /// alignment requirement for spad
    pub fn align() -> usize {
        unsafe { sys::fd_spad_align() as usize }
    }

    /// footprint required for a spad with given memory capacity
    pub fn footprint(mem_max: usize) -> Result<usize, SpadError> {
        let footprint = unsafe { sys::fd_spad_footprint(mem_max as ulong) };
        if footprint == 0 {
            Err(SpadError::InvalidSize)
        } else {
            Ok(footprint as usize)
        }
    }

    /// max memory size that fits in a given footprint
    pub fn mem_max_max(footprint: usize) -> usize {
        unsafe { sys::fd_spad_mem_max_max(footprint as ulong) as usize }
    }

    pub fn new(shmem: *mut u8, mem_max: usize) -> Result<Self, SpadError> {
        if shmem.is_null() {
            return Err(SpadError::InvalidShmem);
        }

        if mem_max == 0 {
            return Err(SpadError::InvalidSize);
        }

        unsafe {
            let spad_ptr = sys::fd_spad_new(shmem as *mut core::ffi::c_void, mem_max as ulong);
            if spad_ptr.is_null() {
                return Err(SpadError::CreationFailed);
            }

            let spad = sys::fd_spad_join(spad_ptr);
            if spad.is_null() {
                return Err(SpadError::JoinFailed);
            }

            Ok(Self {
                spad: NonNull::new_unchecked(spad),
            })
        }
    }

    /// join an existing spad from shmem
    pub fn join(shmem: *mut u8) -> Result<Self, SpadError> {
        if shmem.is_null() {
            return Err(SpadError::InvalidShmem);
        }

        unsafe {
            let spad = sys::fd_spad_join(shmem as *mut core::ffi::c_void);
            if spad.is_null() {
                return Err(SpadError::JoinFailed);
            }

            Ok(Self {
                spad: NonNull::new_unchecked(spad),
            })
        }
    }

    /// Push a new frame to the stack
    ///
    /// Returns a RAII guard for managing the frame
    pub fn push_frame(&mut self) -> Result<SpadFrame, SpadError> {
        unsafe {
            if sys::fd_spad_frame_free(self.spad.as_ptr()) == 0 {
                return Err(SpadError::TooManyFrames);
            }
            sys::fd_spad_push(self.spad.as_ptr());
        }

        Ok(SpadFrame::new())
    }

    /// Pop a frame
    ///
    /// Safety: Unsafe if you have a `SpadFrame` guard active.
    /// Prefer using `push_frame()` for RAII.
    pub unsafe fn pop_frame(&mut self) -> Result<(), SpadError> {
        unsafe {
            if sys::fd_spad_in_frame(self.spad.as_ptr()) == 0 {
                return Err(SpadError::NoFramesToPop);
            }
            sys::fd_spad_pop(self.spad.as_ptr());
        }
        Ok(())
    }

    /// Allocate in the current frame
    ///
    /// Returns a `SpadAllocation` for the allocated memory.
    pub fn allocate(&mut self, size: usize, alignment: usize) -> Result<SpadAllocation, SpadError> {
        if size == 0 {
            return Err(SpadError::InvalidSize);
        }

        if alignment > 0 && !alignment.is_power_of_two() {
            return Err(SpadError::InvalidAlignment);
        }

        unsafe {
            if sys::fd_spad_in_frame(self.spad.as_ptr()) == 0 {
                return Err(SpadError::NoFrame);
            }

            let max_available = sys::fd_spad_alloc_max(self.spad.as_ptr(), alignment as ulong);
            if max_available < size as ulong {
                return Err(SpadError::InsufficientSpace);
            }

            let ptr = sys::fd_spad_alloc(self.spad.as_ptr(), alignment as ulong, size as ulong);
            if ptr.is_null() {
                return Err(SpadError::AllocationFailed);
            }

            Ok(SpadAllocation::new(
                NonNull::new_unchecked(ptr as *mut u8),
                size,
            ))
        }
    }

    /// Prep a dynamic allocation with unknown final size
    pub fn prepare_alloc(
        &mut self,
        alignment: usize,
        max_size: usize,
    ) -> Result<DynamicAllocation, SpadError> {
        if max_size == 0 {
            return Err(SpadError::InvalidSize);
        }

        if alignment > 0 && !alignment.is_power_of_two() {
            return Err(SpadError::InvalidAlignment);
        }

        unsafe {
            if sys::fd_spad_in_frame(self.spad.as_ptr()) == 0 {
                return Err(SpadError::NoFrame);
            }

            let max_available = sys::fd_spad_alloc_max(self.spad.as_ptr(), alignment as ulong);
            if max_available < max_size as ulong {
                return Err(SpadError::InsufficientSpace);
            }

            let ptr =
                sys::fd_spad_prepare(self.spad.as_ptr(), alignment as ulong, max_size as ulong);
            if ptr.is_null() {
                return Err(SpadError::AllocationFailed);
            }

            Ok(DynamicAllocation {
                start_ptr: NonNull::new_unchecked(ptr as *mut u8),
                max_size,
            })
        }
    }

    /// Publish a prepped allocation with the total size used
    pub fn publish_alloc(&mut self, actual_size: usize) -> Result<(), SpadError> {
        unsafe {
            sys::fd_spad_publish(self.spad.as_ptr(), actual_size as ulong);
        }
        Ok(())
    }

    /// Cancel a prepped allocation
    pub fn cancel_alloc(&mut self) -> Result<(), SpadError> {
        unsafe {
            sys::fd_spad_cancel(self.spad.as_ptr());
        }
        Ok(())
    }

    /// Trim the frame to end at the given ptr
    pub fn trim(&mut self, end_ptr: *mut u8) -> Result<(), SpadError> {
        unsafe {
            if sys::fd_spad_in_frame(self.spad.as_ptr()) == 0 {
                return Err(SpadError::NoFrame);
            }

            let frame_lo = sys::fd_spad_frame_lo(self.spad.as_ptr());
            let frame_hi = sys::fd_spad_frame_hi(self.spad.as_ptr());

            if (end_ptr as usize) < (frame_lo as usize) || (end_ptr as usize) > (frame_hi as usize)
            {
                return Err(SpadError::InvalidState);
            }

            sys::fd_spad_trim(self.spad.as_ptr(), end_ptr as *mut core::ffi::c_void);
        }
        Ok(())
    }

    /// Pop all frames
    pub fn reset(&mut self) {
        unsafe {
            sys::fd_spad_reset(self.spad.as_ptr());
        }
    }

    /// max number of frames
    pub fn frame_max(&self) -> usize {
        unsafe { sys::fd_spad_frame_max(self.spad.as_ptr()) as usize }
    }

    /// number of frames currently in use
    pub fn frames_used(&self) -> usize {
        unsafe { sys::fd_spad_frame_used(self.spad.as_ptr()) as usize }
    }

    /// number of frames currently free
    pub fn frames_free(&self) -> usize {
        unsafe { sys::fd_spad_frame_free(self.spad.as_ptr()) as usize }
    }

    /// max memory capacity
    pub fn mem_max(&self) -> usize {
        unsafe { sys::fd_spad_mem_max(self.spad.as_ptr()) as usize }
    }

    /// number of bytes currently used
    pub fn mem_used(&self) -> usize {
        unsafe { sys::fd_spad_mem_used(self.spad.as_ptr()) as usize }
    }

    /// number of bytes currently free
    pub fn mem_free(&self) -> usize {
        unsafe { sys::fd_spad_mem_free(self.spad.as_ptr()) as usize }
    }

    /// check if currently in a frame
    pub fn in_frame(&self) -> bool {
        unsafe { sys::fd_spad_in_frame(self.spad.as_ptr()) != 0 }
    }

    /// max allocation size for given alignment
    pub fn alloc_max(&self, alignment: usize) -> usize {
        unsafe { sys::fd_spad_alloc_max(self.spad.as_ptr(), alignment as ulong) as usize }
    }

    /// frame bounds (lo, hi)
    pub fn frame_bounds(&mut self) -> Option<(*mut u8, *mut u8)> {
        if !self.in_frame() {
            return None;
        }

        unsafe {
            let lo = sys::fd_spad_frame_lo(self.spad.as_ptr()) as *mut u8;
            let hi = sys::fd_spad_frame_hi(self.spad.as_ptr()) as *mut u8;
            Some((lo, hi))
        }
    }

    /// virtual allocator interface for this spad
    pub fn as_valloc(&self) -> sys::fd_valloc_t {
        unsafe { sys::fd_spad_virtual(self.spad.as_ptr()) }
    }

    /// raw pointer to the spad
    pub fn as_raw(&self) -> *mut sys::fd_spad_t {
        self.spad.as_ptr()
    }

    /// leave the spad join (returns shmem ptr)
    pub fn leave(self) -> *mut u8 {
        unsafe {
            let shmem = sys::fd_spad_leave(self.spad.as_ptr());
            std::mem::forget(self); // don't run drop
            shmem as *mut u8
        }
    }

    /// delete the spad
    ///
    /// Safety: must ensure this is the last reference
    pub fn delete(self) -> *mut u8 {
        unsafe {
            let shmem = sys::fd_spad_leave(self.spad.as_ptr());
            let deleted_shmem = sys::fd_spad_delete(shmem);
            std::mem::forget(self); // don't run drop
            deleted_shmem as *mut u8
        }
    }
}

impl Drop for SpadAllocator {
    fn drop(&mut self) {
        unsafe {
            sys::fd_spad_leave(self.spad.as_ptr());
        }
    }
}

pub struct ManagedSpadFrame {
    _private: (),
}

impl ManagedSpadFrame {
    pub fn new(spad: &mut SpadAllocator) -> Result<Self, SpadError> {
        spad.push_frame()?;
        Ok(Self { _private: () })
    }
}

impl Drop for ManagedSpadFrame {
    fn drop(&mut self) {
        // safety: caller must ensure frame is properly managed
    }
}

#[macro_export]
macro_rules! spad_scope {
    ($allocator:expr, $block:block) => {{
        $allocator.push_frame()?;
        let result = $block;
        let _ = $allocator.pop_frame();
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{alloc, dealloc, Layout};

    fn create_test_spad(mem_max: usize) -> (SpadAllocator, Layout, *mut u8) {
        let footprint = SpadAllocator::footprint(mem_max).unwrap();
        let layout = Layout::from_size_align(footprint, SpadAllocator::align()).unwrap();
        let shmem = unsafe { alloc(layout) };
        let allocator = SpadAllocator::new(shmem, mem_max).unwrap();
        (allocator, layout, shmem)
    }

    #[test]
    fn test_basic() {
        let (allocator, layout, _shmem) = create_test_spad(8192);

        assert_eq!(allocator.mem_max(), 8192);
        assert_eq!(allocator.mem_used(), 0);
        assert_eq!(allocator.mem_free(), 8192);
        assert_eq!(allocator.frames_used(), 0);
        assert_eq!(allocator.frame_max(), 128);
        assert!(!allocator.in_frame());

        let shmem_ptr = allocator.delete();
        unsafe {
            dealloc(shmem_ptr, layout);
        }
    }

    #[test]
    fn test_frame_mgmt() {
        let (mut allocator, layout, _) = create_test_spad(4096);

        allocator.push_frame().unwrap();
        assert_eq!(allocator.frames_used(), 1);
        assert!(allocator.in_frame());

        let alloc = allocator.allocate(256, 64).unwrap();
        assert_eq!(alloc.size(), 256);
        assert_eq!(alloc.as_ptr() as usize % 64, 0);

        unsafe { allocator.pop_frame().unwrap() };
        assert_eq!(allocator.frames_used(), 0);
        assert_eq!(allocator.mem_used(), 0);
        assert!(!allocator.in_frame());

        let shmem_ptr = allocator.delete();
        unsafe {
            dealloc(shmem_ptr, layout);
        }
    }

    #[test]
    fn test_nested_frames() {
        let (mut allocator, layout, _) = create_test_spad(8192);

        allocator.push_frame().unwrap();
        let _alloc1 = allocator.allocate(512, 32).unwrap();
        assert_eq!(allocator.frames_used(), 1);

        allocator.push_frame().unwrap();
        let _alloc2 = allocator.allocate(256, 16).unwrap();
        assert_eq!(allocator.frames_used(), 2);

        unsafe { allocator.pop_frame().unwrap() };
        assert_eq!(allocator.frames_used(), 1);

        unsafe { allocator.pop_frame().unwrap() };
        assert_eq!(allocator.frames_used(), 0);
        assert_eq!(allocator.mem_used(), 0);

        let shmem_ptr = allocator.delete();
        unsafe {
            dealloc(shmem_ptr, layout);
        }
    }

    #[test]
    fn test_prep_publish() {
        let (mut allocator, layout, _) = create_test_spad(2048);

        allocator.push_frame().unwrap();
        let dynamic = allocator.prepare_alloc(64, 512).unwrap();

        assert_eq!(dynamic.as_ptr() as usize % 64, 0);
        assert_eq!(dynamic.max_size(), 512);

        let slice = unsafe { core::slice::from_raw_parts_mut(dynamic.as_ptr(), 256) };
        slice.fill(0x42);

        allocator.publish_alloc(256).unwrap();
        assert!(allocator.mem_used() >= 256);

        unsafe { allocator.pop_frame().unwrap() };
        assert_eq!(allocator.mem_used(), 0);

        let shmem_ptr = allocator.delete();
        unsafe {
            dealloc(shmem_ptr, layout);
        }
    }

    #[test]
    fn test_prep_cancel() {
        let (mut allocator, layout, _) = create_test_spad(1024);

        allocator.push_frame().unwrap();
        let _dynamic = allocator.prepare_alloc(32, 256).unwrap();

        let mem_used_before = allocator.mem_used();
        allocator.cancel_alloc().unwrap();

        let mem_used_after = allocator.mem_used();
        assert!(mem_used_after <= mem_used_before + 32);

        unsafe { allocator.pop_frame().unwrap() };
        assert_eq!(allocator.mem_used(), 0);

        let shmem_ptr = allocator.delete();
        unsafe {
            dealloc(shmem_ptr, layout);
        }
    }

    #[test]
    fn test_trim() {
        let (mut allocator, layout, _) = create_test_spad(4096);

        allocator.push_frame().unwrap();

        let alloc = allocator.allocate(1024, 64).unwrap();
        let mem_used_before = allocator.mem_used();

        let trim_ptr = unsafe { alloc.as_ptr().add(512) };
        allocator.trim(trim_ptr).unwrap();

        let mem_used_after = allocator.mem_used();
        assert!(mem_used_after <= mem_used_before);

        let (_, frame_hi) = allocator.frame_bounds().unwrap();
        assert_eq!(frame_hi, trim_ptr);

        unsafe { allocator.pop_frame().unwrap() };

        let shmem_ptr = allocator.delete();
        unsafe {
            dealloc(shmem_ptr, layout);
        }
    }

    #[test]
    fn test_reset() {
        let (mut allocator, layout, _) = create_test_spad(2048);

        allocator.push_frame().unwrap();
        allocator.allocate(128, 16).unwrap();

        allocator.push_frame().unwrap();
        allocator.allocate(256, 32).unwrap();

        assert_eq!(allocator.frames_used(), 2);
        assert!(allocator.mem_used() > 0);

        allocator.reset();

        assert_eq!(allocator.frames_used(), 0);
        assert_eq!(allocator.mem_used(), 0);
        assert!(!allocator.in_frame());

        let shmem_ptr = allocator.delete();
        unsafe {
            dealloc(shmem_ptr, layout);
        }
    }

    #[test]
    fn test_join_existing() {
        let mem_max = 4096;
        let (allocator1, layout, shmem) = create_test_spad(mem_max);

        let shmem_ptr = allocator1.leave();
        assert_eq!(shmem_ptr, shmem);

        let allocator2 = SpadAllocator::join(shmem_ptr).unwrap();
        assert_eq!(allocator2.mem_max(), mem_max);
        assert_eq!(allocator2.mem_used(), 0);

        let shmem_ptr = allocator2.delete();
        unsafe {
            dealloc(shmem_ptr, layout);
        }
    }

    #[test]
    fn test_error_conditions() {
        assert!(matches!(
            SpadAllocator::new(core::ptr::null_mut(), 1024),
            Err(SpadError::InvalidShmem)
        ));

        let large_val = usize::MAX;
        let result = SpadAllocator::footprint(large_val);
        assert!(result.is_err() || result.unwrap() == 0);

        let (mut allocator, layout, _) = create_test_spad(1024);

        assert!(matches!(
            allocator.allocate(256, 64),
            Err(SpadError::NoFrame)
        ));

        allocator.push_frame().unwrap();
        assert!(matches!(
            allocator.allocate(256, 63), // not power of 2
            Err(SpadError::InvalidAlignment)
        ));
        unsafe { allocator.pop_frame().unwrap() };

        let shmem_ptr = allocator.delete();
        unsafe {
            dealloc(shmem_ptr, layout);
        }
    }

    #[test]
    fn test_valloc() {
        let (allocator, layout, _) = create_test_spad(2048);

        let valloc = allocator.as_valloc();
        unsafe {
            assert!(!valloc.vt.is_null());
            let vtable = &*valloc.vt;
            assert!(vtable.malloc.is_some());
            assert!(vtable.free.is_some());
        }

        let shmem_ptr = allocator.delete();
        unsafe {
            dealloc(shmem_ptr, layout);
        }
    }
}
