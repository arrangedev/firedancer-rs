//! Low-level FFI bindings to Firedancer's fd_tmpl module.
//!
//! This crate provides raw, unsafe bindings to the Firedancer template data structures.
//! For safe, idiomatic Rust wrappers, see the `fd_tmpl` crate.
//!
//! # Overview
//!
//! The fd_tmpl system provides high-performance template-based data structures that are
//! generated at compile time using C macros. This includes:
//!
//! - **Maps**: Key-value associative containers with various implementations
//! - **Sets**: Key-only containers for membership testing
//! - **Deques**: Double-ended queues with constant-time push/pop at both ends
//! - **Heaps**: Priority queues with efficient min/max operations
//! - **Pools**: Object pools for efficient memory management
//! - **Queues**: FIFO queues with constant-time operations
//! - **Stacks**: LIFO stacks with constant-time operations
//! - **Vectors**: Dynamic arrays with amortized constant-time operations
//!
//! # Template System
//!
//! The template system works by including template source files (`.c` files) with
//! predefined macros that specify the container name, element type, and other
//! configuration options. This generates type-safe, high-performance code at
//! compile time.
//!
//! # Safety
//!
//! All functions in this crate are unsafe and require careful handling of:
//! - Memory management and lifetime guarantees
//! - Type safety across template instantiations
//! - Proper initialization and cleanup of data structures
//! - Thread safety considerations
//!
//! # Example
//!
//! ```rust,no_run
//! use fd_tmpl_sys::*;
//! use std::ptr;
//!
//! unsafe {
//!     // Create a ulong map
//!     let map_size = fd_ulong_map_footprint();
//!     let map_align = fd_ulong_map_align();
//!     
//!     // Allocate aligned memory (simplified example)
//!     let map_mem = std::alloc::alloc(
//!         std::alloc::Layout::from_size_align(map_size as usize, map_align as usize).unwrap()
//!     );
//!     
//!     // Initialize the map
//!     let map_shmem = fd_ulong_map_new(map_mem as *mut std::ffi::c_void);
//!     let map = fd_ulong_map_join(map_shmem);
//!     
//!     if !map.is_null() {
//!         // Use the map...
//!         let key = 42u64;
//!         let value = 123u64;
//!         
//!         // Create an element
//!         let mut ele = fd_ulong_map_ele_t {
//!             key,
//!             hash: key as u32, // Simple hash
//!             value,
//!         };
//!         
//!         // Insert into map  
//!         let inserted = fd_ulong_map_insert(map, key);
//!         if inserted != fd_ulong_map_null() {
//!             println!("Successfully inserted key {} with value {}", key, value);
//!         }
//!         
//!         // Clean up
//!         let map_shmem = fd_ulong_map_leave(map);
//!         fd_ulong_map_delete(map_shmem);
//!     }
//!     
//!     std::alloc::dealloc(
//!         map_mem,
//!         std::alloc::Layout::from_size_align(map_size as usize, map_align as usize).unwrap()
//!     );
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
    use std::ptr;

    #[test]
    fn test_map_constants() {
        assert_eq!(FD_MAP_SUCCESS, 0);
        assert!(FD_MAP_ERR_INVAL < 0);
        assert!(FD_MAP_ERR_FULL < 0);
        assert!(FD_MAP_ERR_KEY < 0);
    }

    #[test]
    fn test_map_footprint() {
        unsafe {
            let footprint = fd_ulong_map_footprint();
            let align = fd_ulong_map_align();

            assert!(footprint > 0);
            assert!(align > 0);
            assert!(align.is_power_of_two());
        }
    }

    #[test]
    fn test_heap_footprint() {
        unsafe {
            let max_elements = 100u64;
            let footprint = fd_ulong_heap_footprint(max_elements);
            let align = fd_ulong_heap_align();

            assert!(footprint > 0);
            assert!(align > 0);
            assert!(align.is_power_of_two());
        }
    }

    #[test]
    fn test_pool_footprint() {
        unsafe {
            let max_elements = 100u64;
            let footprint = fd_ulong_pool_footprint(max_elements);
            let align = fd_ulong_pool_align();

            assert!(footprint > 0);
            assert!(align > 0);
            assert!(align.is_power_of_two());
        }
    }

    #[test]
    fn test_queue_footprint() {
        unsafe {
            let footprint = fd_ulong_queue_footprint();
            let align = fd_ulong_queue_align();

            assert!(footprint > 0);
            assert!(align > 0);
            assert!(align.is_power_of_two());
        }
    }

    #[test]
    fn test_set_footprint() {
        unsafe {
            let footprint = fd_ulong_set_footprint();
            let align = fd_ulong_set_align();

            assert!(footprint > 0);
            assert!(align > 0);
            assert!(align.is_power_of_two());
        }
    }

    #[test]
    fn test_stack_footprint() {
        unsafe {
            let max_elements = 64u64;
            let footprint = fd_ulong_stack_footprint(max_elements);
            let align = fd_ulong_stack_align();

            assert!(footprint > 0);
            assert!(align > 0);
            assert!(align.is_power_of_two());
        }
    }

    #[test]
    fn test_vec_footprint() {
        unsafe {
            let max_elements = 100u64;
            let footprint = fd_ulong_vec_footprint(max_elements);
            let align = fd_ulong_vec_align();

            assert!(footprint > 0);
            assert!(align > 0);
            assert!(align.is_power_of_two());
        }
    }

    #[test]
    fn test_element_types() {
        // Test that our element types are properly defined
        let _map_ele = fd_ulong_map_ele_t {
            key: 0,
            hash: 0,
            value: 0,
        };

        let _heap_ele = fd_ulong_heap_ele_t {
            left: 0,
            right: 0,
            value: 0,
        };

        let _set_ele = fd_ulong_set_ele_t { key: 0, hash: 0 };

        let _pool_ele = fd_ulong_pool_ele_t { next: 0, value: 0 };
    }
}
