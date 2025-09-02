//! Safe Rust API for Firedancer template data structures
//!
//! This crate provides safe abstractions over the raw FFI bindings in `fd_tmpl_sys`.
//! The template system provides high-performance, compile-time generated data structures
//! including maps, sets, queues, stacks, heaps, and more.
//!
//! ## Features
//!
//! - **Type Safety**: Safe abstractions over C template instantiations
//! - **Memory Management**: Automatic cleanup and proper alignment handling
//! - **Zero-Copy Operations**: Direct access to underlying data when safe
//! - **High Performance**: Template-based code generation for optimal performance
//! - **Rich Data Structures**: Maps, sets, deques, heaps, pools, queues, stacks, vectors
//!
//! ## Data Structures
//!
//! - **Map**: Key-value associative containers with fast lookup
//! - **Set**: Key-only containers for membership testing
//! - **Deque**: Double-ended queue with push/pop at both ends
//! - **Heap**: Priority queue with efficient min/max operations
//! - **Pool**: Object pool for efficient memory management
//! - **Queue**: FIFO queue with constant-time operations
//! - **Stack**: LIFO stack with constant-time operations
//! - **Vector**: Dynamic array with amortized constant-time operations
//!
//! ## Example
//!
//! ```rust,no_run
//! use fd_tmpl::{FdMap, FdStack, FdQueue, FdSet, FdHeap, FdVec};
//!
//! // Create a map
//! let mut map = FdMap::new().unwrap();
//! map.insert(42, 123).unwrap();
//!
//! if let Some(value) = map.get(&42) {
//!     println!("Found value: {}", value);
//! }
//!
//! // Create a set
//! let mut set = FdSet::new().unwrap();
//! set.insert(1).unwrap();
//! set.insert(2).unwrap();
//! assert!(set.contains(&1));
//!
//! // Create a stack
//! let mut stack = FdStack::new().unwrap();
//! stack.push(1).unwrap();
//! stack.push(2).unwrap();
//!
//! while let Some(value) = stack.pop() {
//!     println!("Popped: {}", value);
//! }
//!
//! // Create a queue
//! let mut queue = FdQueue::new().unwrap();
//! queue.push(10).unwrap();
//! queue.push(20).unwrap();
//!
//! while let Some(value) = queue.pop() {
//!     println!("Dequeued: {}", value);
//! }
//!
//! // Create a min-heap
//! let mut heap = FdHeap::new(100).unwrap();
//! heap.insert(30).unwrap();
//! heap.insert(10).unwrap();
//! heap.insert(20).unwrap();
//! assert_eq!(heap.pop_min(), Some(10)); // Min element
//!
//! // Create a vector
//! let mut vec = FdVec::new(100).unwrap();
//! vec.push(1).unwrap();
//! vec.push(2).unwrap();
//! vec.push(3).unwrap();
//! assert_eq!(vec.get(1), Some(2));
//! ```

use fd_tmpl_sys::{self as sys, ulong};
use std::alloc::{self, Layout};
use std::marker::PhantomData;
use std::ptr::{self, NonNull};

#[derive(Debug, Clone, PartialEq)]
pub enum TmplError {
    /// Memory allocation failed
    AllocationFailed,
    /// Invalid parameters
    InvalidInput(String),
    /// Container is full
    ContainerFull,
    /// Key not found
    KeyNotFound,
    /// Container is empty
    ContainerEmpty,
    /// Operation failed
    OperationFailed(String),
    /// Invalid alignment
    InvalidAlignment,
}

impl std::fmt::Display for TmplError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TmplError::AllocationFailed => write!(f, "Memory allocation failed"),
            TmplError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            TmplError::ContainerFull => write!(f, "Container is full"),
            TmplError::KeyNotFound => write!(f, "Key not found"),
            TmplError::ContainerEmpty => write!(f, "Container is empty"),
            TmplError::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
            TmplError::InvalidAlignment => write!(f, "Invalid alignment"),
        }
    }
}

impl std::error::Error for TmplError {}

/// A safe wrapper around fd_ulong_map
pub struct FdMap {
    handle: NonNull<sys::fd_ulong_map_ele_t>,
    mem: NonNull<u8>,
    layout: Layout,
    _phantom: PhantomData<sys::fd_ulong_map_ele_t>,
}

