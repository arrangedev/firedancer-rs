//! Raw FFI bindings to Firedancer shared scratchpad (spad) allocator
//!
//! This crate provides low-level, unsafe bindings to the Firedancer spad utilities:
//! - High performance persistent inter-process shared scratch pad memory
//! - Frame-based memory management with push/pop semantics
//! - Alignment-aware allocation for shared memory environments
//! - Virtual allocator interface integration
//! - Prepare/publish/cancel allocation patterns for dynamic sizing
//! - Trim support for reducing allocation sizes
//!
//! The spad system provides shared scratch pad memory that behaves like a thread's stack
//! but can be used across multiple threads and processes when backed by shared memory.
//!
//! Key features:
//! - O(1) assembly operations for allocations and frame operations
//! - Frame-based grouping with automatic cleanup on frame pop
//! - Nested frame support
//! - Shared memory compatibility for inter-process communication
//! - Custom alignment support
//! - Dynamic allocation sizing with prepare/publish pattern
//!
//! For a safe Rust API, consider using the higher-level wrapper crate.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{alloc, dealloc, Layout};

    #[test]
    fn test_spad_constants() {
        assert_eq!(FD_SPAD_ALIGN, 128);
        assert_eq!(FD_SPAD_FRAME_MAX, 128);
        assert_eq!(FD_SPAD_ALLOC_ALIGN_DEFAULT, 16);
    }

    #[test]
    fn test_spad_footprint_calculations() {
        let mem_max = 4096usize;
        let footprint = unsafe { fd_spad_footprint(mem_max as ulong) };

        assert!(footprint > 0);
        assert!(footprint >= mem_max as ulong);

        let align = unsafe { fd_spad_align() };
        assert_eq!(align, FD_SPAD_ALIGN as ulong);

        let mem_max_max = unsafe { fd_spad_mem_max_max(footprint) };
        assert!(mem_max_max >= mem_max as ulong);
    }

    #[test]
    fn test_spad_basic_lifecycle() {
        let mem_max = 8192usize;
        let footprint = unsafe { fd_spad_footprint(mem_max as ulong) } as usize;

        let layout = Layout::from_size_align(footprint, FD_SPAD_ALIGN as usize).unwrap();
        let shmem = unsafe { alloc(layout) };
        assert!(!shmem.is_null());

        unsafe {
            let spad_ptr = fd_spad_new(shmem as *mut core::ffi::c_void, mem_max as ulong);
            assert!(!spad_ptr.is_null());

            let spad = fd_spad_join(spad_ptr);
            assert!(!spad.is_null());

            assert_eq!(fd_spad_frame_max(spad), FD_SPAD_FRAME_MAX as ulong);
            assert_eq!(fd_spad_frame_used(spad), 0);
            assert_eq!(fd_spad_frame_free(spad), FD_SPAD_FRAME_MAX as ulong);
            assert_eq!(fd_spad_mem_max(spad), mem_max as ulong);
            assert_eq!(fd_spad_mem_used(spad), 0);
            assert_eq!(fd_spad_mem_free(spad), mem_max as ulong);
            assert_eq!(fd_spad_in_frame(spad), 0);

            fd_spad_push(spad);
            assert_eq!(fd_spad_frame_used(spad), 1);
            assert_eq!(fd_spad_frame_free(spad), FD_SPAD_FRAME_MAX as ulong - 1);
            assert_eq!(fd_spad_in_frame(spad), 1);

            let alloc_ptr = fd_spad_alloc(spad, 64, 256);
            assert!(!alloc_ptr.is_null());
            assert_eq!(alloc_ptr as usize % 64, 0);

            let mem_used = fd_spad_mem_used(spad);
            assert!(mem_used >= 256);
            assert_eq!(fd_spad_mem_free(spad), mem_max as ulong - mem_used);

            let frame_lo = fd_spad_frame_lo(spad);
            let frame_hi = fd_spad_frame_hi(spad);
            assert!(!frame_lo.is_null());
            assert!(!frame_hi.is_null());
            assert!(frame_lo <= alloc_ptr);
            assert!(alloc_ptr < frame_hi);

            fd_spad_pop(spad);
            assert_eq!(fd_spad_frame_used(spad), 0);
            assert_eq!(fd_spad_mem_used(spad), 0);
            assert_eq!(fd_spad_in_frame(spad), 0);

            let returned_shmem = fd_spad_leave(spad);
            assert_eq!(returned_shmem, spad_ptr);

            let deleted_shmem = fd_spad_delete(spad_ptr);
            assert_eq!(deleted_shmem, spad_ptr);

            dealloc(shmem, layout);
        }
    }

    #[test]
    fn test_spad_prepare_publish_cancel() {
        let mem_max = 4096usize;
        let footprint = unsafe { fd_spad_footprint(mem_max as ulong) } as usize;

        let layout = Layout::from_size_align(footprint, FD_SPAD_ALIGN as usize).unwrap();
        let shmem = unsafe { alloc(layout) };

        unsafe {
            let spad_ptr = fd_spad_new(shmem as *mut core::ffi::c_void, mem_max as ulong);
            let spad = fd_spad_join(spad_ptr);

            fd_spad_push(spad);

            let max_size = 512usize;
            let prepare_ptr = fd_spad_prepare(spad, 32, max_size as ulong);
            assert!(!prepare_ptr.is_null());
            assert_eq!(prepare_ptr as usize % 32, 0);

            let actual_size = 256usize;
            fd_spad_publish(spad, actual_size as ulong);

            let mem_used = fd_spad_mem_used(spad);
            assert!(mem_used >= actual_size as ulong);

            let prepare_ptr2 = fd_spad_prepare(spad, 16, 128);
            assert!(!prepare_ptr2.is_null());

            fd_spad_cancel(spad);
            fd_spad_pop(spad);
            fd_spad_delete(fd_spad_leave(spad));

            dealloc(shmem, layout);
        }
    }

    #[test]
    fn test_spad_trim() {
        let mem_max = 2048usize;
        let footprint = unsafe { fd_spad_footprint(mem_max as ulong) } as usize;

        let layout = Layout::from_size_align(footprint, FD_SPAD_ALIGN as usize).unwrap();
        let shmem = unsafe { alloc(layout) };

        unsafe {
            let spad_ptr = fd_spad_new(shmem as *mut core::ffi::c_void, mem_max as ulong);
            let spad = fd_spad_join(spad_ptr);

            fd_spad_push(spad);

            let alloc_ptr = fd_spad_alloc(spad, 64, 1024);
            assert!(!alloc_ptr.is_null());

            let mem_used_before = fd_spad_mem_used(spad);
            assert!(mem_used_before >= 1024);

            let trim_ptr = (alloc_ptr as usize + 512) as *mut core::ffi::c_void;
            fd_spad_trim(spad, trim_ptr);

            let mem_used_after = fd_spad_mem_used(spad);
            assert!(mem_used_after <= mem_used_before);

            let frame_hi = fd_spad_frame_hi(spad);
            assert_eq!(frame_hi, trim_ptr);

            fd_spad_pop(spad);
            fd_spad_delete(fd_spad_leave(spad));

            dealloc(shmem, layout);
        }
    }

    #[test]
    fn test_spad_multiple_frames() {
        let mem_max = 8192usize;
        let footprint = unsafe { fd_spad_footprint(mem_max as ulong) } as usize;

        let layout = Layout::from_size_align(footprint, FD_SPAD_ALIGN as usize).unwrap();
        let shmem = unsafe { alloc(layout) };

        unsafe {
            let spad_ptr = fd_spad_new(shmem as *mut core::ffi::c_void, mem_max as ulong);
            let spad = fd_spad_join(spad_ptr);

            for i in 0..5 {
                fd_spad_push(spad);
                let ptr = fd_spad_alloc(spad, 32, 100 + i * 50);
                assert!(!ptr.is_null());
                assert_eq!(fd_spad_frame_used(spad), (i + 1) as ulong);
            }

            for i in (0..5).rev() {
                fd_spad_pop(spad);
                assert_eq!(fd_spad_frame_used(spad), i as ulong);
            }

            assert_eq!(fd_spad_mem_used(spad), 0);
            fd_spad_delete(fd_spad_leave(spad));

            dealloc(shmem, layout);
        }
    }

    #[test]
    fn test_spad_reset() {
        let mem_max = 4096usize;
        let footprint = unsafe { fd_spad_footprint(mem_max as ulong) } as usize;

        let layout = Layout::from_size_align(footprint, FD_SPAD_ALIGN as usize).unwrap();
        let shmem = unsafe { alloc(layout) };

        unsafe {
            let spad_ptr = fd_spad_new(shmem as *mut core::ffi::c_void, mem_max as ulong);
            let spad = fd_spad_join(spad_ptr);

            fd_spad_push(spad);
            fd_spad_alloc(spad, 64, 128);

            fd_spad_push(spad);
            fd_spad_alloc(spad, 32, 256);
            assert_eq!(fd_spad_frame_used(spad), 2);
            assert!(fd_spad_mem_used(spad) > 0);

            fd_spad_reset(spad);
            assert_eq!(fd_spad_frame_used(spad), 0);
            assert_eq!(fd_spad_mem_used(spad), 0);
            assert_eq!(fd_spad_mem_free(spad), mem_max as ulong);

            fd_spad_delete(fd_spad_leave(spad));

            dealloc(shmem, layout);
        }
    }

    #[test]
    fn test_spad_alloc_max() {
        let mem_max = 1024usize;
        let footprint = unsafe { fd_spad_footprint(mem_max as ulong) } as usize;

        let layout = Layout::from_size_align(footprint, FD_SPAD_ALIGN as usize).unwrap();
        let shmem = unsafe { alloc(layout) };

        unsafe {
            let spad_ptr = fd_spad_new(shmem as *mut core::ffi::c_void, mem_max as ulong);
            let spad = fd_spad_join(spad_ptr);

            fd_spad_push(spad);

            let max_16 = fd_spad_alloc_max(spad, 16);
            let max_64 = fd_spad_alloc_max(spad, 64);
            let max_default = fd_spad_alloc_max(spad, 0);

            assert!(max_16 <= mem_max as ulong);
            assert!(max_64 <= mem_max as ulong);
            assert!(max_default <= mem_max as ulong);
            assert!(max_64 <= max_16);

            fd_spad_pop(spad);
            fd_spad_delete(fd_spad_leave(spad));

            dealloc(shmem, layout);
        }
    }

    #[test]
    fn test_spad_virtual_allocator() {
        let mem_max = 2048usize;
        let footprint = unsafe { fd_spad_footprint(mem_max as ulong) } as usize;

        let layout = Layout::from_size_align(footprint, FD_SPAD_ALIGN as usize).unwrap();
        let shmem = unsafe { alloc(layout) };

        unsafe {
            let spad_ptr = fd_spad_new(shmem as *mut core::ffi::c_void, mem_max as ulong);
            let spad = fd_spad_join(spad_ptr);
            let valloc = fd_spad_virtual(spad);

            assert!(!valloc.vt.is_null());
            assert_eq!(valloc.self_, spad as *mut core::ffi::c_void);

            let vtable = &*valloc.vt;
            assert!(vtable.malloc.is_some());
            assert!(vtable.free.is_some());

            fd_spad_delete(fd_spad_leave(spad));

            dealloc(shmem, layout);
        }
    }
}
