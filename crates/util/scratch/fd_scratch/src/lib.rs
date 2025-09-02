//! Safe Rust API for Firedancer scratch pad memory allocator
//!
//! This crate provides safe abstractions over the raw FFI bindings in `libfd_scratch_sys`.
//! The scratch system provides high-performance, frame-based temporary memory allocation
//! with automatic cleanup and memory safety guarantees.
//!
//! ## Features
//!
//! - **Frame-based allocation**: Push/pop semantics for automatic cleanup
//! - **High performance**: O(3-5) assembly operations for most operations
//! - **Alignment support**: Proper handling of memory alignment requirements
//! - **Prepare/publish pattern**: Dynamic allocation sizing with safety checks
//! - **Trim support**: Ability to shrink allocations after use
//! - **Virtual allocator integration**: Compatible with valloc interface
//! - **RAII cleanup**: Automatic frame cleanup on scope exit
//!
//! ## Example
//!
//! ```rust
//! use fd_scratch::{ScratchAllocator, ScratchFrame};
//!
//! // create scratch allocator (8KB space, 16 framedepth)
//! let mut allocator = ScratchAllocator::new(8192, 16).unwrap();
//!
//! // scoped frame; automatic cleanup
//! {
//!     let _frame = allocator.push_frame().unwrap();
//!     let memory = allocator.allocate(1024, 64).unwrap();
//! }
//! ```

use core::ptr::{self, NonNull};
use fd_scratch_sys::{self as sys, ulong};
use std::alloc::{self, Layout};

#[derive(Debug, Clone, PartialEq)]
pub enum ScratchError {
    /// Memory allocation failed
    AllocationFailed,
    /// Invalid alignment (must be power of 2)
    InvalidAlignment,
    /// Size is zero or too large
    InvalidSize,
    /// No scratch frame available for allocation
    NoFrame,
    /// Too many frames (exceeded max depth)
    TooManyFrames,
    /// Attempted to pop when no frames exist
    NoFramesToPop,
    /// Insufficient scratch space
    InsufficientSpace,
    /// Invalid operation state
    InvalidState,
    /// Prepare/publish/cancel state mismatch
    PrepareMismatch,
}

impl core::fmt::Display for ScratchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ScratchError::AllocationFailed => write!(f, "Scratch allocation failed"),
            ScratchError::InvalidAlignment => write!(f, "Invalid alignment - must be power of 2"),
            ScratchError::InvalidSize => write!(f, "Invalid size - must be > 0"),
            ScratchError::NoFrame => write!(f, "No scratch frame available for allocation"),
            ScratchError::TooManyFrames => write!(f, "Too many frames - exceeded maximum depth"),
            ScratchError::NoFramesToPop => write!(f, "No frames available to pop"),
            ScratchError::InsufficientSpace => write!(f, "Insufficient scratch space"),
            ScratchError::InvalidState => write!(f, "Invalid operation state"),
            ScratchError::PrepareMismatch => write!(f, "Prepare/publish/cancel state mismatch"),
        }
    }
}

impl std::error::Error for ScratchError {}

/// RAII guard for a scratch frame
pub struct ScratchFrame {
    _private: (),
}

impl ScratchFrame {
    fn new() -> Self {
        Self { _private: () }
    }
}

impl Drop for ScratchFrame {
    fn drop(&mut self) {
        unsafe {
            if sys::fd_scratch_pop_is_safe() != 0 {
                sys::fd_scratch_pop();
            }
        }
    }
}

pub struct ScratchAllocation {
    ptr: NonNull<u8>,
    size: usize,
}

impl ScratchAllocation {
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

    /// trim the allocation to a smaller size, moving the position of `end_ptr`
    /// to the first byte after the desired allocation.
    pub fn trim(&mut self, new_size: usize) -> Result<(), ScratchError> {
        if new_size > self.size {
            return Err(ScratchError::InvalidSize);
        }

        let end_ptr = unsafe { self.ptr.as_ptr().add(new_size) };

        unsafe {
            if sys::fd_scratch_trim_is_safe(end_ptr as *mut core::ffi::c_void) == 0 {
                return Err(ScratchError::InvalidState);
            }
            sys::fd_scratch_trim(end_ptr as *mut core::ffi::c_void);
        }

        self.size = new_size;
        Ok(())
    }
}

pub struct ScratchAllocator {
    smem: NonNull<u8>,
    fmem: NonNull<u8>,
    smem_layout: Layout,
    fmem_layout: Layout,
}

