//! Safe API for Firedancer thread pool utilities
//!
//! This wraps the FFI bindings provided by `libfd_tpool_sys` and provides
//! safer abstractions for their use.
//!
//! ## Structure
//!
//! - `pool`: Thread pool creation and management
//! - `task`: Task execution and scheduling utilities
//! - `partition`: Work partitioning utilities
//!
//! ## TODO -- Missing Deps
//!
//! - **`fd_tile`**: Core tile (lightweight thread) management and execution
//!   - `fd_tile_exec_new()`, `fd_tile_exec_delete()` - thread lifecycle
//!   - `fd_tile_idx()`, `fd_tile_cnt()` - thread identification
//!   - Located at: `vendor/util/tile/`
//! - **`fd_scratch`**: High-performance scratch memory allocation
//!   - `fd_scratch_push()`, `fd_scratch_pop()`, `fd_scratch_alloc()`
//!   - Used for temporary allocations during map-reduce operations
//!   - Located at: `vendor/util/scratch/`
//! - **`fd_util`**: Core utility functions and memory operations
//!   - `fd_memset()`, `fd_ulong_*()` functions, alignment utilities
//!   - Located at: `vendor/util/fd_util.h`, `vendor/util/fd_util_base.h`
//! - **`fd_shmem`**: Shared memory management for multi-process thread pools
//!   - Located at: `vendor/util/shmem/`
//! - **`fd_valloc`**: Virtual memory allocator (used by scratch)
//!   - Located at: `vendor/util/valloc/`
//! - **`fd_wksp`**: Workspace memory management
//!   - Located at: `vendor/util/wksp/`

use std::marker::PhantomData;

pub mod pool {
    use super::*;
    use std::alloc::{alloc_zeroed, dealloc, Layout};

    #[derive(Clone, Copy, Debug)]
    pub struct Options {
        /// Sleep mode for idle workers (higher latency but saves CPU resources)
        pub sleep: bool,
    }

    impl Default for Options {
        fn default() -> Self {
            Self { sleep: false }
        }
    }

    impl From<Options> for u64 {
        fn from(opts: Options) -> Self {
            let mut flags = 0u64;
            if opts.sleep {
                flags |= libfd_tpool_sys::FD_TPOOL_OPT_SLEEP as u64;
            }
            flags
        }
    }

    pub struct ThreadPool {
        tpool: *mut libfd_tpool_sys::fd_tpool_t,
        mem: *mut u8,
        layout: Layout,
        worker_max: usize,
        _marker: PhantomData<*mut libfd_tpool_sys::fd_tpool_t>,
    }

    unsafe impl Send for ThreadPool {}
    unsafe impl Sync for ThreadPool {}

    impl ThreadPool {
        /// Create a new thread pool with the specified maximum number of workers
        /// - `worker_max` - Maximum number of worker threads (must be >= 1)
        /// - `options` - Thread pool configuration options
        pub fn new(worker_max: usize, options: Options) -> Result<Self, &'static str> {
            if worker_max == 0 {
                return Err("worker_max must be at least 1");
            }

            unsafe {
                let align = libfd_tpool_sys::fd_tpool_align() as usize;
                let footprint = libfd_tpool_sys::fd_tpool_footprint(worker_max as u64) as usize;

                if footprint == 0 {
                    return Err("invalid worker_max");
                }

                let layout =
                    Layout::from_size_align(footprint, align).map_err(|_| "invalid layout")?;

                let mem = alloc_zeroed(layout);
                if mem.is_null() {
                    return Err("memory allocation failed");
                }

                let tpool = libfd_tpool_sys::fd_tpool_init(
                    mem as *mut _,
                    worker_max as u64,
                    options.into(),
                );

                if tpool.is_null() {
                    dealloc(mem, layout);
                    return Err("thread pool initialization failed");
                }

                Ok(ThreadPool {
                    tpool,
                    mem,
                    layout,
                    worker_max,
                    _marker: PhantomData,
                })
            }
        }

        pub fn worker_count(&self) -> usize {
            unsafe { (*self.tpool).worker_cnt as usize }
        }

        pub fn worker_max(&self) -> usize {
            self.worker_max
        }

        pub fn options(&self) -> Options {
            unsafe {
                let opt = (*self.tpool).opt;
                Options {
                    sleep: (opt & libfd_tpool_sys::FD_TPOOL_OPT_SLEEP as u64) != 0,
                }
            }
        }

        /// # Safety
        /// The caller must ensure that the returned pointer is not used after the
        /// ThreadPool is dropped, and that any operations on it are thread-safe.
        pub unsafe fn as_raw(&self) -> *mut libfd_tpool_sys::fd_tpool_t {
            self.tpool
        }
    }

    impl Drop for ThreadPool {
        fn drop(&mut self) {
            unsafe {
                let returned_mem = libfd_tpool_sys::fd_tpool_fini(self.tpool);
                if returned_mem != self.mem as *mut _ {
                    eprintln!("Warning: fd_tpool_fini returned unexpected memory pointer");
                }
                dealloc(self.mem, self.layout);
            }
        }
    }
}

pub mod task {
    use super::*;

    /// An executable task on a thread pool
    pub trait Task: Send + Sync {
        /// Execute the current task
        /// - `worker_idx` - Index of the worker executing
        /// - `worker_count` - Total worker threads available
        fn execute(&self, worker_idx: usize, worker_count: usize);
    }

