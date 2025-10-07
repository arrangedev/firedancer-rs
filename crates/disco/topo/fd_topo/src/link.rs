//! Link management for Firedancer topology.
//!
//! A link is an mcache in a workspace that has one producer and one or
//! more consumers. A link may optionally also have a dcache, that holds
//! fragments referred to by the mcache entries.

use core::ffi::CStr;
use fd_topo_sys as sys;

pub struct Link {
    inner: *mut sys::fd_topo_link_t,
}

impl Link {
    /// Create a link wrapper from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ptr` is a valid pointer to an initialized
    /// `fd_topo_link_t` that remains valid for the lifetime of this `Link`.
    pub unsafe fn from_raw(ptr: *mut sys::fd_topo_link_t) -> Self {
        Self { inner: ptr }
    }

    /// Get the raw pointer to the underlying link.
    pub fn as_ptr(&self) -> *const sys::fd_topo_link_t {
        self.inner
    }

    /// Get a mutable raw pointer to the underlying link.
    pub fn as_mut_ptr(&mut self) -> *mut sys::fd_topo_link_t {
        self.inner
    }

    /// Get the link ID.
    pub fn id(&self) -> usize {
        unsafe { (*self.inner).id as usize }
    }

    /// Get the link name.
    pub fn name(&self) -> &str {
        unsafe {
            CStr::from_ptr((*self.inner).name.as_ptr())
                .to_str()
                .unwrap_or("")
        }
    }

    /// Get the link kind ID.
    pub fn kind_id(&self) -> usize {
        unsafe { (*self.inner).kind_id as usize }
    }

    /// Get the depth of the mcache representing the link.
    pub fn depth(&self) -> usize {
        unsafe { (*self.inner).depth as usize }
    }

    /// Get the MTU of data fragments in the mcache.
    /// Returns 0 if there is no dcache.
    pub fn mtu(&self) -> usize {
        unsafe { (*self.inner).mtu as usize }
    }

    /// Get the maximum burst size.
    pub fn burst(&self) -> usize {
        unsafe { (*self.inner).burst as usize }
    }

    /// Get the mcache object ID.
    pub fn mcache_object_id(&self) -> usize {
        unsafe { (*self.inner).mcache_obj_id as usize }
    }

    /// Get the dcache object ID.
    pub fn dcache_object_id(&self) -> usize {
        unsafe { (*self.inner).dcache_obj_id as usize }
    }

    /// Check if this link has a dcache.
    pub fn has_dcache(&self) -> bool {
        self.mtu() > 0
    }

    /// Check if the link permits having no consumers.
    pub fn permit_no_consumers(&self) -> bool {
        unsafe { (*self.inner).permit_no_consumers() != 0 }
    }

    /// Check if the link permits having no producers.
    pub fn permit_no_producers(&self) -> bool {
        unsafe { (*self.inner).permit_no_producers() != 0 }
    }

    /// Check if the mcache is currently mapped.
    pub fn is_mcache_mapped(&self) -> bool {
        unsafe { !(*self.inner).__bindgen_anon_1.mcache.is_null() }
    }

    /// Check if the dcache is currently mapped.
    pub fn is_dcache_mapped(&self) -> bool {
        unsafe { !(*self.inner).__bindgen_anon_1.dcache.is_null() }
    }
}

unsafe impl Send for Link {}
unsafe impl Sync for Link {}
