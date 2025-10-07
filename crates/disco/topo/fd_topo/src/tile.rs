//! Tile management for Firedancer topology.
//!
//! A tile is a unique process that is spawned by Firedancer to represent
//! one thread of execution. Firedancer sandboxes all tiles to their own
//! process for security reasons.

use core::ffi::CStr;
use fd_topo_sys as sys;

pub struct Tile {
    inner: *mut sys::fd_topo_tile_t,
}

impl Tile {
    /// Create a tile wrapper from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ptr` is a valid pointer to an initialized
    /// `fd_topo_tile_t` that remains valid for the lifetime of this `Tile`.
    pub unsafe fn from_raw(ptr: *mut sys::fd_topo_tile_t) -> Self {
        Self { inner: ptr }
    }

    /// Get the raw pointer to the underlying tile.
    pub fn as_ptr(&self) -> *const sys::fd_topo_tile_t {
        self.inner
    }

    /// Get a mutable raw pointer to the underlying tile.
    pub fn as_mut_ptr(&mut self) -> *mut sys::fd_topo_tile_t {
        self.inner
    }

    /// Get the tile ID.
    pub fn id(&self) -> usize {
        unsafe { (*self.inner).id as usize }
    }

    /// Get the tile name.
    pub fn name(&self) -> &str {
        unsafe {
            CStr::from_ptr((*self.inner).name.as_ptr())
                .to_str()
                .unwrap_or("")
        }
    }

    /// Get the tile kind ID.
    pub fn kind_id(&self) -> usize {
        unsafe { (*self.inner).kind_id as usize }
    }

    /// Check if this is an Agave tile.
    pub fn is_agave(&self) -> bool {
        unsafe { (*self.inner).is_agave != 0 }
    }

    /// Check if the tile is allowed to shutdown gracefully.
    pub fn allow_shutdown(&self) -> bool {
        unsafe { (*self.inner).allow_shutdown != 0 }
    }

    /// Get the CPU index this tile is pinned to.
    /// Returns `None` if the tile is floating (not pinned).
    pub fn cpu_idx(&self) -> Option<usize> {
        unsafe {
            let cpu_idx = (*self.inner).cpu_idx;
            if cpu_idx == u64::MAX {
                None
            } else {
                Some(cpu_idx as usize)
            }
        }
    }

    /// Get the number of input links.
    pub fn input_count(&self) -> usize {
        unsafe { (*self.inner).in_cnt as usize }
    }

    /// Get the number of output links.
    pub fn output_count(&self) -> usize {
        unsafe { (*self.inner).out_cnt as usize }
    }

    /// Get the input link IDs.
    pub fn input_link_ids(&self) -> Vec<usize> {
        unsafe {
            let count = (*self.inner).in_cnt as usize;
            let mut ids = Vec::with_capacity(count);
            for i in 0..count {
                ids.push((*self.inner).in_link_id[i] as usize);
            }
            ids
        }
    }

    /// Get the output link IDs.
    pub fn output_link_ids(&self) -> Vec<usize> {
        unsafe {
            let count = (*self.inner).out_cnt as usize;
            let mut ids = Vec::with_capacity(count);
            for i in 0..count {
                ids.push((*self.inner).out_link_id[i] as usize);
            }
            ids
        }
    }

    /// Check if a specific input link is reliable.
    pub fn is_input_link_reliable(&self, index: usize) -> Option<bool> {
        unsafe {
            if index < (*self.inner).in_cnt as usize {
                Some((*self.inner).in_link_reliable[index] != 0)
            } else {
                None
            }
        }
    }

    /// Check if a specific input link is polled.
    pub fn is_input_link_polled(&self, index: usize) -> Option<bool> {
        unsafe {
            if index < (*self.inner).in_cnt as usize {
                Some((*self.inner).in_link_poll[index] != 0)
            } else {
                None
            }
        }
    }

    /// Get the tile object ID.
    pub fn tile_object_id(&self) -> usize {
        unsafe { (*self.inner).tile_obj_id as usize }
    }

    /// Get the metrics object ID.
    pub fn metrics_object_id(&self) -> usize {
        unsafe { (*self.inner).metrics_obj_id as usize }
    }

    /// Get the keyswitch object ID.
    pub fn keyswitch_object_id(&self) -> usize {
        unsafe { (*self.inner).keyswitch_obj_id as usize }
    }

    /// Get the number of objects this tile uses.
    pub fn uses_object_count(&self) -> usize {
        unsafe { (*self.inner).uses_obj_cnt as usize }
    }

    /// Get the IDs of objects this tile uses.
    pub fn uses_object_ids(&self) -> Vec<usize> {
        unsafe {
            let count = (*self.inner).uses_obj_cnt as usize;
            let mut ids = Vec::with_capacity(count);
            for i in 0..count {
                ids.push((*self.inner).uses_obj_id[i] as usize);
            }
            ids
        }
    }
}

unsafe impl Send for Tile {}
unsafe impl Sync for Tile {}
