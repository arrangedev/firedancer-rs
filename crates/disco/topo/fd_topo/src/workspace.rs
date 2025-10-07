use core::ffi::CStr;
use fd_topo_sys as sys;

use crate::types::{PageSize, _TopoInternal, _WorkspaceInternal};

/// A memory management component that is comprised of multiple orchestrated
/// tiles, objects for them to access, and sits on top of one or more
/// memory mapped gigantic or huge pages mounted to the hugetlbfs.
#[repr(C)]
pub struct Workspace {
    inner: *mut _WorkspaceInternal,
}

impl Workspace {
    pub fn create(&mut self, topo: *mut _TopoInternal, update_existing: bool) -> crate::Result<()> {
        unsafe {
            let result = sys::fd_topo_create_workspace(
                topo,
                self.inner,
                if update_existing { 1 } else { 0 },
            );
            if result != 0 {
                return Err(crate::TopoError::SystemError);
            }
        }
        Ok(())
    }

    /// Create an anonymous workspace
    ///
    /// Anonymous workspaces don't require shared memory setup.
    /// They exist only in memory and are automatically cleaned up when the process exits.
    pub fn create_anonymous(
        &mut self,
        topo: *mut _TopoInternal,
        page_sz: Option<PageSize>,
    ) -> crate::Result<()> {
        unsafe {
            let original_page_cnt = (*self.inner).__bindgen_anon_1.page_cnt;
            let original_page_sz = (*self.inner).__bindgen_anon_1.page_sz;
            let _total_footprint = (*self.inner).__bindgen_anon_1.total_footprint;

            if original_page_cnt == 0 || original_page_sz == 0 {
                return Err(crate::TopoError::SystemError);
            }

            let wksp_name = CStr::from_ptr((*self.inner).name.as_ptr());
            let app_name = CStr::from_ptr((*topo).app_name.as_ptr());

            let combined_name = format!(
                "{}_{}\0",
                app_name.to_str().unwrap_or("app"),
                wksp_name.to_str().unwrap_or("wksp")
            );

            let cpu_idx = sys::fd_shmem_cpu_idx((*self.inner).numa_idx);
            let requested_page_sz = match page_sz {
                Some(PageSize::Normal) => sys::FD_SHMEM_NORMAL_PAGE_SZ as u64,
                Some(PageSize::Huge) => sys::FD_SHMEM_HUGE_PAGE_SZ as u64,
                Some(PageSize::Gigantic) => sys::FD_SHMEM_GIGANTIC_PAGE_SZ as u64,
                None => original_page_sz,
            };

            let total_size = original_page_cnt * original_page_sz;
            let requested_page_cnt = (total_size + requested_page_sz - 1) / requested_page_sz;
            (*self.inner).__bindgen_anon_1.page_sz = requested_page_sz;
            (*self.inner).__bindgen_anon_1.page_cnt = requested_page_cnt;

            let wksp_join = sys::fd_wksp_new_anon(
                combined_name.as_ptr() as *const i8,
                requested_page_sz,
                1, // sub_cnt
                &requested_page_cnt as *const u64,
                &cpu_idx as *const u64,
                0, // seed
                (*self.inner).__bindgen_anon_1.part_max,
            );

            if wksp_join.is_null() {
                let page_type = match requested_page_sz {
                    x if x == sys::FD_SHMEM_NORMAL_PAGE_SZ as u64 => "normal (4KB)",
                    x if x == sys::FD_SHMEM_HUGE_PAGE_SZ as u64 => "huge (2MB)",
                    x if x == sys::FD_SHMEM_GIGANTIC_PAGE_SZ as u64 => "gigantic (1GB)",
                    _ => "unknown",
                };

                panic!(
                    "Failed to create anonymous workspace '{}' with {} pages.",
                    wksp_name.to_str().unwrap_or("unknown"),
                    page_type
                );
            }

            (*self.inner).__bindgen_anon_1.wksp = wksp_join;
        }
        Ok(())
    }

    /// Create a workspace wrapper from a raw pointer.
    ///
    /// SAFETY: caller must ensure that `ptr` is a valid pointer to an initialized
    /// `fd_topo_wksp_t` that remains valid for the lifetime of this `Workspace`.
    pub unsafe fn from_raw(ptr: *mut _WorkspaceInternal) -> Self {
        Self { inner: ptr }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const _WorkspaceInternal {
        self.inner
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut _WorkspaceInternal {
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
    pub fn numa_idx(&self) -> usize {
        unsafe { (*self.inner).numa_idx as usize }
    }

    /// if the workspace uses locked pages
    #[inline]
    pub fn is_locked(&self) -> bool {
        unsafe { (*self.inner).is_locked != 0 }
    }

    /// page size for this workspace
    #[inline]
    pub fn page_sz(&self) -> usize {
        unsafe { (*self.inner).__bindgen_anon_1.page_sz as usize }
    }

    /// number of pages in this workspace
    #[inline]
    pub fn page_cnt(&self) -> usize {
        unsafe { (*self.inner).__bindgen_anon_1.page_cnt as usize }
    }

    #[inline]
    pub fn partition_max(&self) -> usize {
        unsafe { (*self.inner).__bindgen_anon_1.part_max as usize }
    }

    #[inline]
    pub fn known_footprint(&self) -> usize {
        unsafe { (*self.inner).__bindgen_anon_1.known_footprint as usize }
    }

    #[inline]
    pub fn total_footprint(&self) -> usize {
        unsafe { (*self.inner).__bindgen_anon_1.total_footprint as usize }
    }

    /// if the workspace is currently mapped into the process (referred to as a "join")
    #[inline]
    pub fn is_joined(&self) -> bool {
        unsafe { !(*self.inner).__bindgen_anon_1.wksp.is_null() }
    }
}

unsafe impl Send for Workspace {}
unsafe impl Sync for Workspace {}
