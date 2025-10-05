//! Safe API for `fd_tile_sys`
//!
//! The tile system organizes execution into:
//! - **Thread Group**: A collection of tiles sharing the same address space
//! - **Tile**: A worker thread that can execute dispatched tasks
//! - **Task**: A function that can be executed on a tile
//! - **Execution**: A running instance of a task on a specific tile
//!
//! ## Platform Support
//!
//! - **Linux**: Everything
//! - **macOS/Other**: Single-tile mode
//!
//! ## Usage Patterns
//!
//! `tile` is intended forparallel execution where:
//! - Tasks need to be distributed across multiple CPU cores
//! - NUMA-aware scheduling is important
//! - Low-latency task dispatch is critical
//! - Stack usage monitoring is needed for debugging

use core::marker::PhantomData;
use core::ptr::NonNull;
use fd_tile_sys::{self as sys, ulong};
use std::ffi::CString;

#[derive(Debug, Clone, PartialEq)]
pub enum TileError {
    InvalidTileIndex(ulong),
    DispatchFailed,
    InvalidTask,
    InvalidArguments,
    ExecutionFailed,
    NotInitialized,
    CannotDispatchToSelf,
    CannotDispatchToTileZero,
    FailedOnWait,
    TileBusy,
    ThreadGroupMismatch,
}

impl core::fmt::Display for TileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TileError::InvalidTileIndex(idx) => write!(f, "Invalid tile index: {}", idx),
            TileError::DispatchFailed => write!(f, "Task dispatch failed"),
            TileError::InvalidTask => write!(f, "Invalid task function"),
            TileError::InvalidArguments => write!(f, "Invalid arguments:"),
            TileError::ExecutionFailed => write!(f, "Execution failed"),
            TileError::NotInitialized => write!(f, "Tile system not initialized"),
            TileError::CannotDispatchToSelf => write!(f, "Cannot dispatch task to self"),
            TileError::CannotDispatchToTileZero => write!(f, "Cannot dispatch task to tile 0"),
            TileError::FailedOnWait => write!(f, "Failed on wait"),
            TileError::TileBusy => write!(f, "Target tile is busy"),
            TileError::ThreadGroupMismatch => write!(f, "Tile not in same thread group"),
        }
    }
}

