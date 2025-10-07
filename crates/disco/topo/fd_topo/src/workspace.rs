//! Workspace management for Firedancer topology.
//!
//! A workspace is a Firedancer specific memory management structure that
//! sits on top of 1 or more memory mapped gigantic or huge pages mounted
//! to the hugetlbfs.

use core::ffi::CStr;
use fd_topo_sys as sys;

/// Represents a workspace in the topology.
pub struct Workspace {
    inner: *mut sys::fd_topo_wksp_t,
}

impl Workspace {
    /// Create a workspace wrapper from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ptr` is a valid pointer to an initialized
    /// `fd_topo_wksp_t` that remains valid for the lifetime of this `Workspace`.
    pub unsafe fn from_raw(ptr: *mut sys::fd_topo_wksp_t) -> Self {
        Self { inner: ptr }
    }

    /// Get the raw pointer to the underlying workspace.
    pub fn as_ptr(&self) -> *const sys::fd_topo_wksp_t {
        self.inner
    }

    /// Get a mutable raw pointer to the underlying workspace.
    pub fn as_mut_ptr(&mut self) -> *mut sys::fd_topo_wksp_t {
        self.inner
    }

    /// Get the workspace ID.
    pub fn id(&self) -> usize {
        unsafe { (*self.inner).id as usize }
    }

    /// Get the workspace name.
    pub fn name(&self) -> &str {
        unsafe {
            CStr::from_ptr((*self.inner).name.as_ptr())
                .to_str()
                .unwrap_or("")
        }
    }

    /// Get the NUMA node index for this workspace.
    pub fn numa_idx(&self) -> usize {
        unsafe { (*self.inner).numa_idx as usize }
    }

    /// Check if the workspace uses locked pages.
    pub fn is_locked(&self) -> bool {
        unsafe { (*self.inner).is_locked != 0 }
    }

    /// Get the page size for this workspace.
    pub fn page_size(&self) -> usize {
        unsafe { (*self.inner).__bindgen_anon_1.page_sz as usize }
    }

    /// Get the number of pages in this workspace.
    pub fn page_count(&self) -> usize {
        unsafe { (*self.inner).__bindgen_anon_1.page_cnt as usize }
    }

    /// Get the maximum number of partitions.
    pub fn partition_max(&self) -> usize {
        unsafe { (*self.inner).__bindgen_anon_1.part_max as usize }
    }

    /// Get the known footprint size in bytes.
    pub fn known_footprint(&self) -> usize {
        unsafe { (*self.inner).__bindgen_anon_1.known_footprint as usize }
    }

    /// Get the total footprint size in bytes.
    pub fn total_footprint(&self) -> usize {
        unsafe { (*self.inner).__bindgen_anon_1.total_footprint as usize }
    }

    /// Check if the workspace is currently joined (mapped into the process).
    pub fn is_joined(&self) -> bool {
        unsafe { !(*self.inner).__bindgen_anon_1.wksp.is_null() }
    }
}

unsafe impl Send for Workspace {}
unsafe impl Sync for Workspace {}
