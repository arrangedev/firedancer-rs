//! Raw bindings example for fd_tile_sys
//!
//! This example demonstrates direct usage of the unsafe FFI bindings.
//! For safe usage, prefer the fd_tile crate instead.

use fd_tile_sys::*;
use std::ffi::CString;
use std::ptr;

unsafe extern "C" fn example_task(argc: i32, argv: *mut *mut i8) -> i32 {
    println!("- executing with {}", argc);

    for i in 0..argc {
        let arg_ptr = *argv.offset(i as isize);
        if !arg_ptr.is_null() {
            let c_str = std::ffi::CStr::from_ptr(arg_ptr);
            if let Ok(s) = c_str.to_str() {
                println!("  arg[{i}]: {s}");
            }
        }
    }

    println!("√ completed on tile {}", fd_tile_idx());
    42
}

fn main() {
    unsafe {
        let tile_id0 = fd_tile_id0();
        let tile_id1 = fd_tile_id1();
        let tile_id = fd_tile_id();
        let tile_idx = fd_tile_idx();
        let tile_cnt = fd_tile_cnt();
        println!("  thread-group-range: [{tile_id0}, {tile_id1})");
        println!("  current-tile-id: {tile_id}");
        println!("  current-tile-idx: {tile_idx}");
        println!("  total-tile-cnt: {tile_cnt}");
        println!();

        if tile_cnt > 0 {
            println!("cpu-affinity:");
            for i in 0..tile_cnt {
                let cpu_id = fd_tile_cpu_id(i);
                let status = if cpu_id == u64::MAX {
                    "invalid".to_string()
                } else if cpu_id == u64::MAX - 1 {
                    "floating".to_string()
                } else {
                    format!("CPU {cpu_id}")
                };
                println!("  Tile {i}: {status}");
            }
            println!();
        }

        println!("stack-info:");
        let stack0 = fd_tile_stack0();
        let stack1 = fd_tile_stack1();
        let stack_sz = fd_tile_stack_sz();
        let stack_used = fd_tile_stack_est_used();
        let stack_free = fd_tile_stack_est_free();

        println!("  stack-range: {:p} - {:p}", stack0, stack1);
        println!("  stack-size: {stack_sz} bytes");
        println!("  est-used: {stack_used} bytes");
        println!("  est-free: {stack_free} bytes");
        println!("  FD_TILE_MAX: {FD_TILE_MAX}");
        println!("  FD_TILE_PRIVATE_STACK_SZ: {FD_TILE_PRIVATE_STACK_SZ}");

        if tile_cnt > 1 && tile_idx < tile_cnt - 1 {
            let arg0 = CString::new("example_task").unwrap();
            let arg1 = CString::new("test_argument").unwrap();
            let mut argv = vec![
                arg0.as_ptr() as *mut i8,
                arg1.as_ptr() as *mut i8,
                ptr::null_mut(),
            ];

            let target_tile = tile_idx + 1;
            let exec = fd_tile_exec_new(
                target_tile,
                Some(example_task),
                2, // argc
                argv.as_mut_ptr(),
            );

            if !exec.is_null() {
                println!("√ dispatched to tile {target_tile}");
                let exec_id = fd_tile_exec_id(exec);
                let exec_idx = fd_tile_exec_idx(exec);
                let exec_argc = fd_tile_exec_argc(exec);

                println!("  exec-id: {exec_id}");
                println!("  exec-idx: {exec_idx}");
                println!("  exec-argc: {exec_argc}");

                let mut return_code = 0;
                let error_msg = fd_tile_exec_delete(exec, &mut return_code);

                if error_msg.is_null() {
                    println!("√ completed with return code: {return_code}");
                } else {
                    let error_str = std::ffi::CStr::from_ptr(error_msg);
                    println!("× failed: {error_str:?}");
                }
            } else {
                println!("× dispatch failed");
            }
        } else {
            if tile_cnt <= 1 {
                println!("  - {tile_cnt} tile(s) available");
            }
            if tile_idx >= tile_cnt - 1 {
                println!("  - {tile_idx} is the last tile");
            }
        }
    }
}
