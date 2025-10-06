//! Object management for Firedancer topology.
//!
//! Objects represent memory allocations within workspaces.

use fd_topo_sys as sys;
use std::ffi::CStr;

/// Represents an object in the topology.
pub struct Object {
    inner: *mut sys::fd_topo_obj_t,
}

impl Object {
    /// Create an object wrapper from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ptr` is a valid pointer to an initialized
    /// `fd_topo_obj_t` that remains valid for the lifetime of this `Object`.
    pub unsafe fn from_raw(ptr: *mut sys::fd_topo_obj_t) -> Self {
        Self { inner: ptr }
    }

    /// Get the raw pointer to the underlying object.
    pub fn as_ptr(&self) -> *const sys::fd_topo_obj_t {
        self.inner
    }

    /// Get a mutable raw pointer to the underlying object.
    pub fn as_mut_ptr(&mut self) -> *mut sys::fd_topo_obj_t {
        self.inner
    }

    /// Get the object ID.
    pub fn id(&self) -> usize {
        unsafe { (*self.inner).id as usize }
    }

    /// Get the object name.
    pub fn name(&self) -> &str {
        unsafe {
            CStr::from_ptr((*self.inner).name.as_ptr())
                .to_str()
                .unwrap_or("")
        }
    }

    /// Get the workspace ID this object belongs to.
    pub fn workspace_id(&self) -> usize {
        unsafe { (*self.inner).wksp_id as usize }
    }

    /// Get the offset within the workspace.
    pub fn offset(&self) -> usize {
        unsafe { (*self.inner).offset as usize }
    }

    /// Get the footprint (size) of the object.
    pub fn footprint(&self) -> usize {
        unsafe { (*self.inner).footprint as usize }
    }
}

unsafe impl Send for Object {}
unsafe impl Sync for Object {}
