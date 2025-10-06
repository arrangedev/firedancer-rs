use crate::{Result, Topo, TopoError};
use core::mem;
use core::ptr;
use fd_topo_sys as sys;
use std::ffi::{CStr, CString};

/// Callback functions for topology objects
pub struct ObjectCallbacks {
    /// Object name (must match the name used when creating the object)
    pub name: String,
    /// Calculate the memory footprint required for this object
    pub footprint:
        unsafe extern "C" fn(topo: *const sys::fd_topo_t, obj: *const sys::fd_topo_obj_t) -> u64,
    /// Calculate the memory alignment required for this object
    pub align:
        unsafe extern "C" fn(topo: *const sys::fd_topo_t, obj: *const sys::fd_topo_obj_t) -> u64,
    /// Calculate loose memory requirements (optional, can be None)
    pub loose: Option<
        unsafe extern "C" fn(topo: *const sys::fd_topo_t, obj: *const sys::fd_topo_obj_t) -> u64,
    >,
    /// Initialize the object after memory allocation (optional, can be None)
    pub new:
        Option<unsafe extern "C" fn(topo: *const sys::fd_topo_t, obj: *const sys::fd_topo_obj_t)>,
}

impl ObjectCallbacks {
    pub fn new(
        name: impl Into<String>,
        footprint: unsafe extern "C" fn(
            topo: *const sys::fd_topo_t,
            obj: *const sys::fd_topo_obj_t,
        ) -> u64,
        align: unsafe extern "C" fn(
            topo: *const sys::fd_topo_t,
            obj: *const sys::fd_topo_obj_t,
        ) -> u64,
    ) -> Self {
        Self {
            name: name.into(),
            footprint,
            align,
            loose: None,
            new: None,
        }
    }

    /// loose memory callback
    pub fn with_loose(
        mut self,
        loose: unsafe extern "C" fn(
            topo: *const sys::fd_topo_t,
            obj: *const sys::fd_topo_obj_t,
        ) -> u64,
    ) -> Self {
        self.loose = Some(loose);
        self
    }

    pub fn with_new(
        mut self,
        new: unsafe extern "C" fn(topo: *const sys::fd_topo_t, obj: *const sys::fd_topo_obj_t),
    ) -> Self {
        self.new = Some(new);
        self
    }
}

/// A collection of object callbacks that can be passed to the topology builder
pub struct CallbackRegistry {
    callbacks: Vec<ObjectCallbacks>,
    // Keep the C-compatible structures alive
    c_callbacks: Vec<sys::fd_topo_obj_callbacks_t>,
    c_names: Vec<CString>,
    c_callback_ptrs: Vec<*mut sys::fd_topo_obj_callbacks_t>,
}

impl CallbackRegistry {
    /// Create a new empty callback registry
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
            c_callbacks: Vec::new(),
            c_names: Vec::new(),
            c_callback_ptrs: Vec::new(),
        }
    }

    /// Add an object callback to the registry
    pub fn add_callback(&mut self, callback: ObjectCallbacks) -> Result<()> {
        self.callbacks.push(callback);
        Ok(())
    }

    /// Finalize the registry and return a pointer suitable for fd_topob_finish
    ///
    /// This must be called after all callbacks are added and before calling build()
    pub fn finalize(&mut self) -> Result<*mut *mut sys::fd_topo_obj_callbacks_t> {
        self.c_callbacks.clear();
        self.c_names.clear();
        self.c_callback_ptrs.clear();

        for callback in &self.callbacks {
            let c_name = CString::new(callback.name.clone())?;

            let c_callback = sys::fd_topo_obj_callbacks_t {
                name: c_name.as_ptr(),
                footprint: Some(callback.footprint),
                align: Some(callback.align),
                loose: callback.loose,
                new: callback.new,
            };

            self.c_names.push(c_name);
            self.c_callbacks.push(c_callback);
        }

        // Create pointers to the C callbacks
        for c_callback in &mut self.c_callbacks {
            self.c_callback_ptrs.push(c_callback as *mut _);
        }

        // Add null terminator
        self.c_callback_ptrs.push(ptr::null_mut());

        Ok(self.c_callback_ptrs.as_mut_ptr())
    }
}

impl Default for CallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
pub struct TopoBuilder {
    inner: *mut sys::fd_topo_t,
    _backing: ptr::NonNull<u8>,
}

impl TopoBuilder {
    pub fn new(app_name: &str) -> Result<Self> {
        let c_app_name = CString::new(app_name)?;
        let topo_size = mem::size_of::<sys::fd_topo_t>();
        let align = mem::align_of::<sys::fd_topo_t>();

        let mem = unsafe {
            std::alloc::alloc(std::alloc::Layout::from_size_align(topo_size, align).unwrap())
        };

        unsafe {
            let topo_ptr = sys::fd_topob_new(mem as *mut _, c_app_name.as_ptr());
            if topo_ptr.is_null() {
                return Err(TopoError::MemoryError);
            }

            Ok(Self {
                inner: topo_ptr,
                _backing: ptr::NonNull::new(mem).unwrap(),
            })
        }
    }