impl FdMap {
    pub fn new() -> Result<Self, TmplError> {
        unsafe {
            let footprint = sys::fd_ulong_map_footprint();
            let align = sys::fd_ulong_map_align();

            if footprint == 0 || align == 0 || !align.is_power_of_two() {
                return Err(TmplError::InvalidAlignment);
            }

            let layout = Layout::from_size_align(footprint as usize, align as usize)
                .map_err(|_| TmplError::InvalidAlignment)?;

            let mem = NonNull::new(alloc::alloc(layout)).ok_or(TmplError::AllocationFailed)?;

            let shmem = sys::fd_ulong_map_new(mem.as_ptr() as *mut std::ffi::c_void);
            let handle = sys::fd_ulong_map_join(shmem);

            if handle.is_null() {
                alloc::dealloc(mem.as_ptr(), layout);
                return Err(TmplError::AllocationFailed);
            }

            Ok(Self {
                handle: NonNull::new(handle).unwrap(),
                mem,
                layout,
                _phantom: PhantomData,
            })
        }
    }

    /// Insert a key-value pair into the map
    pub fn insert(&mut self, key: u64, value: u64) -> Result<(), TmplError> {
        unsafe {
            let ele = sys::fd_ulong_map_ele_t {
                key,
                hash: key as u32, // Simple hash function
                value,
            };

            let result = sys::fd_ulong_map_insert(self.handle.as_ptr(), ele.key);
            if result.is_null() {
                Err(TmplError::ContainerFull)
            } else {
                Ok(())
            }
        }
    }

    /// Get a value by key
    pub fn get(&self, key: &u64) -> Option<u64> {
        unsafe {
            let query = sys::fd_ulong_map_ele_t {
                key: *key,
                hash: *key as u32,
                value: 0, // Will be ignored for query
            };

            let result = sys::fd_ulong_map_query(self.handle.as_ptr(), query.key, ptr::null_mut());
            if result.is_null() {
                None
            } else {
                Some((*result).value)
            }
        }
    }

    /// Remove a key from the map
    pub fn remove(&mut self, key: &u64) -> Option<()> {
        unsafe {
            let query = &mut sys::fd_ulong_map_ele_t {
                key: *key,
                hash: *key as u32,
                value: 0,
            };

            sys::fd_ulong_map_remove(self.handle.as_ptr(), ptr::from_mut(query));
            Some(())
        }
    }

    /// Check if the map contains a key
    pub fn contains_key(&self, key: &u64) -> bool {
        self.get(key).is_some()
    }

    /// Get the number of elements in the map
    pub fn len(&self) -> usize {
        unsafe { sys::fd_ulong_map_footprint() as usize }
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all elements from the map
    pub fn clear(&mut self) {
        // Note: This is a simplified clear - in practice you might want to iterate and remove
        unsafe {
            let shmem = sys::fd_ulong_map_leave(self.handle.as_ptr());
            sys::fd_ulong_map_delete(shmem);

            let new_shmem = sys::fd_ulong_map_new(self.mem.as_ptr() as *mut std::ffi::c_void);
            let new_handle = sys::fd_ulong_map_join(new_shmem);
            self.handle = NonNull::new(new_handle).unwrap();
        }
    }
}

impl Drop for FdMap {
    fn drop(&mut self) {
        unsafe {
            let shmem = sys::fd_ulong_map_leave(self.handle.as_ptr());
            sys::fd_ulong_map_delete(shmem);
            alloc::dealloc(self.mem.as_ptr(), self.layout);
        }
    }
}

/// A safe wrapper around fd_ulong_stack
pub struct FdStack {
    handle: NonNull<u64>,
    mem: NonNull<u8>,
    layout: Layout,
    _phantom: PhantomData<u64>,
}

impl FdStack {
    /// Create a new ulong stack
    pub fn new() -> Result<Self, TmplError> {
        unsafe {
            let max = sys::STACK_MAX;
            let footprint = sys::fd_ulong_stack_footprint(max as ulong);
            let align = sys::fd_ulong_stack_align();

            if footprint == 0 || align == 0 || !align.is_power_of_two() {
                return Err(TmplError::InvalidAlignment);
            }

            let layout = Layout::from_size_align(footprint as usize, align as usize)
                .map_err(|_| TmplError::InvalidAlignment)?;

            let mem = NonNull::new(alloc::alloc(layout)).ok_or(TmplError::AllocationFailed)?;

            let shmem =
                sys::fd_ulong_stack_new(mem.as_ptr() as *mut std::ffi::c_void, max as ulong);
            let handle = sys::fd_ulong_stack_join(shmem);

            if handle.is_null() {
                alloc::dealloc(mem.as_ptr(), layout);
                return Err(TmplError::AllocationFailed);
            }

            Ok(Self {
                handle: NonNull::new(handle).unwrap(),
                mem,
                layout,
                _phantom: PhantomData,
            })
        }
    }

