//! Raw FFI bindings to Firedancer virtual memory allocator (valloc)
//!
//! This crate provides low-level, unsafe bindings to the Firedancer valloc utilities:
//! - Virtual allocator abstraction for different memory allocation strategies
//! - libc allocator implementation using aligned_alloc
//! - Backtracing allocator for debugging memory leaks (when FD_HAS_HOSTED is enabled)
//! - Null allocator for testing/special cases
//!
//! The valloc system provides a virtual table-based approach to memory allocation,
//! allowing different allocation strategies to be swapped at runtime.
//!
//! For a safe Rust API, consider using the higher-level wrapper crate.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use core::ptr;

pub unsafe fn fd_libc_alloc_virtual() -> fd_valloc_t {
    fd_valloc_t {
        self_: ptr::null_mut(),
        vt: unsafe { &fd_libc_vtable },
    }
}

pub unsafe fn fd_null_alloc_virtual() -> fd_valloc_t {
    fd_valloc_t {
        self_: ptr::null_mut(),
        vt: ptr::null(),
    }
}

pub unsafe fn fd_is_null_alloc_virtual(valloc: fd_valloc_t) -> i32 {
    if valloc.vt.is_null() {
        1
    } else {
        0
    }
}

pub unsafe fn fd_backtracing_alloc_virtual(inner_valloc: *mut fd_valloc_t) -> fd_valloc_t {
    fd_valloc_t {
        self_: inner_valloc as *mut core::ffi::c_void,
        vt: unsafe { &fd_backtracing_vtable },
    }
}

pub unsafe fn fd_valloc_malloc(
    valloc: fd_valloc_t,
    align: ulong,
    sz: ulong,
) -> *mut core::ffi::c_void {
    if let Some(malloc_fn) = valloc.vt.as_ref().and_then(|vt| vt.malloc) {
        malloc_fn(valloc.self_, align, sz)
    } else {
        ptr::null_mut()
    }
}

pub unsafe fn fd_valloc_free(valloc: fd_valloc_t, ptr: *mut core::ffi::c_void) {
    if let Some(free_fn) = valloc.vt.as_ref().and_then(|vt| vt.free) {
        free_fn(valloc.self_, ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_libc_valloc_basic() {
        let valloc = unsafe { fd_libc_alloc_virtual() };

        let size = 1024;
        let align = 64;
        let ptr = unsafe { fd_valloc_malloc(valloc, align as ulong, size as ulong) };

        assert!(!ptr.is_null());

        unsafe { fd_valloc_free(valloc, ptr) };
    }

    #[test]
    fn test_null_valloc() {
        let null_valloc = unsafe { fd_null_alloc_virtual() };
        assert_eq!(unsafe { fd_is_null_alloc_virtual(null_valloc) }, 1);

        let non_null_valloc = unsafe { fd_libc_alloc_virtual() };
        assert_eq!(unsafe { fd_is_null_alloc_virtual(non_null_valloc) }, 0);
    }

    #[test]
    fn test_valloc_vtable_exists() {
        let _libc_vtable = unsafe { &fd_libc_vtable };
        let _backtracing_vtable = unsafe { &fd_backtracing_vtable };
    }

    #[test]
    fn test_valloc_alignment() {
        let valloc = unsafe { fd_libc_alloc_virtual() };

        for align_exp in 3..=12 {
            // 8 bytes to 4KB
            let align = 1usize << align_exp;
            let size = align * 2;

            let ptr = unsafe { fd_valloc_malloc(valloc, align as ulong, size as ulong) };
            assert!(!ptr.is_null());

            let addr = ptr as usize;
            assert_eq!(addr % align, 0, "Pointer not properly aligned to {}", align);

            unsafe { fd_valloc_free(valloc, ptr) };
        }
    }

    #[test]
    fn test_multiple_allocations() {
        let valloc = unsafe { fd_libc_alloc_virtual() };
        let mut ptrs = Vec::new();

        for i in 1..=10 {
            let size = i * 64;
            let align = 64;
            let ptr = unsafe { fd_valloc_malloc(valloc, align as ulong, size as ulong) };
            assert!(!ptr.is_null());
            ptrs.push(ptr);
        }

        for ptr in ptrs {
            unsafe { fd_valloc_free(valloc, ptr) };
        }
    }
}