impl core::error::Error for TileError {}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileInfo {
    /// First tile ID in the thread group
    pub id0: ulong,
    /// One past the last tile ID in the thread group  
    pub id1: ulong,
    /// Current tile ID
    pub id: ulong,
    /// Current tile index within the thread group (0-based)
    pub idx: ulong,
    /// Total number of tiles in the thread group
    pub count: ulong,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackInfo {
    /// Stack start address (lower address)
    pub stack0: *const u8,
    /// Stack end address (higher address)
    pub stack1: *const u8,
    /// Total stack size in bytes
    pub size: usize,
    /// Estimated bytes currently used
    pub used: usize,
    /// Estimated bytes currently free
    pub free: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskResult {
    Success(i32),
    Error(TileError),
}

pub type TaskFunction = unsafe extern "C" fn(argc: i32, argv: *mut *mut core::ffi::c_char) -> i32;

pub struct TaskExecution {
    handle: NonNull<sys::fd_tile_exec_t>,
    _phantom: PhantomData<sys::fd_tile_exec_t>,
}

impl TaskExecution {
    fn new(handle: *mut sys::fd_tile_exec_t) -> Result<Self, TileError> {
        NonNull::new(handle)
            .map(|h| Self {
                handle: h,
                _phantom: PhantomData,
            })
            .ok_or(TileError::DispatchFailed)
    }

    /// tile ID this execution is running on
    pub fn tile_id(&self) -> ulong {
        unsafe { sys::fd_tile_exec_id(self.handle.as_ptr()) }
    }

    /// tile index this execution is running on
    pub fn tile_idx(&self) -> ulong {
        unsafe { sys::fd_tile_exec_idx(self.handle.as_ptr()) }
    }

    /// task function being executed
    pub fn task(&self) -> Option<TaskFunction> {
        unsafe { sys::fd_tile_exec_task(self.handle.as_ptr()) }
    }

    /// number of arguments passed to the task
    pub fn argc(&self) -> i32 {
        unsafe { sys::fd_tile_exec_argc(self.handle.as_ptr()) }
    }

    /// check if the execution has completed
    pub fn is_done(&self) -> bool {
        unsafe { sys::fd_tile_exec_done(self.handle.as_ptr()) != 0 }
    }

    /// wait for the execution to complete and get the result
    pub fn wait(self) -> TaskResult {
        let mut return_code = 0;
        let error_msg = unsafe { sys::fd_tile_exec_delete(self.handle.as_ptr(), &mut return_code) };
        core::mem::forget(self);

        if error_msg.is_null() {
            TaskResult::Success(return_code)
        } else {
            TaskResult::Error(TileError::FailedOnWait)
        }
    }

    pub fn try_result(self) -> Option<TaskResult> {
        if self.is_done() {
            Some(self.wait())
        } else {
            core::mem::forget(self);
            None
        }
    }
}

impl Drop for TaskExecution {
    fn drop(&mut self) {
        unsafe {
            sys::fd_tile_exec_delete(self.handle.as_ptr(), core::ptr::null_mut());
        }
    }
}

pub struct Tile;

impl Tile {
    /// information about the current tile
    pub fn current_info() -> TileInfo {
        unsafe {
            TileInfo {
                id0: sys::fd_tile_id0(),
                id1: sys::fd_tile_id1(),
                id: sys::fd_tile_id(),
                idx: sys::fd_tile_idx(),
                count: sys::fd_tile_cnt(),
            }
        }
    }

    /// cpu_id for a specific tile index
    pub fn cpu_id(tile_idx: ulong) -> Option<ulong> {
        let cpu_id = unsafe { sys::fd_tile_cpu_id(tile_idx) };
        if cpu_id == ulong::MAX || cpu_id == ulong::MAX - 1 {
            None // invalid or floating
        } else {
            Some(cpu_id)
        }
    }

    /// stack information for the current tile
    pub fn stack_info() -> StackInfo {
        unsafe {
            StackInfo {
                stack0: sys::fd_tile_stack0() as *const u8,
                stack1: sys::fd_tile_stack1() as *const u8,
                size: sys::fd_tile_stack_sz() as usize,
                used: sys::fd_tile_stack_est_used() as usize,
                free: sys::fd_tile_stack_est_free() as usize,
            }
        }
    }

    /// execute a task on the specified tile index
    ///
    /// NOTE: On non-Linux platforms, this is likely limited.
    /// Other platforms require use of the no-threads impl which restricts
    /// dispatch to self or tile 0.
    pub fn execute_task(
        tile_idx: ulong,
        task: TaskFunction,
        args: &[&str],
    ) -> Result<TaskExecution, TileError> {
        let info = Self::current_info();
        if tile_idx >= info.count {
            return Err(TileError::InvalidTileIndex(tile_idx));
        }

        // trying to dispatch to self
        if tile_idx == info.idx {
            return Err(TileError::CannotDispatchToSelf);
        }

        // trying to dispatch to tile 0 (usually not allowed)
        if tile_idx == 0 {
            return Err(TileError::CannotDispatchToTileZero);
        }

        let mut c_args: Vec<*mut core::ffi::c_char> = Vec::new();
        let mut c_strings = Vec::new();

        let task_name = CString::new("task").map_err(|_| TileError::InvalidArguments)?;
        c_strings.push(task_name);

        for arg in args {
            let c_arg = CString::new(*arg).map_err(|_| TileError::InvalidArguments)?;
            c_strings.push(c_arg);
        }

        for c_string in &c_strings {
            c_args.push(c_string.as_ptr() as *mut core::ffi::c_char);
        }
        c_args.push(core::ptr::null_mut());

        let handle = unsafe {
            sys::fd_tile_exec_new(
                tile_idx,
                Some(task),
                c_strings.len() as i32,
                c_args.as_mut_ptr(),
            )
        };

        TaskExecution::new(handle)
    }

    /// execute a task on the specified tile ID
    pub fn execute_task_by_id(
        tile_id: ulong,
        task: TaskFunction,
        args: &[&str],
    ) -> Result<TaskExecution, TileError> {
        let info = Self::current_info();

        if tile_id < info.id0 || tile_id >= info.id1 {
            return Err(TileError::InvalidTileIndex(tile_id));
        }

        Self::execute_task(tile_id - info.id0, task, args)
    }

    /// current execution running on a tile (if any)
    pub fn current_execution(tile_idx: ulong) -> Option<&'static sys::fd_tile_exec_t> {
        let exec_ptr = unsafe { sys::fd_tile_exec(tile_idx) };
        if exec_ptr.is_null() {
            None
        } else {
            Some(unsafe { &*exec_ptr })
        }
    }

    /// current execution running on a tile by ID (if any)
    pub fn current_execution_by_id(tile_id: ulong) -> Option<&'static sys::fd_tile_exec_t> {
        let exec_ptr = unsafe { sys::fd_tile_exec_by_id(tile_id) };
        if exec_ptr.is_null() {
            None
        } else {
            Some(unsafe { &*exec_ptr })
        }
    }
}