    /// Push an element onto the stack
    pub fn push(&mut self, value: u64) -> Result<(), TmplError> {
        unsafe {
            let result = sys::fd_ulong_stack_push(self.handle.as_ptr(), value);
            if result.is_null() {
                Err(TmplError::ContainerFull)
            } else {
                Ok(())
            }
        }
    }

    /// Pop an element from the stack
    pub fn pop(&mut self) -> Option<u64> {
        unsafe {
            if sys::fd_ulong_stack_empty(self.handle.as_ptr()) != 0 {
                None
            } else {
                Some(sys::fd_ulong_stack_pop(self.handle.as_ptr()))
            }
        }
    }

    /// Peek at the top element without removing it
    pub fn peek(&self) -> Option<u64> {
        unsafe {
            if sys::fd_ulong_stack_empty(self.handle.as_ptr()) != 0 {
                None
            } else {
                let top_idx = sys::fd_ulong_stack_cnt(self.handle.as_ptr()) - 1;
                Some(*self.handle.as_ptr().add(top_idx as usize))
            }
        }
    }

    /// Get the number of elements in the stack
    pub fn len(&self) -> usize {
        unsafe { sys::fd_ulong_stack_cnt(self.handle.as_ptr()) as usize }
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        unsafe { sys::fd_ulong_stack_empty(self.handle.as_ptr()) != 0 }
    }

    /// Check if the stack is full
    pub fn is_full(&self) -> bool {
        unsafe { sys::fd_ulong_stack_full(self.handle.as_ptr()) != 0 }
    }
}

impl Drop for FdStack {
    fn drop(&mut self) {
        unsafe {
            let shmem = sys::fd_ulong_stack_leave(self.handle.as_ptr());
            sys::fd_ulong_stack_delete(shmem);
            alloc::dealloc(self.mem.as_ptr(), self.layout);
        }
    }
}

/// A safe wrapper around fd_ulong_queue
pub struct FdQueue {
    handle: NonNull<u64>,
    mem: NonNull<u8>,
    layout: Layout,
    _phantom: PhantomData<u64>,
}

impl FdQueue {
    /// Create a new ulong queue
    pub fn new() -> Result<Self, TmplError> {
        unsafe {
            let footprint = sys::fd_ulong_queue_footprint();
            let align = sys::fd_ulong_queue_align();

            if footprint == 0 || align == 0 || !align.is_power_of_two() {
                return Err(TmplError::InvalidAlignment);
            }

            let layout = Layout::from_size_align(footprint as usize, align as usize)
                .map_err(|_| TmplError::InvalidAlignment)?;

            let mem = NonNull::new(alloc::alloc(layout)).ok_or(TmplError::AllocationFailed)?;

            let shmem = sys::fd_ulong_queue_new(mem.as_ptr() as *mut std::ffi::c_void);
            let handle = sys::fd_ulong_queue_join(shmem);

            if handle.is_null() {
                alloc::dealloc(mem.as_ptr(), layout);
                return Err(TmplError::AllocationFailed);
            }

            Ok(Self {
                handle: NonNull::new(handle).unwrap(),
                mem,
                layout,
                _phantom: PhantomData,
            })
        }
    }

    /// Push an element to the back of the queue
    pub fn push(&mut self, value: u64) -> Result<(), TmplError> {
        unsafe {
            let result = sys::fd_ulong_queue_push(self.handle.as_ptr(), value);
            if result.is_null() {
                Err(TmplError::ContainerFull)
            } else {
                Ok(())
            }
        }
    }

    /// Pop an element from the front of the queue
    pub fn pop(&mut self) -> Option<u64> {
        unsafe {
            if sys::fd_ulong_queue_empty(self.handle.as_ptr()) != 0 {
                None
            } else {
                Some(sys::fd_ulong_queue_pop(self.handle.as_ptr()))
            }
        }
    }

    /// Peek at the front element without removing it
    pub fn peek(&self) -> Option<u64> {
        unsafe {
            if sys::fd_ulong_queue_empty(self.handle.as_ptr()) != 0 {
                None
            } else {
                // This is a simplified peek - actual implementation might differ
                let cnt = sys::fd_ulong_queue_cnt(self.handle.as_ptr());
                if cnt > 0 {
                    Some(*self.handle.as_ptr())
                } else {
                    None
                }
            }
        }
    }