impl ScratchAllocator {
    pub fn new(memory_size: usize, max_frames: usize) -> Result<Self, ScratchError> {
        if memory_size == 0 || max_frames == 0 {
            return Err(ScratchError::InvalidSize);
        }

        let smem_align = unsafe { sys::fd_scratch_smem_align() } as usize;
        let fmem_align = unsafe { sys::fd_scratch_fmem_align() } as usize;

        let smem_footprint =
            unsafe { sys::fd_scratch_smem_footprint(memory_size as ulong) } as usize;
        let fmem_footprint =
            unsafe { sys::fd_scratch_fmem_footprint(max_frames as ulong) } as usize;

        let smem_layout = Layout::from_size_align(smem_footprint, smem_align)
            .map_err(|_| ScratchError::InvalidAlignment)?;
        let smem = NonNull::new(unsafe { alloc::alloc(smem_layout) })
            .ok_or(ScratchError::AllocationFailed)?;

        let fmem_layout = Layout::from_size_align(fmem_footprint, fmem_align)
            .map_err(|_| ScratchError::InvalidAlignment)?;
        let fmem = NonNull::new(unsafe { alloc::alloc(fmem_layout) })
            .ok_or(ScratchError::AllocationFailed)?;

        let mut allocator = Self {
            smem,
            fmem,
            smem_layout,
            fmem_layout,
        };

        allocator.attach(memory_size, max_frames)?;

        Ok(allocator)
    }

    fn attach(&mut self, memory_size: usize, max_frames: usize) -> Result<(), ScratchError> {
        unsafe {
            if sys::fd_scratch_attach_is_safe() == 0 {
                return Err(ScratchError::InvalidState);
            }

            sys::fd_scratch_attach(
                self.smem.as_ptr() as *mut core::ffi::c_void,
                self.fmem.as_ptr() as *mut core::ffi::c_void,
                memory_size as ulong,
                max_frames as ulong,
            );
        }

        Ok(())
    }

    /// Push a new frame onto the stack
    ///
    /// Returns: RAII guard for the new frame
    pub fn push_frame(&mut self) -> Result<ScratchFrame, ScratchError> {
        unsafe {
            if sys::fd_scratch_push_is_safe() == 0 {
                return Err(ScratchError::TooManyFrames);
            }
            sys::fd_scratch_push();
        }

        Ok(ScratchFrame::new())
    }

    /// Pop a frame manually
    ///
    /// not recommended! - use `push_frame()`
    pub fn pop_frame(&mut self) -> Result<(), ScratchError> {
        unsafe {
            if sys::fd_scratch_pop_is_safe() == 0 {
                return Err(ScratchError::NoFramesToPop);
            }
            sys::fd_scratch_pop();
        }
        Ok(())
    }

    /// Allocate in the current frame
    pub fn allocate(
        &mut self,
        size: usize,
        alignment: usize,
    ) -> Result<ScratchAllocation, ScratchError> {
        if size == 0 {
            return Err(ScratchError::InvalidSize);
        }

        if !alignment.is_power_of_two() {
            return Err(ScratchError::InvalidAlignment);
        }

        unsafe {
            if sys::fd_scratch_alloc_is_safe(alignment as ulong, size as ulong) == 0 {
                return Err(ScratchError::InsufficientSpace);
            }

            let ptr = sys::fd_scratch_alloc(alignment as ulong, size as ulong);
            if ptr.is_null() {
                return Err(ScratchError::AllocationFailed);
            }

            Ok(ScratchAllocation::new(
                NonNull::new_unchecked(ptr as *mut u8),
                size,
            ))
        }
    }

    /// Prep a dynamic allocation with unknown final size
    pub fn prepare_alloc(&mut self, alignment: usize) -> Result<DynamicAllocation, ScratchError> {
        if !alignment.is_power_of_two() {
            return Err(ScratchError::InvalidAlignment);
        }

        unsafe {
            if sys::fd_scratch_prepare_is_safe(alignment as ulong) == 0 {
                return Err(ScratchError::InsufficientSpace);
            }

            let ptr = sys::fd_scratch_prepare(alignment as ulong);
            if ptr.is_null() {
                return Err(ScratchError::AllocationFailed);
            }

            Ok(DynamicAllocation {
                start_ptr: NonNull::new_unchecked(ptr as *mut u8),
                current_size: 0,
                max_available: self.free_bytes(),
            })
        }
    }

    /// number of bytes currently used
    pub fn used_bytes(&self) -> usize {
        unsafe { sys::fd_scratch_used() as usize }
    }

    /// number of frames currently in use
    pub fn frames_used(&self) -> usize {
        unsafe { sys::fd_scratch_frame_used() as usize }
    }

    /// number of bytes currently free
    pub fn free_bytes(&self) -> usize {
        unsafe { sys::fd_scratch_free() as usize }
    }

    /// number of frames currently free
    pub fn frames_free(&self) -> usize {
        unsafe { sys::fd_scratch_frame_free() as usize }
    }

