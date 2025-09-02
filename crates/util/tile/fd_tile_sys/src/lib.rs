//! Low-level FFI bindings to Firedancer's fd_tile module.
//!
//! This crate provides raw, unsafe bindings to the Firedancer tile (task dispatch) API.
//! For safe, idiomatic Rust wrappers, see the `fd_tile` crate.
//!
//! # Overview
//!
//! The fd_tile system provides fast dispatching of tasks within a thread group.
//! It allows parallel execution of tasks across multiple tiles (worker threads)
//! with proper synchronization and communication.
//!
//! # Safety
//!
//! All functions in this crate are unsafe and require careful handling of:
//! - Memory management and lifetime guarantees
//! - Thread safety and concurrency
//! - Proper initialization and cleanup
//! - Task execution and synchronization
//!
//! # Example
//!
//! ```rust,no_run
//! use fd_tile_sys::*;
//! use std::ffi::CString;
//! use std::ptr;
//!
//! unsafe extern "C" fn sample_task(argc: i32, argv: *mut *mut i8) -> i32 {
//!     println!("Task executed with {} arguments", argc);
//!     0 // success code
//! }
//!
//! unsafe {
//!     // get tile info
//!     let tile_cnt = fd_tile_cnt();
//!     let tile_idx = fd_tile_idx();
//!     let tile_id = fd_tile_id();
//!     
//!     if tile_cnt > 1 && tile_idx < tile_cnt - 1 {
//!         // prep args
//!         let task_name = CString::new("sample_task").unwrap();
//!         let mut argv = vec![task_name.as_ptr() as *mut i8, ptr::null_mut()];
//!         
//!         // dispatch to another tile
//!         let exec = fd_tile_exec_new(
//!             tile_idx + 1,  // targe
//!             Some(sample_task),
//!             1,  // argc
//!             argv.as_mut_ptr()
//!         );
//!         
//!         if !exec.is_null() {
//!             // wait for complete
//!             let mut ret_code = 0;
//!             let error = fd_tile_exec_delete(exec, &mut ret_code);
//!             
//!             if error.is_null() {
//!                 println!("Task completed with return code: {}", ret_code);
//!             } else {
//!                 println!("Task failed");
//!             }
//!         }
//!     }
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(FD_TILE_MAX, 1024);
        assert_eq!(FD_TILE_PRIVATE_STACK_SZ, 8 * 1024 * 1024); // 8 MiB
    }

    #[test]
    fn test_tile_info() {
        unsafe {
            // callable even without proper tile boot (return defaults or 0s)
            let id0 = fd_tile_id0();
            let id1 = fd_tile_id1();
            let id = fd_tile_id();
            let idx = fd_tile_idx();
            let cnt = fd_tile_cnt();

            assert!(id >= id0);
            assert!(id < id1 || (id0 == 0 && id1 == 0)); // uninitialized case
            assert!(idx < cnt || cnt == 0); // uninitialized case
            assert_eq!(cnt, id1.saturating_sub(id0));
        }
    }

    #[test]
    fn test_cpu_id() {
        unsafe {
            let cpu_id = fd_tile_cpu_id(0);
            // should return either a valid CPU ID or special values
            // ULONG_MAX for invalid tile_idx, ULONG_MAX-1 for floating
            assert!(cpu_id == u64::MAX || cpu_id == u64::MAX - 1 || cpu_id < 1024);
        }
    }

    #[test]
    fn test_stack() {
        unsafe {
            let stack0 = fd_tile_stack0();
            let stack1 = fd_tile_stack1();
            let stack_sz = fd_tile_stack_sz();

            if !stack0.is_null() && !stack1.is_null() {
                assert!(stack1 as usize > stack0 as usize);
                assert_eq!(stack_sz, ((stack1 as usize) - (stack0 as usize)) as u64);
            } else {
                assert_eq!(stack_sz, 0);
            }

            let _used = fd_tile_stack_est_used();
            let _free = fd_tile_stack_est_free();
        }
    }

    #[test]
    fn test_function_type() {
        unsafe extern "C" fn dummy_task(_argc: i32, _argv: *mut *mut i8) -> i32 {
            42
        }

        let _task: fd_tile_task_t = Some(dummy_task);
        unsafe {
            if let Some(task_fn) = _task {
                let result = task_fn(0, std::ptr::null_mut());
                assert_eq!(result, 42);
            }
        }
    }

    #[test]
    fn test_exec_new_invalid() {
        unsafe {
            let result = fd_tile_exec_new(
                999999,               // invalid tile idx
                None,                 // NULL task
                0,                    // argc
                std::ptr::null_mut(), // argv
            );

            assert!(result.is_null());
        }
    }
}
