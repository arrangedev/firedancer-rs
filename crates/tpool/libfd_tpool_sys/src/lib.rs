//! Raw FFI bindings to Firedancer thread pool utilities
//!
//! This crate provides low-level, unsafe bindings to the Firedancer thread pool utilities:
//! - Thread pool creation and management
//! - High-performance task execution and scheduling
//! - Worker thread management and synchronization
//! - Map-reduce style parallel computation primitives
//! - Ultra-low overhead thread parallelism
//!
//! For a safe Rust API, consider using the higher-level wrapper crate.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tpool_align_footprint() {
        unsafe {
            let align = fd_tpool_align();
            assert!(align > 0);
            assert!(align.is_power_of_two());

            let footprint = fd_tpool_footprint(1);
            assert!(footprint > 0);
            assert_eq!(footprint % align, 0);

            let invalid_footprint = fd_tpool_footprint(0);
            assert_eq!(invalid_footprint, 0);
        }
    }

    #[test]
    fn test_tpool_constants() {
        assert!(FD_TPOOL_ALIGN > 0);
        assert!(FD_TPOOL_TASK_ARG_MAX > 0);
        assert_eq!(FD_TPOOL_OPT_SLEEP, 1);
    }

    #[test]
    fn test_tpool_init_fini() {
        unsafe {
            let worker_max = 1;
            let align = fd_tpool_align();
            let footprint = fd_tpool_footprint(worker_max);

            if footprint == 0 {
                return;
            }

            let layout =
                std::alloc::Layout::from_size_align(footprint as usize, align as usize).unwrap();
            let mem = std::alloc::alloc_zeroed(layout);
            if mem.is_null() {
                panic!("Failed to allocate memory");
            }

            let tpool = fd_tpool_init(mem as *mut _, worker_max, 0);

            if tpool.is_null() {
                panic!("Failed to initialize tpool");
            }

            let returned_mem = fd_tpool_fini(tpool);
            if returned_mem != mem as *mut _ {
                panic!("Failed to finalize tpool");
            }

            std::alloc::dealloc(mem, layout);
        }
    }

    #[test]
    fn test_bindings_exist() {
        unsafe {
            let _align = fd_tpool_align();
            let _footprint = fd_tpool_footprint(1);
            let _opt_sleep = FD_TPOOL_OPT_SLEEP;
            let _align_const = FD_TPOOL_ALIGN;
            let _task_arg_max = FD_TPOOL_TASK_ARG_MAX;
        }
    }
}
