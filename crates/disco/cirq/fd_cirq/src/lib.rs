//! Safe API for `fd_circq_sys`
#![no_std]

use core::marker::PhantomData;
use core::ptr::NonNull;
use core::slice;

use fd_cirq_sys::*;
use firedancer_rs_common::define_errors;

define_errors!(
    CircqError,
    { InvalidAlignment => "Alignment must be a power of 2 and <= 4096" },
    { MessageTooLarge => "Message size exceeds queue capacity" },
    { AllocationFailed => "Failed to allocate memory for queue" },
    { NullPointer => "Null pointer encountered" }
);

/// A circular queue for storing messages with automatic eviction.
///
/// The circular queue is a fixed-size data structure that stores messages
/// in a FIFO manner. When new messages are pushed and there isn't enough
/// space, old messages are automatically evicted to make room.
pub struct Cirq {
    ptr: NonNull<fd_circq_t>,
    _phantom: PhantomData<fd_circq_t>,
}

impl Cirq {
    pub fn align() -> usize {
        unsafe { fd_circq_align() as usize }
    }

    pub fn footprint(size: usize) -> usize {
        unsafe { fd_circq_footprint(size as u64) as usize }
    }

    /// Creates a new circular queue in the provided memory
    ///
    /// # Safety
    ///
    /// - `shmem` must point to valid memory of at least `footprint(size)` bytes
    /// - `shmem` must be properly aligned (use `align()` to get alignment requirement)
    /// - The memory must remain valid for the lifetime of the Cirq
    pub unsafe fn new(shmem: *mut u8, size: usize) -> Option<Self> {
        core::ptr::write_bytes(shmem, 0, Self::footprint(size));
        let ptr = fd_circq_new(shmem as *mut core::ffi::c_void, size as u64);
        NonNull::new(ptr as *mut fd_circq_t).map(|ptr| Self {
            ptr,
            _phantom: PhantomData,
        })
    }

    /// Joins an existing circular queue in memory
    ///
    /// # Safety
    ///
    /// - `shbuf` must point to a valid circular queue created with `new()`
    /// - The memory must remain valid for the lifetime of the Cirq
    pub unsafe fn join(shbuf: *mut u8) -> Option<Self> {
        let ptr = fd_circq_join(shbuf as *mut core::ffi::c_void);
        NonNull::new(ptr).map(|ptr| Self {
            ptr,
            _phantom: PhantomData,
        })
    }

    /// Pushes a message to the back of the queue
    ///
    /// Returns a mutable slice where the message data can be written.
    /// If the operation fails (due to invalid alignment or message too large),
    /// returns None.
    ///
    /// # Arguments
    ///
    /// - `align`: Required alignment for the message (must be power of 2, ≤ 4096)
    /// - `footprint`: Size of the message in bytes
    pub fn push_back(&mut self, align: usize, footprint: usize) -> Option<&mut [u8]> {
        unsafe {
            let ptr = fd_circq_push_back(self.ptr.as_ptr(), align as u64, footprint as u64);
            if ptr.is_null() {
                None
            } else {
                Some(slice::from_raw_parts_mut(ptr, footprint))
            }
        }
    }

    /// Pops the oldest message from the front of the queue
    ///
    /// Returns a slice containing the message data, or None if the queue is empty.
    /// The returned slice is valid until the next call to `push_back()`.
    pub fn pop_front(&mut self) -> Option<&[u8]> {
        unsafe {
            let ptr = fd_circq_pop_front(self.ptr.as_ptr());
            if ptr.is_null() {
                None
            } else {
                // struct fd_circq_message_private {
                //   ulong align;      // 0x0
                //   ulong footprint;  // 0x8
                //   ulong next;       // 0x10
                // }; // sz: 24 (3 * 8)
                //
                // go back 24 bytes then read footprint (0x8)
                let message_ptr = (ptr as *const u8).offset(-24); // retrace to start of message
                let footprint_ptr = (message_ptr as *const u64).offset(1); // footprint -> 0x8 (1 * u64)
                let footprint = *footprint_ptr as usize;

                Some(slice::from_raw_parts(ptr, footprint))
            }
        }
    }

    pub fn count(&self) -> usize {
        unsafe { (*self.ptr.as_ptr()).cnt as usize }
    }

    pub fn size(&self) -> usize {
        unsafe { (*self.ptr.as_ptr()).size as usize }
    }

    /// Returns the number of messages that have been evicted
    pub fn drop_count(&self) -> usize {
        unsafe { (*self.ptr.as_ptr()).metrics.drop_cnt as usize }
    }
}

impl Drop for Cirq {
    fn drop(&mut self) {
        unsafe {
            fd_circq_leave(self.ptr.as_ptr());
        }
    }
}

// Cirq is Send and Sync if properly synchronized externally
unsafe impl Send for Cirq {}
unsafe impl Sync for Cirq {}

/// A builder for creating circular queues with proper memory management.
pub struct CirqBuilder {
    size: usize,
}

impl CirqBuilder {
    /// Creates a new builder for a circular queue of the given size.
    pub fn new(size: usize) -> Self {
        Self { size }
    }

    /// Returns the required memory footprint for this queue configuration.
    pub fn footprint(&self) -> usize {
        Cirq::footprint(self.size)
    }

    /// Returns the required alignment for this queue configuration.
    pub fn align(&self) -> usize {
        Cirq::align()
    }

    /// Builds the circular queue using the provided memory.
    ///
    /// # Safety
    ///
    /// - `memory` must be at least `footprint()` bytes long
    /// - `memory` must be aligned to `align()` bytes
    /// - The memory must remain valid for the lifetime of the returned Cirq
    pub unsafe fn build(self, memory: *mut u8) -> Option<Cirq> {
        Cirq::new(memory, self.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern crate alloc;
    use alloc::alloc::{alloc, dealloc, Layout};

    #[test]
    fn test_circular_queue_creation() {
        let size = 1024;
        let footprint = Cirq::footprint(size);
        let align = Cirq::align();

        unsafe {
            let layout = Layout::from_size_align(footprint, align).unwrap();
            let memory = alloc(layout);
            assert!(!memory.is_null());

            let queue = Cirq::new(memory, size);
            assert!(queue.is_some());

            let queue = queue.unwrap();
            assert_eq!(queue.count(), 0);
            assert_eq!(queue.size(), size);
            assert_eq!(queue.drop_count(), 0);

            drop(queue);
            dealloc(memory, layout);
        }
    }

    #[test]
    fn test_builder() {
        let size = 2048;
        let builder = CirqBuilder::new(size);

        let footprint = builder.footprint();
        let align = builder.align();

        assert!(footprint > size);
        assert!(align > 0);
        assert!(align.is_power_of_two());
    }
}