    /// virtual allocator for this scratch instance
    pub fn as_valloc(&self) -> sys::fd_valloc_t {
        unsafe { sys::fd_scratch_virtual() }
    }
}

impl Drop for ScratchAllocator {
    fn drop(&mut self) {
        unsafe {
            if sys::fd_scratch_detach_is_safe() != 0 {
                sys::fd_scratch_detach(ptr::null_mut());
            }
        }

        unsafe {
            alloc::dealloc(self.smem.as_ptr(), self.smem_layout);
            alloc::dealloc(self.fmem.as_ptr() as *mut u8, self.fmem_layout);
        }
    }
}

/// Allocation that can be resized before being finalized
pub struct DynamicAllocation {
    start_ptr: NonNull<u8>,
    current_size: usize,
    max_available: usize,
}

impl DynamicAllocation {
    pub fn as_ptr(&self) -> *mut u8 {
        self.start_ptr.as_ptr()
    }

    pub fn current_size(&self) -> usize {
        self.current_size
    }

    pub fn max_available(&self) -> usize {
        self.max_available
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.start_ptr.as_ptr(), self.max_available) }
    }

    /// Finalize with the specified size
    pub fn publish(self, final_size: usize) -> Result<ScratchAllocation, ScratchError> {
        if final_size > self.max_available {
            return Err(ScratchError::InvalidSize);
        }

        let end_ptr = unsafe { self.start_ptr.as_ptr().add(final_size) };

        unsafe {
            if sys::fd_scratch_publish_is_safe(end_ptr as *mut core::ffi::c_void) == 0 {
                return Err(ScratchError::PrepareMismatch);
            }
            sys::fd_scratch_publish(end_ptr as *mut core::ffi::c_void);
        }

        Ok(ScratchAllocation::new(self.start_ptr, final_size))
    }

    pub fn cancel(self) {
        unsafe {
            sys::fd_scratch_cancel();
        }
    }
}

