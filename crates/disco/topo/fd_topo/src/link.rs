use crate::types::_LinkInternal;
use core::ffi::CStr;

/// Links represent an SPSC/SPMC mcache in a workspace. A link may optionally also have a dcache,
/// that holds fragments referred to by the mcache entries.
#[repr(C)]
pub struct Link {
    inner: *mut _LinkInternal,
}

impl Link {
    /// SAFETY: caller must ensure that `ptr` is a valid pointer to an initialized
    /// `fd_topo_link_t` that remains valid for the lifetime of this `Link`.
    #[inline]
    pub unsafe fn from_raw(ptr: *mut _LinkInternal) -> Self {
        Self { inner: ptr }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const _LinkInternal {
        self.inner
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut _LinkInternal {
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

    /// depth of the mcache representing this link
    #[inline]
    pub fn depth(&self) -> usize {
        unsafe { (*self.inner).depth as usize }
    }

    /// MTU of data fragments in the mcache (0 if no dcache)
    #[inline]
    pub fn mtu(&self) -> usize {
        unsafe { (*self.inner).mtu as usize }
    }

    /// maximum burst size
    #[inline]
    pub fn burst(&self) -> usize {
        unsafe { (*self.inner).burst as usize }
    }

    #[inline]
    pub fn mcache_obj_id(&self) -> usize {
        unsafe { (*self.inner).mcache_obj_id as usize }
    }

    #[inline]
    pub fn dcache_object_id(&self) -> usize {
        unsafe { (*self.inner).dcache_obj_id as usize }
    }

    /// does this link have a dcache?
    #[inline]
    pub fn has_dcache(&self) -> bool {
        self.mtu() > 0
    }

    /// does the link permit having no consumers?
    #[inline]
    pub fn permit_no_consumers(&self) -> bool {
        unsafe { (*self.inner).permit_no_consumers() != 0 }
    }

    /// does the link permit having no producers?
    #[inline]
    pub fn permit_no_producers(&self) -> bool {
        unsafe { (*self.inner).permit_no_producers() != 0 }
    }

    /// is the mcache currently mapped?
    #[inline]
    pub fn is_mcache_mapped(&self) -> bool {
        unsafe { !(*self.inner).__bindgen_anon_1.mcache.is_null() }
    }

    /// is the dcache currently mapped?
    #[inline]
    pub fn is_dcache_mapped(&self) -> bool {
        unsafe { !(*self.inner).__bindgen_anon_1.dcache.is_null() }
    }
}

unsafe impl Send for Link {}
unsafe impl Sync for Link {}