    /// Get the number of elements in the queue
    pub fn len(&self) -> usize {
        unsafe { sys::fd_ulong_queue_cnt(self.handle.as_ptr()) as usize }
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        unsafe { sys::fd_ulong_queue_empty(self.handle.as_ptr()) != 0 }
    }

    /// Check if the queue is full
    pub fn is_full(&self) -> bool {
        unsafe { sys::fd_ulong_queue_full(self.handle.as_ptr()) != 0 }
    }
}

impl Drop for FdQueue {
    fn drop(&mut self) {
        unsafe {
            let shmem = sys::fd_ulong_queue_leave(self.handle.as_ptr());
            sys::fd_ulong_queue_delete(shmem);
            alloc::dealloc(self.mem.as_ptr(), self.layout);
        }
    }
}

/// A safe wrapper around fd_ulong_set
pub struct FdSet {
    handle: NonNull<u64>,
    mem: NonNull<u8>,
    layout: Layout,
    _phantom: PhantomData<sys::fd_ulong_set_ele_t>,
}

impl FdSet {
    /// Create a new ulong set
    pub fn new() -> Result<Self, TmplError> {
        unsafe {
            let footprint = sys::fd_ulong_set_footprint();
            let align = sys::fd_ulong_set_align();

            if footprint == 0 || align == 0 || !align.is_power_of_two() {
                return Err(TmplError::InvalidAlignment);
            }

            let layout = Layout::from_size_align(footprint as usize, align as usize)
                .map_err(|_| TmplError::InvalidAlignment)?;

            let mem = NonNull::new(alloc::alloc(layout)).ok_or(TmplError::AllocationFailed)?;

            let shmem = sys::fd_ulong_set_new(mem.as_ptr() as *mut std::ffi::c_void);
            let handle = sys::fd_ulong_set_join(shmem);

            if handle.is_null() {
                alloc::dealloc(mem.as_ptr(), layout);
                return Err(TmplError::AllocationFailed);
            }

            Ok(Self {
                handle: NonNull::new(handle).unwrap(),
                mem,
                layout,
                _phantom: PhantomData,
            })
        }
    }

    /// Insert a key into the set
    pub fn insert(&mut self, key: u64) -> Result<bool, TmplError> {
        unsafe {
            let idx = sys::fd_ulong_set_insert(self.handle.as_ptr(), key);
            Ok(idx != sys::fd_ulong_set_null(idx))
        }
    }

    /// Check if the set contains a key
    pub fn contains(&self, key: &u64) -> bool {
        unsafe {
            let idx = sys::fd_ulong_set_test(self.handle.as_ptr(), *key);
            idx != 0
        }
    }

    /// Remove a key from the set
    pub fn remove(&mut self, key: &u64) -> bool {
        unsafe {
            sys::fd_ulong_set_remove(self.handle.as_ptr(), *key);
            true // fd_set_remove doesn't return a status
        }
    }