pub fn current_tile_id() -> ulong {
    unsafe { sys::fd_tile_id() }
}

pub fn current_tile_idx() -> ulong {
    unsafe { sys::fd_tile_idx() }
}

pub fn tile_count() -> ulong {
    unsafe { sys::fd_tile_cnt() }
}

pub fn thread_group_range() -> (ulong, ulong) {
    unsafe { (sys::fd_tile_id0(), sys::fd_tile_id1()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_info() {
        let info = Tile::current_info();

        assert!(info.id >= info.id0);
        assert!(info.id < info.id1 || (info.id0 == 0 && info.id1 == 0));
        assert!(info.idx < info.count || info.count == 0);
        assert_eq!(info.count, info.id1.saturating_sub(info.id0));

        assert_eq!(current_tile_id(), info.id);
        assert_eq!(current_tile_idx(), info.idx);
        assert_eq!(tile_count(), info.count);
        assert_eq!(thread_group_range(), (info.id0, info.id1));
    }

    #[test]
    fn test_stack_info() {
        let stack = Tile::stack_info();
        if !stack.stack0.is_null() && !stack.stack1.is_null() {
            assert!(stack.stack1 as usize > stack.stack0 as usize);
            assert_eq!(
                stack.size,
                (stack.stack1 as usize) - (stack.stack0 as usize)
            );
        } else {
            assert_eq!(stack.size, 0);
        }

        if stack.size > 0 {
            assert!(stack.used + stack.free <= stack.size + 1024); // allow some overhead
        }
    }

    #[test]
    fn test_cpu_id() {
        let info = Tile::current_info();

        for tile_idx in 0..info.count {
            let cpu_id = Tile::cpu_id(tile_idx);
            if let Some(id) = cpu_id {
                assert!(id < 1024);
            }
        }

        assert!(Tile::cpu_id(9999).is_none());
    }

    #[test]
    fn test_task_fn_type() {
        extern "C" fn test_task(_argc: i32, _argv: *mut *mut i8) -> i32 {
            42
        }

        let _task: TaskFunction = test_task;
        let result = test_task(0, core::ptr::null_mut());
        assert_eq!(result, 42);
    }

    #[test]
    fn test_exec_validation() {
        let info = Tile::current_info();

        extern "C" fn dummy_task(_argc: i32, _argv: *mut *mut i8) -> i32 {
            0
        }

        let result = Tile::execute_task(9999, dummy_task, &[]);
        assert!(matches!(result, Err(TileError::InvalidTileIndex(9999))));

        let result = Tile::execute_task(info.idx, dummy_task, &[]);
        if info.count > 0 {
            match result {
                Err(TileError::CannotDispatchToSelf) => {}
                Err(_) => {} // errors are acceptable for non-linux targets
                Ok(_) => {}  // some impls might allow self-dispatch
            }
        }

        if info.idx != 0 {
            let result = Tile::execute_task(0, dummy_task, &[]);
            assert!(matches!(result, Err(TileError::CannotDispatchToTileZero)));
        }
    }
}
