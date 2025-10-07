//! Safe API for `fd_topo_sys`
//!
//! `topo` manages the structure workspaces, tiles, and links between them.
//!
//! # Concepts
//!
//! - Topology (`Topo`): The overall structure describing workspaces, tiles, and links
//! - Workspace (`Workspace`): Memory management structures backed by huge pages
//! - Tile (`Tile`): Individual processes/threads of execution
//! - Link (`Link`): Communication channels between tiles with mcache/dcache
//! - Object (`Object`): Memory objects within workspaces

use core::ffi::CStr;
use fd_topo_sys as sys;
use std::ffi::CString;
use std::sync::Once;

pub mod builder;
pub mod cpu_topo;
pub mod error;
pub mod link;
pub mod object;
pub mod tile;
pub mod workspace;

pub use builder::{CallbackRegistry, ObjectCallbacks, TopoBuilder};
pub use cpu_topo::CpuTopology;
pub use error::{Result, TopoError};
pub use link::Link;
pub use object::Object;
pub use tile::Tile;
pub use workspace::Workspace;

static INIT: Once = Once::new();

pub unsafe fn init(program_name: &'static CStr) {
    INIT.call_once(|| {
        let program_name = program_name.as_ptr() as *mut i8;
        let mut argv_array = [program_name, core::ptr::null_mut()];
        let mut argc = 1i32;
        let mut argv = argv_array.as_mut_ptr();
        sys::fd_boot(&mut argc, &mut argv);
    });
}

pub unsafe fn shutdown() {
    sys::fd_halt();
}

pub type TopoCallbackFn<R> =
    unsafe extern "C" fn(topo: *const sys::fd_topo_t, obj: *const sys::fd_topo_obj_t) -> R;

pub const MAX_WORKSPACES: usize = sys::FD_TOPO_MAX_WKSPS as usize;
pub const MAX_LINKS: usize = sys::FD_TOPO_MAX_LINKS as usize;
pub const MAX_TILES: usize = sys::FD_TOPO_MAX_TILES as usize;
pub const MAX_OBJECTS: usize = sys::FD_TOPO_MAX_OBJS as usize;
pub const MAX_TILE_IN_LINKS: usize = sys::FD_TOPO_MAX_TILE_IN_LINKS as usize;
pub const MAX_TILE_OUT_LINKS: usize = sys::FD_TOPO_MAX_TILE_OUT_LINKS as usize;
pub const MAX_TILE_OBJECTS: usize = sys::FD_TOPO_MAX_TILE_OBJS as usize;

pub struct Topo {
    inner: *mut sys::fd_topo_t,
    owned: bool,
}

impl Topo {
    /// SAFETY: caller must ensure that:
    /// - `ptr` is a valid pointer to an initialized `fd_topo_t`
    /// - Memory pointed to by `ptr` remains valid for the lifetime of this `Topo`
    /// - If `owned` is true, this `Topo` will take ownership and free the memory on drop
    pub unsafe fn from_raw(ptr: *mut sys::fd_topo_t, owned: bool) -> Self {
        Self { inner: ptr, owned }
    }

    /// SAFETY: returned pointer is valid only as long as this `Topo` exists.
    pub fn as_ptr(&self) -> *const sys::fd_topo_t {
        self.inner
    }

    /// SAFETY: returned pointer is valid only as long as this `Topo` exists.
    pub fn as_mut_ptr(&mut self) -> *mut sys::fd_topo_t {
        self.inner
    }

    #[inline]
    pub fn app_name(&self) -> &str {
        unsafe {
            let topo = &*self.inner;
            CStr::from_ptr(topo.app_name.as_ptr())
                .to_str()
                .unwrap_or("")
        }
    }

    #[inline]
    pub fn workspace_cnt(&self) -> usize {
        unsafe { (*self.inner).wksp_cnt as usize }
    }

    #[inline]
    pub fn link_cnt(&self) -> usize {
        unsafe { (*self.inner).link_cnt as usize }
    }

    #[inline]
    pub fn tile_cnt(&self) -> usize {
        unsafe { (*self.inner).tile_cnt as usize }
    }

    #[inline]
    pub fn object_cnt(&self) -> usize {
        unsafe { (*self.inner).obj_cnt as usize }
    }

    #[inline]
    pub fn find_wksp(&self, name: &str) -> Option<usize> {
        let c_name = CString::new(name).ok()?;
        unsafe {
            let result = sys::fd_topo_find_wksp(self.inner, c_name.as_ptr());
            if result == u64::MAX {
                None
            } else {
                Some(result as usize)
            }
        }
    }

    #[inline]
    pub fn find_tile(&self, name: &str, kind_id: usize) -> Option<usize> {
        let c_name = CString::new(name).ok()?;
        unsafe {
            let result = sys::fd_topo_find_tile(self.inner, c_name.as_ptr(), kind_id as u64);
            if result == u64::MAX {
                None
            } else {
                Some(result as usize)
            }
        }
    }