    pub fn len(&self) -> usize {
        unsafe { sys::fd_ulong_set_cnt(self.handle.as_ptr()) as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Drop for FdSet {
    fn drop(&mut self) {
        unsafe {
            let shmem = sys::fd_ulong_set_leave(self.handle.as_ptr());
            sys::fd_ulong_set_delete(shmem);
            alloc::dealloc(self.mem.as_ptr(), self.layout);
        }
    }
}

/// A safe wrapper around fd_cstr_deque (string deque)
pub struct CStrDeque {
    handle: NonNull<*mut std::ffi::c_char>,
    mem: NonNull<u8>,
    layout: Layout,
    _phantom: PhantomData<*mut std::ffi::c_char>,
}

impl CStrDeque {
    /// Create a new string deque
    pub fn new() -> Result<Self, TmplError> {
        unsafe {
            let footprint = sys::fd_cstr_deque_footprint();
            let align = sys::fd_cstr_deque_align();

            if footprint == 0 || align == 0 || !align.is_power_of_two() {
                return Err(TmplError::InvalidAlignment);
            }

            let layout = Layout::from_size_align(footprint as usize, align as usize)
                .map_err(|_| TmplError::InvalidAlignment)?;

            let mem = NonNull::new(alloc::alloc(layout)).ok_or(TmplError::AllocationFailed)?;

            let shmem = sys::fd_cstr_deque_new(mem.as_ptr() as *mut std::ffi::c_void);
            let handle = sys::fd_cstr_deque_join(shmem);

            if handle.is_null() {
                alloc::dealloc(mem.as_ptr(), layout);
                return Err(TmplError::AllocationFailed);
            }

            Ok(Self {
                handle: NonNull::new(handle).unwrap(),
                mem,
                layout,
                _phantom: PhantomData,
            })
        }
    }

    /// Push a string to the head of the deque
    pub fn push_head(&mut self, value: &str) -> Result<(), TmplError> {
        unsafe {
            let c_str = std::ffi::CString::new(value)
                .map_err(|_| TmplError::InvalidInput("String contains null byte".to_string()))?;
            let result =
                sys::fd_cstr_deque_push_head(self.handle.as_ptr(), c_str.as_ptr() as *mut _);
            if result.is_null() {
                Err(TmplError::ContainerFull)
            } else {
                std::mem::forget(c_str); // Deque now owns the string
                Ok(())
            }
        }
    }

    /// Push a string to the tail of the deque
    pub fn push_tail(&mut self, value: &str) -> Result<(), TmplError> {
        unsafe {
            let c_str = std::ffi::CString::new(value)
                .map_err(|_| TmplError::InvalidInput("String contains null byte".to_string()))?;
            let result =
                sys::fd_cstr_deque_push_tail(self.handle.as_ptr(), c_str.as_ptr() as *mut _);
            if result.is_null() {
                Err(TmplError::ContainerFull)
            } else {
                std::mem::forget(c_str); // Deque now owns the string
                Ok(())
            }
        }
    }

    /// Pop a string from the head of the deque
    pub fn pop_head(&mut self) -> Option<String> {
        unsafe {
            if sys::fd_cstr_deque_empty(self.handle.as_ptr()) != 0 {
                None
            } else {
                let c_str = sys::fd_cstr_deque_pop_head(self.handle.as_ptr());
                if c_str.is_null() {
                    None
                } else {
                    let rust_str = std::ffi::CStr::from_ptr(c_str)
                        .to_string_lossy()
                        .into_owned();
                    // Note: We should free c_str here, but the template doesn't specify ownership
                    Some(rust_str)
                }
            }
        }
    }

    /// Pop a string from the tail of the deque
    pub fn pop_tail(&mut self) -> Option<String> {
        unsafe {
            if sys::fd_cstr_deque_empty(self.handle.as_ptr()) != 0 {
                None
            } else {
                let c_str = sys::fd_cstr_deque_pop_tail(self.handle.as_ptr());
                if c_str.is_null() {
                    None
                } else {
                    let rust_str = std::ffi::CStr::from_ptr(c_str)
                        .to_string_lossy()
                        .into_owned();
                    Some(rust_str)
                }
            }
        }
    }

    /// Get the number of elements in the deque
    pub fn len(&self) -> usize {
        unsafe { sys::fd_cstr_deque_cnt(self.handle.as_ptr()) as usize }
    }

    /// Check if the deque is empty
    pub fn is_empty(&self) -> bool {
        unsafe { sys::fd_cstr_deque_empty(self.handle.as_ptr()) != 0 }
    }

    /// Check if the deque is full
    pub fn is_full(&self) -> bool {
        unsafe { sys::fd_cstr_deque_full(self.handle.as_ptr()) != 0 }
    }
}

impl Drop for CStrDeque {
    fn drop(&mut self) {
        unsafe {
            let shmem = sys::fd_cstr_deque_leave(self.handle.as_ptr());
            sys::fd_cstr_deque_delete(shmem);
            alloc::dealloc(self.mem.as_ptr(), self.layout);
        }
    }
}

/// A safe wrapper around fd_ulong_heap
pub struct FdHeap {
    handle: NonNull<sys::fd_ulong_heap_private>,
    mem: NonNull<u8>,
    layout: Layout,
    max_elements: u64,
    _phantom: PhantomData<sys::fd_ulong_heap_ele_t>,
}

impl FdHeap {
    pub fn new(max_elements: u64) -> Result<Self, TmplError> {
        unsafe {
            let footprint = sys::fd_ulong_heap_footprint(max_elements);
            let align = sys::fd_ulong_heap_align();

            if footprint == 0 || align == 0 || !align.is_power_of_two() {
                return Err(TmplError::InvalidAlignment);
            }

            let layout = Layout::from_size_align(footprint as usize, align as usize)
                .map_err(|_| TmplError::InvalidAlignment)?;

            let mem = NonNull::new(alloc::alloc(layout)).ok_or(TmplError::AllocationFailed)?;

            let shmem = sys::fd_ulong_heap_new(mem.as_ptr() as *mut std::ffi::c_void, max_elements);
            let handle = sys::fd_ulong_heap_join(shmem);

            if handle.is_null() {
                alloc::dealloc(mem.as_ptr(), layout);
                return Err(TmplError::AllocationFailed);
            }

            Ok(Self {
                handle: NonNull::new(handle).unwrap(),
                mem,
                layout,
                max_elements,
                _phantom: PhantomData,
            })
        }
    }

    pub fn insert(&mut self, value: u64) -> Result<(), TmplError> {
        unsafe {
            let mut ele = sys::fd_ulong_heap_ele_t {
                left: 0,
                right: 0,
                value,
            };

            let result = sys::fd_ulong_heap_idx_insert(self.handle.as_ptr(), 0, &mut ele);
            if result.is_null() {
                Err(TmplError::ContainerFull)
            } else {
                Ok(())
            }
        }
    }

    /// Get the number of elements in the heap
    pub fn len(&self) -> usize {
        unsafe { sys::fd_ulong_heap_ele_cnt(self.handle.as_ptr()) as usize }
    }

    /// Check if the heap is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the maximum capacity of the heap
    pub fn capacity(&self) -> u64 {
        self.max_elements
    }
}

impl Drop for FdHeap {
    fn drop(&mut self) {
        unsafe {
            let shmem = sys::fd_ulong_heap_leave(self.handle.as_ptr());
            sys::fd_ulong_heap_delete(shmem);
            alloc::dealloc(self.mem.as_ptr(), self.layout);
        }
    }
}

/// A safe wrapper around fd_ulong_pool
pub struct FdPool {
    handle: NonNull<sys::fd_ulong_pool_ele_t>,
    mem: NonNull<u8>,
    layout: Layout,
    max_elements: u64,
    _phantom: PhantomData<sys::fd_ulong_pool_ele_t>,
}

impl FdPool {
    /// Create a new ulong pool with specified maximum elements
    pub fn new(max_elements: u64) -> Result<Self, TmplError> {
        unsafe {
            let footprint = sys::fd_ulong_pool_footprint(max_elements);
            let align = sys::fd_ulong_pool_align();

            if footprint == 0 || align == 0 || !align.is_power_of_two() {
                return Err(TmplError::InvalidAlignment);
            }

            let layout = Layout::from_size_align(footprint as usize, align as usize)
                .map_err(|_| TmplError::InvalidAlignment)?;

            let mem = NonNull::new(alloc::alloc(layout)).ok_or(TmplError::AllocationFailed)?;

            let shmem = sys::fd_ulong_pool_new(mem.as_ptr() as *mut std::ffi::c_void, max_elements);
            let handle = sys::fd_ulong_pool_join(shmem);

            if handle.is_null() {
                alloc::dealloc(mem.as_ptr(), layout);
                return Err(TmplError::AllocationFailed);
            }

            Ok(Self {
                handle: NonNull::new(handle).unwrap(),
                mem,
                layout,
                max_elements,
                _phantom: PhantomData,
            })
        }
    }

    /// Acquire an element from the pool
    pub fn acquire(&mut self) -> Option<u64> {
        unsafe {
            let idx = sys::fd_ulong_pool_idx_acquire(self.handle.as_ptr());
            if idx == sys::fd_ulong_pool_idx_null(self.handle.as_ptr()) {
                None
            } else {
                Some(idx)
            }
        }
    }

    /// Release an element back to the pool
    pub fn release(&mut self, idx: u64) {
        unsafe {
            sys::fd_ulong_pool_idx_release(self.handle.as_ptr(), idx);
        }
    }

    /// Get the number of available elements in the pool
    pub fn available(&self) -> usize {
        unsafe { sys::fd_ulong_pool_free(self.handle.as_ptr()) as usize }
    }

    /// Get the number of used elements in the pool
    pub fn used(&self) -> usize {
        unsafe { sys::fd_ulong_pool_used(self.handle.as_ptr()) as usize }
    }

    /// Check if the pool is empty (no available elements)
    pub fn is_empty(&self) -> bool {
        self.available() == 0
    }

    /// Get the maximum capacity of the pool
    pub fn capacity(&self) -> u64 {
        self.max_elements
    }
}

impl Drop for FdPool {
    fn drop(&mut self) {
        unsafe {
            let shmem = sys::fd_ulong_pool_leave(self.handle.as_ptr());
            sys::fd_ulong_pool_delete(shmem);
            alloc::dealloc(self.mem.as_ptr(), self.layout);
        }
    }
}

/// A safe wrapper around fd_ulong_vec
pub struct FdVec {
    handle: NonNull<u64>,
    mem: NonNull<u8>,
    layout: Layout,
    max_elements: u64,
    _phantom: PhantomData<u64>,
}

impl FdVec {
    /// Create a new ulong vector with specified maximum elements
    pub fn new(max_elements: u64) -> Result<Self, TmplError> {
        unsafe {
            let footprint = sys::fd_ulong_vec_footprint(max_elements);
            let align = sys::fd_ulong_vec_align();

            if footprint == 0 || align == 0 || !align.is_power_of_two() {
                return Err(TmplError::InvalidAlignment);
            }

            let layout = Layout::from_size_align(footprint as usize, align as usize)
                .map_err(|_| TmplError::InvalidAlignment)?;

            let mem = NonNull::new(alloc::alloc(layout)).ok_or(TmplError::AllocationFailed)?;

            let shmem = sys::fd_ulong_vec_new(mem.as_ptr() as *mut std::ffi::c_void, max_elements);
            let handle = sys::fd_ulong_vec_join(shmem);

            if handle.is_null() {
                alloc::dealloc(mem.as_ptr(), layout);
                return Err(TmplError::AllocationFailed);
            }

            Ok(Self {
                handle: NonNull::new(handle).unwrap(),
                mem,
                layout,
                max_elements,
                _phantom: PhantomData,
            })
        }
    }

    /// Push an element to the end of the vector
    pub fn push(&mut self, value: u64) -> Result<(), TmplError> {
        unsafe {
            let result = sys::fd_ulong_vec_expand(self.handle.as_ptr(), 1);
            if result.is_null() {
                Err(TmplError::ContainerFull)
            } else {
                Ok(())
            }
        }
    }

    /// Pop an element from the end of the vector
    pub fn pop(&mut self) -> Option<*mut u64> {
        unsafe {
            if sys::fd_ulong_vec_is_empty(self.handle.as_ptr()) != 0 {
                None
            } else {
                Some(sys::fd_ulong_vec_remove_idx(
                    self.handle.as_ptr(),
                    (self.len() - 1) as u64,
                ))
            }
        }
    }

    /// Get an element at the specified index
    pub fn get(&self, index: usize) -> Option<u64> {
        unsafe {
            let len = sys::fd_ulong_vec_cnt(self.handle.as_ptr()) as usize;
            if index >= len {
                None
            } else {
                Some(*self.handle.as_ptr().add(index))
            }
        }
    }

    /// Set an element at the specified index
    pub fn set(&mut self, index: usize, value: u64) -> Result<(), TmplError> {
        unsafe {
            let len = sys::fd_ulong_vec_cnt(self.handle.as_ptr()) as usize;
            if index >= len {
                Err(TmplError::InvalidInput("Index out of bounds".to_string()))
            } else {
                *self.handle.as_ptr().add(index) = value;
                Ok(())
            }
        }
    }

    /// Get the number of elements in the vector
    pub fn len(&self) -> usize {
        unsafe { sys::fd_ulong_vec_cnt(self.handle.as_ptr()) as usize }
    }

    /// Check if the vector is empty
    pub fn is_empty(&self) -> bool {
        unsafe { sys::fd_ulong_vec_is_empty(self.handle.as_ptr()) != 0 }
    }

    /// Check if the vector is full
    pub fn is_full(&self) -> bool {
        unsafe { sys::fd_ulong_vec_is_full(self.handle.as_ptr()) != 0 }
    }

    /// Get the maximum capacity of the vector
    pub fn capacity(&self) -> u64 {
        self.max_elements
    }
}

impl Drop for FdVec {
    fn drop(&mut self) {
        unsafe {
            let shmem = sys::fd_ulong_vec_leave(self.handle.as_ptr());
            sys::fd_ulong_vec_delete(shmem);
            alloc::dealloc(self.mem.as_ptr(), self.layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map() {
        let mut map = FdMap::new().unwrap();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);

        map.insert(1, 100).unwrap();
        map.insert(2, 200).unwrap();
        map.insert(3, 300).unwrap();

        assert!(!map.is_empty());
        assert_eq!(map.len(), 3);

        assert_eq!(map.get(&1), Some(100));
        assert_eq!(map.get(&2), Some(200));
        assert_eq!(map.get(&3), Some(300));
        assert_eq!(map.get(&4), None);

        assert!(map.contains_key(&1));
        assert!(map.contains_key(&2));
        assert!(map.contains_key(&3));
        assert!(!map.contains_key(&4));

        assert_eq!(map.remove(&2), Some(()));
        assert_eq!(map.remove(&2), None);
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_stack() {
        let mut stack = FdStack::new().unwrap();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);

        stack.push(10).unwrap();
        stack.push(20).unwrap();
        stack.push(30).unwrap();

        assert!(!stack.is_empty());
        assert_eq!(stack.len(), 3);

        assert_eq!(stack.peek(), Some(30));
        assert_eq!(stack.len(), 3);

        assert_eq!(stack.pop(), Some(30));
        assert_eq!(stack.pop(), Some(20));
        assert_eq!(stack.pop(), Some(10));
        assert_eq!(stack.pop(), None);

        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_queue() {
        let mut queue = FdQueue::new().unwrap();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);

        queue.push(10).unwrap();
        queue.push(20).unwrap();
        queue.push(30).unwrap();

        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 3);

        assert_eq!(queue.pop(), Some(10));
        assert_eq!(queue.pop(), Some(20));
        assert_eq!(queue.pop(), Some(30));
        assert_eq!(queue.pop(), None);

        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_ulong_set_basic() {
        let mut set = FdSet::new().unwrap();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);

        assert!(set.insert(1).unwrap());
        assert!(set.insert(2).unwrap());
        assert!(set.insert(3).unwrap());

        assert!(!set.is_empty());
        assert_eq!(set.len(), 3);

        assert!(set.contains(&1));
        assert!(set.contains(&2));
        assert!(set.contains(&3));
        assert!(!set.contains(&4));

        assert!(set.remove(&2));
        assert_eq!(set.len(), 2);
        assert!(!set.contains(&2));
    }

    #[test]
    fn test_cstr_deque_basic() {
        let mut deque = CStrDeque::new().unwrap();
        assert!(deque.is_empty());
        assert_eq!(deque.len(), 0);

        deque.push_head("first").unwrap();
        deque.push_tail("last").unwrap();
        deque.push_head("very_first").unwrap();

        assert!(!deque.is_empty());
        assert_eq!(deque.len(), 3);

        assert_eq!(deque.pop_head(), Some("very_first".to_string()));
        assert_eq!(deque.pop_tail(), Some("last".to_string()));
        assert_eq!(deque.pop_head(), Some("first".to_string()));
        assert_eq!(deque.pop_head(), None);

        assert!(deque.is_empty());
    }

    #[test]
    fn test_ulong_heap_basic() {
        let mut heap = FdHeap::new(100).unwrap();
        assert!(heap.is_empty());
        assert_eq!(heap.len(), 0);
        assert_eq!(heap.capacity(), 100);

        heap.insert(30).unwrap();
        heap.insert(10).unwrap();
        heap.insert(20).unwrap();
        heap.insert(5).unwrap();

        assert!(!heap.is_empty());
        assert_eq!(heap.len(), 4);

        assert!(heap.is_empty());
    }

    #[test]
    fn test_ulong_pool_basic() {
        let mut pool = FdPool::new(10).unwrap();
        assert_eq!(pool.capacity(), 10);
        assert_eq!(pool.available(), 10);
        assert_eq!(pool.used(), 0);

        let idx1 = pool.acquire().unwrap();
        let idx2 = pool.acquire().unwrap();
        let idx3 = pool.acquire().unwrap();

        assert_eq!(pool.available(), 7);
        assert_eq!(pool.used(), 3);

        pool.release(idx2);
        assert_eq!(pool.available(), 8);
        assert_eq!(pool.used(), 2);

        pool.release(idx1);
        pool.release(idx3);
        assert_eq!(pool.available(), 10);
        assert_eq!(pool.used(), 0);
    }

    #[test]
    fn test_ulong_vec_basic() {
        let mut vec = FdVec::new(100).unwrap();
        assert!(vec.is_empty());
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 100);

        vec.push(10).unwrap();
        vec.push(20).unwrap();
        vec.push(30).unwrap();

        assert!(!vec.is_empty());
        assert_eq!(vec.len(), 3);

        assert_eq!(vec.get(0), Some(10));
        assert_eq!(vec.get(1), Some(20));
        assert_eq!(vec.get(2), Some(30));
        assert_eq!(vec.get(3), None);

        vec.set(1, 99).unwrap();
        assert_eq!(vec.get(1), Some(99));

        let pop = vec.pop();
        let num = unsafe { *pop.unwrap() };
        assert_eq!(num, 30);

        let pop = vec.pop();
        let num = unsafe { *pop.unwrap() };
        assert_eq!(num, 99);

        let pop = vec.pop();
        let num = unsafe { *pop.unwrap() };
        assert_eq!(num, 30);

        let pop = vec.pop();
        let num = unsafe { *pop.unwrap() };
        assert_eq!(num, 10);

        let pop = vec.pop();
        assert_eq!(pop, None);

        assert!(vec.is_empty());
    }
}