    pub fn add_workspace(&mut self, name: &str) -> Result<()> {
        let c_name = CString::new(name)?;

        unsafe {
            let wksp_ptr = sys::fd_topob_wksp(self.inner, c_name.as_ptr());
            if wksp_ptr.is_null() {
                return Err(TopoError::SystemError);
            }
        }

        Ok(())
    }

    pub fn add_object(&mut self, obj_name: &str, wksp_name: &str) -> Result<()> {
        let c_obj_name = CString::new(obj_name)?;
        let c_wksp_name = CString::new(wksp_name)?;

        unsafe {
            let obj_ptr = sys::fd_topob_obj(self.inner, c_obj_name.as_ptr(), c_wksp_name.as_ptr());
            if obj_ptr.is_null() {
                return Err(TopoError::SystemError);
            }
        }

        Ok(())
    }

    pub fn add_link(
        &mut self,
        link_name: &str,
        wksp_name: &str,
        depth: usize,
        mtu: usize,
        burst: usize,
    ) -> Result<()> {
        let c_link_name = CString::new(link_name)?;
        let c_wksp_name = CString::new(wksp_name)?;

        unsafe {
            let link_ptr = sys::fd_topob_link(
                self.inner,
                c_link_name.as_ptr(),
                c_wksp_name.as_ptr(),
                depth as u64,
                mtu as u64,
                burst as u64,
            );
            if link_ptr.is_null() {
                return Err(TopoError::SystemError);
            }
        }

        Ok(())
    }

    pub fn add_tile(
        &mut self,
        tile_name: &str,
        tile_wksp: &str,
        metrics_wksp: &str,
        cpu_idx: Option<usize>,
        is_agave: bool,
        uses_keyswitch: bool,
    ) -> Result<()> {
        let c_tile_name = CString::new(tile_name)?;
        let c_tile_wksp = CString::new(tile_wksp)?;
        let c_metrics_wksp = CString::new(metrics_wksp)?;

        let cpu_idx = cpu_idx.unwrap_or(u64::MAX as usize);

        unsafe {
            let tile_ptr = sys::fd_topob_tile(
                self.inner,
                c_tile_name.as_ptr(),
                c_tile_wksp.as_ptr(),
                c_metrics_wksp.as_ptr(),
                cpu_idx as u64,
                if is_agave { 1 } else { 0 },
                if uses_keyswitch { 1 } else { 0 },
            );
            if tile_ptr.is_null() {
                return Err(TopoError::SystemError);
            }
        }

        Ok(())
    }

    /// Add an input link to a tile
    pub fn add_tile_input(
        &mut self,
        tile_name: &str,
        tile_kind_id: usize,
        fseq_wksp: &str,
        link_name: &str,
        link_kind_id: usize,
        reliable: bool,
        polled: bool,
    ) -> Result<()> {
        let c_tile_name = CString::new(tile_name)?;
        let c_fseq_wksp = CString::new(fseq_wksp)?;
        let c_link_name = CString::new(link_name)?;

        unsafe {
            sys::fd_topob_tile_in(
                self.inner,
                c_tile_name.as_ptr(),
                tile_kind_id as u64,
                c_fseq_wksp.as_ptr(),
                c_link_name.as_ptr(),
                link_kind_id as u64,
                if reliable { 1 } else { 0 },
                if polled { 1 } else { 0 },
            );
        }

        Ok(())
    }

    /// Add an output link to a tile
    pub fn add_tile_output(
        &mut self,
        tile_name: &str,
        tile_kind_id: usize,
        link_name: &str,
        link_kind_id: usize,
    ) -> Result<()> {
        let c_tile_name = CString::new(tile_name)?;
        let c_link_name = CString::new(link_name)?;

        unsafe {
            sys::fd_topob_tile_out(
                self.inner,
                c_tile_name.as_ptr(),
                tile_kind_id as u64,
                c_link_name.as_ptr(),
                link_kind_id as u64,
            );
        }

        Ok(())
    }

    /// Automatically layout tiles onto CPUs
    pub fn auto_layout(&mut self, reserve_agave_cores: bool) -> Result<()> {
        unsafe {
            sys::fd_topob_auto_layout(self.inner, if reserve_agave_cores { 1 } else { 0 });
        }
        Ok(())
    }

    pub fn build(self, callbacks: *mut *mut sys::fd_topo_obj_callbacks_t) -> Result<Topo> {
        unsafe {
            sys::fd_topob_finish(self.inner, callbacks);
            let topo = Topo::from_raw(self.inner, true);

            // we're transferring ownership, so we can use forget here
            std::mem::forget(self);

            Ok(topo)
        }
    }
}

impl Drop for TopoBuilder {
    fn drop(&mut self) {
        // we don't want to free the memory
    }
}
