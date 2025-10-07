use core::ffi::CStr;
use fd_topo_sys as sys;

/// A unique process spawned within a workspace, representing
/// one thread of execution. All tiles are sandboxed to their
/// own process for security reasons.
#[repr(C)]
pub struct Tile {
    inner: *mut sys::fd_topo_tile_t,
}

impl Tile {
    /// SAFETY: The caller must ensure that `ptr` is a valid pointer to an initialized
    /// `fd_topo_tile_t` that remains valid for the lifetime of this `Tile`.
    pub unsafe fn from_raw(ptr: *mut sys::fd_topo_tile_t) -> Self {
        Self { inner: ptr }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const sys::fd_topo_tile_t {
        self.inner
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut sys::fd_topo_tile_t {
        self.inner
    }

    #[inline]
    pub fn id(&self) -> usize {
        unsafe { (*self.inner).id as usize }
    }

    #[inline]
    pub fn name(&self) -> &str {
        unsafe {
            CStr::from_ptr((*self.inner).name.as_ptr())
                .to_str()
                .unwrap_or("")
        }
    }

    #[inline]
    pub fn kind_id(&self) -> usize {
        unsafe { (*self.inner).kind_id as usize }
    }

    /// is this an agave tile?
    #[inline]
    pub fn is_agave(&self) -> bool {
        unsafe { (*self.inner).is_agave != 0 }
    }

    /// is the tile allowed to shutdown gracefully?
    #[inline]
    pub fn allow_shutdown(&self) -> bool {
        unsafe { (*self.inner).allow_shutdown != 0 }
    }

    /// cpu index this tile is pinned to -- will return `None` if floating
    #[inline]
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

    /// number of input links
    #[inline]
    pub fn input_cnt(&self) -> usize {
        unsafe { (*self.inner).in_cnt as usize }
    }

    /// number of output links
    #[inline]
    pub fn output_cnt(&self) -> usize {
        unsafe { (*self.inner).out_cnt as usize }
    }

    #[inline]
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

    #[inline]
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

    /// is a specific input link reliable?
    #[inline]
    pub fn is_input_link_reliable(&self, index: usize) -> Option<bool> {
        unsafe {
            if index < (*self.inner).in_cnt as usize {
                Some((*self.inner).in_link_reliable[index] != 0)
            } else {
                None
            }
        }
    }

    /// is a specific input link polled?
    #[inline]
    pub fn is_input_link_polled(&self, index: usize) -> Option<bool> {
        unsafe {
            if index < (*self.inner).in_cnt as usize {
                Some((*self.inner).in_link_poll[index] != 0)
            } else {
                None
            }
        }
    }

    #[inline]
    pub fn tile_obj_id(&self) -> usize {
        unsafe { (*self.inner).tile_obj_id as usize }
    }

    #[inline]
    pub fn metrics_obj_id(&self) -> usize {
        unsafe { (*self.inner).metrics_obj_id as usize }
    }

    #[inline]
    pub fn keyswitch_obj_id(&self) -> usize {
        unsafe { (*self.inner).keyswitch_obj_id as usize }
    }

    /// number of objects this tile uses
    #[inline]
    pub fn uses_object_cnt(&self) -> usize {
        unsafe { (*self.inner).uses_obj_cnt as usize }
    }

    /// ids of objects this tile uses
    #[inline]
    pub fn uses_obj_ids(&self) -> Vec<usize> {
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
