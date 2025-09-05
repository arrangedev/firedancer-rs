//! example requires a properly initialized multi-tile environment.
//! may not work in single-threaded contexts or without proper setup

use fd_tile::{TaskResult, Tile};
use std::time::{Duration, Instant};

extern "C" fn c_fibonacci(argc: i32, argv: *mut *mut std::os::raw::c_char) -> i32 {
    println!("fibonacci started on tile {}", fd_tile::current_tile_idx());

    let n = if argc > 1 {
        unsafe {
            let arg_ptr = *argv.offset(1);
            if !arg_ptr.is_null() {
                let arg_cstr = std::ffi::CStr::from_ptr(arg_ptr as *const std::os::raw::c_char);
                if let Ok(arg_str) = arg_cstr.to_str() {
                    arg_str.parse::<u32>().unwrap_or(10)
                } else {
                    10
                }
            } else {
                10
            }
        }
    } else {
        10
    };

    let result = fibonacci(n);
    println!(
        "fibonacci({n}) = {result} (on tile {})",
        fd_tile::current_tile_idx()
    );

    result as i32
}

extern "C" fn worksim(argc: i32, argv: *mut *mut std::os::raw::c_char) -> i32 {
    let tile_idx = fd_tile::current_tile_idx();
    println!("worksim started on tile {}", tile_idx);

    let duration_ms = if argc > 1 {
        unsafe {
            let arg_ptr = *argv.offset(1);
            if !arg_ptr.is_null() {
                let arg_cstr = std::ffi::CStr::from_ptr(arg_ptr as *const std::os::raw::c_char);
                if let Ok(arg_str) = arg_cstr.to_str() {
                    arg_str.parse::<u64>().unwrap_or(100)
                } else {
                    100
                }
            } else {
                100
            }
        }
    } else {
        100
    };

    let start = Instant::now();
    std::thread::sleep(Duration::from_millis(duration_ms));
    let elapsed = start.elapsed();

    println!("worksim() = (on tile {tile_idx}, elapsed={elapsed:?})");
    0 // success!
}

fn main() {
    let tile_info = Tile::current_info();
    println!(
        "current-tile: {} of {} tiles",
        tile_info.idx, tile_info.count
    );

    if tile_info.count <= 1 {
        println!("× requires multiple tiles for dispatch");
        println!("current environment has {} tiles", tile_info.count);
        return;
    }

    dispatch(&tile_info);
    concurrency(&tile_info);
    with_args(&tile_info);
}

fn dispatch(tile_info: &fd_tile::TileInfo) {
    let target_tile = find_target_tile(tile_info);
    if target_tile.is_none() {
        println!("× no suitable target tile found for dispatch");
        return;
    }
    let target_tile = target_tile.unwrap();

    println!("dispatching fibonacci to tile {target_tile}");

    match Tile::execute_task(target_tile, c_fibonacci, &["25"]) {
        Ok(execution) => {
            println!("dispatched successfully, waiting for completion...");

            match execution.wait() {
                TaskResult::Success(code) => {
                    println!("√ completed with return code: {code}");
                }
                TaskResult::Error(msg) => {
                    println!("× failed: {msg}");
                }
            }
        }
        Err(e) => {
            println!("× failed to dispatch task: {:?}", e);
        }
    }
    println!();
}

fn concurrency(tile_info: &fd_tile::TileInfo) {
    let available_tiles = find_available_tiles(tile_info);
    if available_tiles.is_empty() {
        println!("× no available tiles for concurrent execution");
        return;
    }

    let num_tasks = std::cmp::min(available_tiles.len(), 3);
    println!("starting {num_tasks} concurrent tasks");

    let mut executions = Vec::new();

    for (i, &tile_idx) in available_tiles.iter().take(num_tasks).enumerate() {
        let work_duration = format!("{}", 200 + i * 100);

        match Tile::execute_task(tile_idx, worksim, &[&work_duration]) {
            Ok(execution) => {
                println!("started task {} on tile {tile_idx}", i + 1);
                executions.push(execution);
            }
            Err(e) => {
                println!("× failed to start task {} on tile {tile_idx}: {e:?}", i + 1,);
            }
        }
    }

    println!("waiting for all tasks to complete...");
    for (i, execution) in executions.into_iter().enumerate() {
        match execution.wait() {
            TaskResult::Success(code) => {
                println!("√ completed with code: {code}");
            }
            TaskResult::Error(msg) => {
                println!("× failed: {msg}");
            }
        }
    }
    println!();
}

fn with_args(tile_info: &fd_tile::TileInfo) {
    let target_tile = find_target_tile(tile_info);
    if target_tile.is_none() {
        println!("× no suitable target tile found for dispatch");
        return;
    }

    let target_tile = target_tile.unwrap();
    let fibonacci_numbers = ["15", "20", "30"];

    for fib_num in &fibonacci_numbers {
        println!("fibonacci({fib_num}) on tile {target_tile}");

        match Tile::execute_task(target_tile, c_fibonacci, &[fib_num]) {
            Ok(execution) => match execution.wait() {
                TaskResult::Success(result) => {
                    println!("√ result: {result}");
                }
                TaskResult::Error(msg) => {
                    println!("× error: {msg}");
                }
            },
            Err(e) => {
                println!("× dispatch failed: {e:?}");
            }
        }
    }
    println!();
}

fn find_target_tile(tile_info: &fd_tile::TileInfo) -> Option<u64> {
    for tile_idx in 1..tile_info.count {
        if tile_idx != tile_info.idx {
            return Some(tile_idx);
        }
    }
    None
}

fn find_available_tiles(tile_info: &fd_tile::TileInfo) -> Vec<u64> {
    let mut available = Vec::new();
    for tile_idx in 1..tile_info.count {
        if tile_idx != tile_info.idx {
            available.push(tile_idx);
        }
    }

    available
}

fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => {
            let mut a = 0;
            let mut b = 1;
            for _ in 2..=n {
                let next = a + b;
                a = b;
                b = next;
            }
            b
        }
    }
}