#[macro_export]
macro_rules! scratch_scope {
    ($allocator:expr, $block:block) => {{
        let _frame = $allocator.push_frame().unwrap();
        $block
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_scratch_state() {
        unsafe {
            if sys::fd_scratch_detach_is_safe() != 0 {
                sys::fd_scratch_detach(ptr::null_mut());
            }
        }
    }

    #[test]
    fn test_scratch_alloc() {
        reset_scratch_state();
        let allocator = ScratchAllocator::new(8192, 16).unwrap();
        assert_eq!(allocator.used_bytes(), 0);
        assert_eq!(allocator.frames_used(), 0);
        assert_eq!(allocator.frames_free(), 16);
    }

    #[test]
    fn test_basic_alloc() {
        reset_scratch_state();
        let mut allocator = ScratchAllocator::new(8192, 16).unwrap();
        {
            let _frame = allocator.push_frame().unwrap();
            let allocation = allocator.allocate(1024, 64).unwrap();
            assert_eq!(allocation.size(), 1024);
            assert_eq!(allocation.as_ptr() as usize % 64, 0);
            assert!(allocator.used_bytes() >= 1024);
            assert_eq!(allocator.frames_used(), 1);
        }
        assert_eq!(allocator.used_bytes(), 0);
        assert_eq!(allocator.frames_used(), 0);
    }

    #[test]
    fn test_multiple_allocs_in_frame() {
        reset_scratch_state();
        let mut allocator = ScratchAllocator::new(8192, 16).unwrap();
        {
            let _frame = allocator.push_frame().unwrap();
            let alloc1 = allocator.allocate(512, 32).unwrap();
            let alloc2 = allocator.allocate(256, 64).unwrap();
            let alloc3 = allocator.allocate(128, 16).unwrap();
            assert_eq!(alloc1.size(), 512);
            assert_eq!(alloc2.size(), 256);
            assert_eq!(alloc3.size(), 128);
            assert_eq!(alloc1.as_ptr() as usize % 32, 0);
            assert_eq!(alloc2.as_ptr() as usize % 64, 0);
            assert_eq!(alloc3.as_ptr() as usize % 16, 0);
            assert!(allocator.used_bytes() >= 512 + 256 + 128);
        }
        assert_eq!(allocator.used_bytes(), 0);
    }

    #[test]
    fn test_nested_frames() {
        reset_scratch_state();
        let mut allocator = ScratchAllocator::new(8192, 16).unwrap();
        {
            let _frame1 = allocator.push_frame().unwrap();
            let _alloc1 = allocator.allocate(1024, 64).unwrap();
            assert_eq!(allocator.frames_used(), 1);
            {
                let _frame2 = allocator.push_frame().unwrap();
                let _alloc2 = allocator.allocate(512, 32).unwrap();
                assert_eq!(allocator.frames_used(), 2);
                {
                    let _frame3 = allocator.push_frame().unwrap();
                    let _alloc3 = allocator.allocate(256, 16).unwrap();
                    assert_eq!(allocator.frames_used(), 3);
                }
                assert_eq!(allocator.frames_used(), 2);
            }
            assert_eq!(allocator.frames_used(), 1);
        }
        assert_eq!(allocator.frames_used(), 0);
        assert_eq!(allocator.used_bytes(), 0);
    }

    #[test]
    fn test_dynamic_alloc() {
        reset_scratch_state();
        let mut allocator = ScratchAllocator::new(8192, 16).unwrap();
        {
            let _frame = allocator.push_frame().unwrap();
            let mut dynamic = allocator.prepare_alloc(64).unwrap();
            assert_eq!(dynamic.as_ptr() as usize % 64, 0);
            let slice = dynamic.as_mut_slice();
            for i in 0..512 {
                slice[i] = (i % 256) as u8;
            }
            let allocation = dynamic.publish(512).unwrap();
            assert_eq!(allocation.size(), 512);
            let slice = allocation.as_slice();
            for i in 0..512 {
                assert_eq!(slice[i], (i % 256) as u8);
            }
        }
        assert_eq!(allocator.used_bytes(), 0);
    }

    #[test]
    fn test_alloc_cancel() {
        reset_scratch_state();
        let mut allocator = ScratchAllocator::new(8192, 16).unwrap();
        {
            let _frame = allocator.push_frame().unwrap();
            let dynamic = allocator.prepare_alloc(32).unwrap();
            let used_before_cancel = allocator.used_bytes();
            dynamic.cancel();

            assert!(allocator.used_bytes() <= used_before_cancel);
        }
        assert_eq!(allocator.used_bytes(), 0);
    }

    #[test]
    fn test_alloc_trim() {
        reset_scratch_state();
        let mut allocator = ScratchAllocator::new(8192, 16).unwrap();
        {
            let _frame = allocator.push_frame().unwrap();
            let mut allocation = allocator.allocate(1024, 64).unwrap();
            assert_eq!(allocation.size(), 1024);

            let used_before_trim = allocator.used_bytes();
            allocation.trim(512).unwrap();
            assert_eq!(allocation.size(), 512);

            let used_after_trim = allocator.used_bytes();
            assert!(used_after_trim <= used_before_trim);
        }
        assert_eq!(allocator.used_bytes(), 0);
    }

    #[test]
    fn test_error_conditions() {
        reset_scratch_state();
        assert!(matches!(
            ScratchAllocator::new(0, 16),
            Err(ScratchError::InvalidSize)
        ));
        assert!(matches!(
            ScratchAllocator::new(8192, 0),
            Err(ScratchError::InvalidSize)
        ));

        let mut allocator = ScratchAllocator::new(8192, 16).unwrap();
        {
            let _frame = allocator.push_frame().unwrap();
            assert!(matches!(
                allocator.allocate(1024, 63),
                Err(ScratchError::InvalidAlignment)
            ));
            assert!(matches!(
                allocator.allocate(0, 64),
                Err(ScratchError::InvalidSize)
            ));
        }
    }

    #[test]
    fn test_memory_slice() {
        reset_scratch_state();
        let mut allocator = ScratchAllocator::new(8192, 16).unwrap();
        {
            let _frame = allocator.push_frame().unwrap();
            let mut allocation = allocator.allocate(1024, 64).unwrap();

            let slice = allocation.as_mut_slice();
            assert_eq!(slice.len(), 1024);

            for i in 0..1024 {
                slice[i] = (i % 256) as u8;
            }

            let slice = allocation.as_slice();
            for i in 0..1024 {
                assert_eq!(slice[i], (i % 256) as u8);
            }
        }
    }

    #[test]
    fn test_valloc() {
        reset_scratch_state();
        let allocator = ScratchAllocator::new(8192, 16).unwrap();
        let valloc = allocator.as_valloc();
        unsafe {
            assert!(!valloc.vt.is_null());
            let vtable = &*valloc.vt;
            assert!(vtable.malloc.is_some());
            assert!(vtable.free.is_some());
        }
    }

    #[test]
    fn test_scope() {
        reset_scratch_state();
        let mut allocator = ScratchAllocator::new(8192, 16).unwrap();
        let result: Result<(), ScratchError> = scratch_scope!(allocator, {
            let _alloc1 = allocator.allocate(1024, 64).unwrap();
            let _alloc2 = allocator.allocate(512, 32).unwrap();
            assert!(allocator.used_bytes() >= 1536);
            Ok(())
        });

        assert!(result.is_ok());
        assert_eq!(allocator.used_bytes(), 0);
    }
}
