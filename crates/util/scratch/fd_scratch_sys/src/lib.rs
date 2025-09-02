//! Raw FFI bindings to Firedancer scratch pad memory allocator
//!
//! This crate provides low-level, unsafe bindings to the Firedancer scratch utilities:
//! - High performance scratch pad memory allocation
//! - Frame-based memory management with push/pop semantics
//! - Alignment-aware allocation similar to alloca
//! - Virtual allocator interface integration
//! - Prepare/publish/cancel allocation patterns for dynamic sizing
//!
//! The scratch system provides two main allocators:
//! 1. `fd_alloca` - stack-based allocator similar to alloca (if FD_HAS_ALLOCA)
//! 2. `fd_scratch_alloc` - frame-based allocator for complex temporary memory usage
//!
//! For a safe Rust API, consider using the higher-level wrapper crate.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use core::ptr;
    use std::alloc::{alloc, dealloc, Layout};

    #[test]
    fn test_scratch_footprint_calculations() {
        let smem_align = FD_SCRATCH_SMEM_ALIGN;
        let fmem_align = FD_SCRATCH_FMEM_ALIGN;

        assert_eq!(smem_align, 128);
        assert_eq!(fmem_align, 8);

        let smem_footprint = unsafe { fd_scratch_smem_footprint(4096) };
        let fmem_footprint = unsafe { fd_scratch_fmem_footprint(10) };

        assert!(smem_footprint >= 4096);
        assert_eq!(fmem_footprint, 10 * 8);
    }

    #[test]
    fn test_scratch_basic_lifecycle() {
        let smax = 8192usize;
        let depth = 16usize;

        let smem_layout = Layout::from_size_align(smax, 128).unwrap();
        let smem = unsafe { alloc(smem_layout) };
        assert!(!smem.is_null());

        let fmem_layout = Layout::from_size_align(depth * 8, 8).unwrap();
        let fmem = unsafe { alloc(fmem_layout) };
        assert!(!fmem.is_null());

        unsafe {
            assert_eq!(fd_scratch_attach_is_safe(), 1);
            assert_eq!(fd_scratch_detach_is_safe(), 0);

            fd_scratch_attach(
                smem as *mut core::ffi::c_void,
                fmem as *mut core::ffi::c_void,
                smax as ulong,
                depth as ulong,
            );

            assert_eq!(fd_scratch_attach_is_safe(), 0);
            assert_eq!(fd_scratch_detach_is_safe(), 1);
            assert_eq!(fd_scratch_reset_is_safe(), 1);

            assert_eq!(fd_scratch_used(), 0);
            assert_eq!(fd_scratch_free(), smax as ulong);
            assert_eq!(fd_scratch_frame_used(), 0);
            assert_eq!(fd_scratch_frame_free(), depth as ulong);

            assert_eq!(fd_scratch_push_is_safe(), 1);
            fd_scratch_push();
            assert_eq!(fd_scratch_frame_used(), 1);
            assert_eq!(fd_scratch_frame_free(), (depth - 1) as ulong);

            assert_eq!(fd_scratch_alloc_is_safe(64, 256), 1);
            let ptr1 = fd_scratch_alloc(64, 256);
            assert!(!ptr1.is_null());
            assert_eq!(ptr1 as usize % 64, 0);

            let used_after_alloc = fd_scratch_used();
            assert!(used_after_alloc >= 256);

            let ptr2 = fd_scratch_alloc(32, 128);
            assert!(!ptr2.is_null());
            assert_eq!(ptr2 as usize % 32, 0);

            let used_after_second_alloc = fd_scratch_used();
            assert!(used_after_second_alloc > used_after_alloc);

            assert_eq!(fd_scratch_pop_is_safe(), 1);
            fd_scratch_pop();
            assert_eq!(fd_scratch_frame_used(), 0);
            assert_eq!(fd_scratch_used(), 0); // all allocs freed

            // detach
            let mut opt_fmem: *mut core::ffi::c_void = ptr::null_mut();
            let returned_smem = fd_scratch_detach(&mut opt_fmem as *mut _);

            assert_eq!(returned_smem, smem as *mut core::ffi::c_void);
            assert_eq!(opt_fmem, fmem as *mut core::ffi::c_void);
            assert_eq!(fd_scratch_attach_is_safe(), 1);
            assert_eq!(fd_scratch_detach_is_safe(), 0);

            // Cleanup
            dealloc(smem, smem_layout);
            dealloc(fmem, fmem_layout);
        }
    }

    #[test]
    fn test_scratch_prepare_publish_cancel() {
        let smax = 4096usize;
        let depth = 8usize;

        let smem_layout = Layout::from_size_align(smax, 128).unwrap();
        let smem = unsafe { alloc(smem_layout) };
        let fmem_layout = Layout::from_size_align(depth * 8, 8).unwrap();
        let fmem = unsafe { alloc(fmem_layout) };

        unsafe {
            fd_scratch_attach(
                smem as *mut core::ffi::c_void,
                fmem as *mut core::ffi::c_void,
                smax as ulong,
                depth as ulong,
            );

            fd_scratch_push();

            assert_eq!(fd_scratch_prepare_is_safe(64), 1);
            let prepare_ptr = fd_scratch_prepare(64);
            assert!(!prepare_ptr.is_null());
            assert_eq!(prepare_ptr as usize % 64, 0);

            let end_ptr = (prepare_ptr as usize + 256) as *mut core::ffi::c_void;
            assert_eq!(fd_scratch_publish_is_safe(end_ptr), 1);
            fd_scratch_publish(end_ptr);

            assert!(fd_scratch_used() >= 256);

            let prepare_ptr2 = fd_scratch_prepare(32);
            assert!(!prepare_ptr2.is_null());

            assert_eq!(fd_scratch_cancel_is_safe(), 1);
            fd_scratch_cancel();

            let ptr3 = fd_scratch_alloc(64, 512);
            assert!(!ptr3.is_null());

            let trim_end = (ptr3 as usize + 256) as *mut core::ffi::c_void;
            assert_eq!(fd_scratch_trim_is_safe(trim_end), 1);
            fd_scratch_trim(trim_end);

            fd_scratch_pop();
            fd_scratch_detach(ptr::null_mut());

            dealloc(smem, smem_layout);
            dealloc(fmem, fmem_layout);
        }
    }

    #[test]
    fn test_scratch_reset() {
        let smax = 2048usize;
        let depth = 4usize;

        let smem_layout = Layout::from_size_align(smax, 128).unwrap();
        let smem = unsafe { alloc(smem_layout) };
        let fmem_layout = Layout::from_size_align(depth * 8, 8).unwrap();
        let fmem = unsafe { alloc(fmem_layout) };

        unsafe {
            fd_scratch_attach(
                smem as *mut core::ffi::c_void,
                fmem as *mut core::ffi::c_void,
                smax as ulong,
                depth as ulong,
            );

            fd_scratch_push();
            fd_scratch_alloc(64, 128);

            fd_scratch_push();
            fd_scratch_alloc(32, 256);

            assert_eq!(fd_scratch_frame_used(), 2);
            assert!(fd_scratch_used() > 0);

            assert_eq!(fd_scratch_reset_is_safe(), 1);
            fd_scratch_reset();

            assert_eq!(fd_scratch_frame_used(), 0);
            assert_eq!(fd_scratch_used(), 0);
            assert_eq!(fd_scratch_free(), smax as ulong);

            fd_scratch_detach(ptr::null_mut());

            dealloc(smem, smem_layout);
            dealloc(fmem, fmem_layout);
        }
    }

    #[test]
    fn test_scratch_virtual_allocator() {
        unsafe {
            let valloc = fd_scratch_virtual();
            assert!(!valloc.vt.is_null());
            assert_eq!(valloc.self_, ptr::null_mut());

            let vtable = &*valloc.vt;
            assert!(vtable.malloc.is_some());
            assert!(vtable.free.is_some());
        }
    }

    #[test]
    fn test_multiple_frames() {
        let smax = 8192usize;
        let depth = 10usize;

        let smem_layout = Layout::from_size_align(smax, 128).unwrap();
        let smem = unsafe { alloc(smem_layout) };
        let fmem_layout = Layout::from_size_align(depth * 8, 8).unwrap();
        let fmem = unsafe { alloc(fmem_layout) };

        unsafe {
            fd_scratch_attach(
                smem as *mut core::ffi::c_void,
                fmem as *mut core::ffi::c_void,
                smax as ulong,
                depth as ulong,
            );

            for i in 0..5 {
                fd_scratch_push();
                let ptr = fd_scratch_alloc(64, 100 + i * 50);
                assert!(!ptr.is_null());
                assert_eq!(fd_scratch_frame_used(), (i + 1) as ulong);
            }

            for i in (0..5).rev() {
                fd_scratch_pop();
                assert_eq!(fd_scratch_frame_used(), i as ulong);
            }

            assert_eq!(fd_scratch_used(), 0);

            fd_scratch_detach(ptr::null_mut());

            dealloc(smem, smem_layout);
            dealloc(fmem, fmem_layout);
        }
    }
}
