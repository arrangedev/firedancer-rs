use core::ffi::CStr;
use fd_topo_sys as sys;

/// Objects represent a memory allocation within a workspace
/// and are both used as units to track allocations, as well
/// as for access control for tiles in the workspace.
#[repr(C)]
pub struct Object {
    inner: *mut sys::fd_topo_obj_t,
}

impl Object {
    /// SAFETY: caller must ensure that `ptr` is a valid pointer to an initialized
    /// `fd_topo_obj_t` that remains valid for the lifetime of this `Object`.
    pub unsafe fn from_raw(ptr: *mut sys::fd_topo_obj_t) -> Self {
        Self { inner: ptr }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const sys::fd_topo_obj_t {
        self.inner
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut sys::fd_topo_obj_t {
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
    pub fn wksp_id(&self) -> usize {
        unsafe { (*self.inner).wksp_id as usize }
    }

    /// get the offset within the workspace
    #[inline]
    pub fn offset(&self) -> usize {
        unsafe { (*self.inner).offset as usize }
    }

    /// get the footprint/size of the object
    #[inline]
    pub fn footprint(&self) -> usize {
        unsafe { (*self.inner).footprint as usize }
    }
}

unsafe impl Send for Object {}
unsafe impl Sync for Object {}
