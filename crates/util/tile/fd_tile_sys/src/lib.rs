//! Raw FFI bindings for `/util/tile`

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_info() {
        unsafe {
            // callable even without proper tile boot
            let id0 = fd_tile_id0();
            let id1 = fd_tile_id1();
            let id = fd_tile_id();
            let idx = fd_tile_idx();
            let cnt = fd_tile_cnt();

            assert!(id >= id0);
            assert!(id < id1 || (id0 == 0 && id1 == 0));
            assert!(idx < cnt || cnt == 0);
            assert_eq!(cnt, id1.saturating_sub(id0));
        }
    }

    #[test]
    fn test_cpu_id() {
        unsafe {
            let cpu_id = fd_tile_cpu_id(0);
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
