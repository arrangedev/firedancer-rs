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

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ObjectInitConfig {
    /// Size of application region for mcache objects (in bytes).
    ///
    /// The application region is used for application-specific data storage
    /// within the mcache. Set to 0 if no application region is needed.
    /// Default: 0
    pub mcache_app_sz: u64,

    /// Size of application region for dcache objects (in bytes).
    ///
    /// The application region is used for application-specific data storage
    /// within the dcache. Set to 0 if no application region is needed.
    /// Default: 0
    pub dcache_app_sz: u64,

    /// Initial sequence number for mcache and fseq objects.
    ///
    /// This is the starting sequence number used for fragment ordering
    /// and flow control. For most applications, 0 is the correct value.
    /// Default: 0
    pub initial_seq: u64,
}

impl Default for ObjectInitConfig {
    fn default() -> Self {
        Self {
            mcache_app_sz: 0,
            dcache_app_sz: 0,
            initial_seq: 0,
        }
    }
}

impl ObjectInitConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_mcache_app_sz(mut self, app_sz: u64) -> Self {
        self.mcache_app_sz = app_sz;
        self
    }

    pub fn with_dcache_app_sz(mut self, app_sz: u64) -> Self {
        self.dcache_app_sz = app_sz;
        self
    }

    /// initial sequence number for mcache and fseq objects.
    pub fn with_initial_seq(mut self, seq: u64) -> Self {
        self.initial_seq = seq;
        self
    }
}