    /// basic closure-based task
    pub struct ClosureTask<F>
    where
        F: Fn(usize, usize) + Send + Sync,
    {
        closure: F,
    }

    impl<F> ClosureTask<F>
    where
        F: Fn(usize, usize) + Send + Sync,
    {
        pub fn new(closure: F) -> Self {
            Self { closure }
        }
    }

    impl<F> Task for ClosureTask<F>
    where
        F: Fn(usize, usize) + Send + Sync,
    {
        fn execute(&self, worker_idx: usize, worker_count: usize) {
            (self.closure)(worker_idx, worker_count);
        }
    }

    /// **TODO**
    /// Execute a task on all workers in the thread pool
    ///
    /// - `tpool` - The thread pool to execute on
    /// - `task` - The task to execute
    pub fn execute_all<T: Task>(tpool: &pool::ThreadPool, task: &T) {
        let worker_count = tpool.worker_count();
        todo!()
    }
}

pub mod partition {
    /// Partition a range of work across multiple workers
    ///
    /// - `task_start` - Start of the task range
    /// - `task_end` - End of the task range (exclusive)
    /// - `lane_count` - Number of lanes per worker (usually 1)
    /// - `worker_idx` - Index of the current worker
    /// - `worker_count` - Total number of workers
    pub fn partition_range(
        task_start: usize,
        task_end: usize,
        lane_count: usize,
        worker_idx: usize,
        worker_count: usize,
    ) -> (usize, usize) {
        if worker_idx >= worker_count || task_start >= task_end || lane_count == 0 {
            return (task_start, task_start);
        }

        let task_count = task_end - task_start;
        let block_count = task_count / lane_count;
        let block_remainder = task_count % lane_count;

        let worker_block_min = block_count / worker_count;
        let worker_extra_count = block_count % worker_count;

        let extra_blocks = if worker_idx < worker_extra_count {
            1
        } else {
            0
        };
        let worker_blocks = worker_block_min + extra_blocks;

        let worker_start = task_start
            + lane_count * (worker_block_min * worker_idx + worker_idx.min(worker_extra_count));

        let mut worker_end = worker_start + lane_count * worker_blocks;

        if worker_idx == worker_count - 1 {
            worker_end += block_remainder;
        }

        (worker_start, worker_end.min(task_end))
    }
}

#[cfg(test)]
mod tests {
    use crate::pool::{Options, ThreadPool};
    use crate::task::ClosureTask;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;
    use task::Task;

    #[test]
    fn test_thread_pool_creation() {
        let options = Options::default();
        let tpool = ThreadPool::new(1, options);
        assert!(tpool.is_ok());

        let tpool = tpool.unwrap();
        assert_eq!(tpool.worker_count(), 1);
        assert_eq!(tpool.worker_max(), 1);
        assert!(!tpool.options().sleep);
    }

    #[test]
    fn test_thread_pool_with_sleep() {
        let options = Options { sleep: true };
        let tpool = ThreadPool::new(1, options);
        assert!(tpool.is_ok());

        let tpool = tpool.unwrap();
        assert!(tpool.options().sleep);
    }

    #[test]
    fn test_thread_pool_invalid_worker_max() {
        let options = Options::default();
        let result = ThreadPool::new(0, options);
        assert!(result.is_err());
    }

    #[test]
    fn test_closure_task() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let task = ClosureTask::new(move |_worker_idx, _worker_count| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        task.execute(0, 1);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_partition_range() {
        let (start, end) = partition::partition_range(0, 100, 1, 0, 4);
        assert_eq!(start, 0);
        assert_eq!(end, 25);

        let (start, end) = partition::partition_range(0, 100, 1, 1, 4);
        assert_eq!(start, 25);
        assert_eq!(end, 50);

        let (start, end) = partition::partition_range(0, 100, 1, 3, 4);
        assert_eq!(start, 75);
        assert_eq!(end, 100);
    }

    #[test]
    fn test_partition_range_with_remainder() {
        let (start, end) = partition::partition_range(0, 101, 1, 0, 4);
        assert_eq!(start, 0);
        assert_eq!(end, 25);

        let (start, end) = partition::partition_range(0, 101, 1, 3, 4);
        assert_eq!(start, 75);
        assert_eq!(end, 101);
    }

    #[test]
    fn test_partition_range_edge_cases() {
        let (start, end) = partition::partition_range(10, 10, 1, 0, 4);
        assert_eq!(start, 10);
        assert_eq!(end, 10);

        let (start, end) = partition::partition_range(0, 100, 1, 5, 4);
        assert_eq!(start, 0);
        assert_eq!(end, 0);

        let (start, end) = partition::partition_range(0, 100, 1, 0, 1);
        assert_eq!(start, 0);
        assert_eq!(end, 100);
    }

    #[test]
    fn test_task_execution() {
        let options = Options::default();
        let tpool = ThreadPool::new(1, options).unwrap();

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let task = ClosureTask::new(move |_worker_idx, _worker_count| {
            executed_clone.store(true, Ordering::SeqCst);
        });

        task::execute_all(&tpool, &task);
        assert!(executed.load(Ordering::SeqCst));
    }
}