    #[inline]
    pub fn find_link(&self, name: &str, kind_id: usize) -> Option<usize> {
        let c_name = CString::new(name).ok()?;
        unsafe {
            let result = sys::fd_topo_find_link(self.inner, c_name.as_ptr(), kind_id as u64);
            if result == u64::MAX {
                None
            } else {
                Some(result as usize)
            }
        }
    }

    #[inline]
    pub fn tile_name_cnt(&self, name: &str) -> usize {
        let c_name = CString::new(name).unwrap_or_default();
        unsafe { sys::fd_topo_tile_name_cnt(self.inner, c_name.as_ptr()) as usize }
    }

    #[inline]
    pub fn max_tile_mlock(&self) -> usize {
        unsafe { sys::fd_topo_mlock_max_tile(self.inner) as usize }
    }

    #[inline]
    pub fn total_mlock(&self) -> usize {
        unsafe { sys::fd_topo_mlock(self.inner) as usize }
    }

    #[inline]
    pub fn gigantic_page_cnt(&self, numa_idx: usize) -> usize {
        unsafe { sys::fd_topo_gigantic_page_cnt(self.inner, numa_idx as u64) as usize }
    }

    #[inline]
    pub fn huge_page_cnt(&self, numa_idx: usize, include_anonymous: bool) -> usize {
        unsafe {
            sys::fd_topo_huge_page_cnt(
                self.inner,
                numa_idx as u64,
                if include_anonymous { 1 } else { 0 },
            ) as usize
        }
    }

    #[inline]
    pub fn join_wksps(&mut self, read_only: bool) -> Result<()> {
        unsafe {
            let mode = if read_only {
                // FD_SHMEM_JOIN_MODE_READ_ONLY = 0
                0
            } else {
                // FD_SHMEM_JOIN_MODE_READ_WRITE = 1
                1
            };
            sys::fd_topo_join_workspaces(self.inner, mode);
        }
        Ok(())
    }

    #[inline]
    pub fn leave_wksps(&mut self) {
        unsafe {
            sys::fd_topo_leave_workspaces(self.inner);
        }
    }

    #[inline]
    pub fn fill(&mut self) {
        unsafe {
            sys::fd_topo_fill(self.inner);
        }
    }

    #[inline]
    pub fn print_to_stdout(&self) {
        unsafe {
            sys::fd_topo_print_log(1, self.inner as *mut _);
        }
    }

    #[inline]
    pub fn print_to_log(&self) {
        unsafe {
            sys::fd_topo_print_log(0, self.inner as *mut _);
        }
    }

    #[cfg(target_os = "linux")]
    #[inline]
    pub fn run_tile(&mut self, tile_id: usize, uid: u32, gid: u32) -> Result<()> {
        if tile_id >= self.tile_cnt() {
            return Err(TopoError::NotFound);
        }

        unsafe {
            let tile_ptr = &mut (*self.inner).tiles[tile_id] as *mut sys::fd_topo_tile_t;
            sys::fd_topo_run_tile(
                self.inner,
                tile_ptr,
                1, // sandbox enabled
                0, // don't keep controlling terminal
                0, // not dumpable
                uid,
                gid,
                -1,                    // no special fd
                core::ptr::null_mut(), // no wait
                core::ptr::null_mut(), // no debugger
                core::ptr::null_mut(), // tile_run function pointer - will use default
            );
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[inline]
    pub fn run_all_tiles(&mut self, uid: u32, gid: u32) -> Result<()> {
        for tile_id in 0..self.tile_cnt() {
            self.run_tile(tile_id, uid, gid)?;
        }
        Ok(())
    }

    #[inline]
    pub fn join_tile_wksps(&mut self, tile_id: usize) -> Result<()> {
        if tile_id >= self.tile_cnt() {
            return Err(TopoError::NotFound);
        }

        unsafe {
            let tile_ptr = &mut (*self.inner).tiles[tile_id] as *mut sys::fd_topo_tile_t;
            sys::fd_topo_join_tile_workspaces(self.inner, tile_ptr);
        }

        Ok(())
    }

    #[inline]
    pub fn fill_tile(&mut self, tile_id: usize) -> Result<()> {
        if tile_id >= self.tile_cnt() {
            return Err(TopoError::NotFound);
        }

        unsafe {
            let tile_ptr = &mut (*self.inner).tiles[tile_id] as *mut sys::fd_topo_tile_t;
            sys::fd_topo_fill_tile(self.inner, tile_ptr);
        }

        Ok(())
    }
}

impl Drop for Topo {
    fn drop(&mut self) {
        if self.owned && !self.inner.is_null() {
            self.leave_wksps();
            // lib doesn't provide a cleanup function
        }
    }
}

unsafe impl Send for Topo {}
unsafe impl Sync for Topo {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_topology() {
        let cpu_topo = CpuTopology::new_simple(c"tch_fd_topo").expect("Failed to get CPU topology");
        assert!(cpu_topo.cpu_count() > 0, "Should have at least one CPU");
        assert!(
            cpu_topo.numa_node_count() > 0,
            "Should have at least one NUMA node"
        );
    }
}
