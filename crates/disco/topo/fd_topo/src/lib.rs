//! Safe API for `fd_topo_sys`
//!
//! `topo` manages the structure workspaces, tiles, and links between them.
//!
//! - Topology: Overarching structure describing workspaces, tiles, and links
//!
//! - Workspace: Group of one or more tiles with corresponding links and objects,
//! handling memory management and backed by huge pages
//!
//! - Tile: Individual processes/threads of execution
//!
//! - Link: Input/Output message channels between tiles using mcaches/dcaches respectively
//!
//! - Object: Blobs/data structures stored within workspace memory

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
pub mod types;
pub mod workspace;

pub use builder::{ObjectCallbacks, TopoBuilder, TopologyCallbacks};
pub use cpu_topo::CpuTopology;
pub use error::{Result, TopoError};
pub use link::Link;
pub use object::Object;
pub use tile::{Tile, TileRunner, TileRunnerRegistry};
pub use types::PageSize;
pub use workspace::Workspace;

use crate::{
    object::ObjectInitConfig,
    types::{
        ActiveTile, ActiveTopology, TopoCallbackFn, _TileInternal, _TileRunnerInternal,
        _TopoInternal,
    },
};

pub const MAX_WORKSPACES: usize = sys::FD_TOPO_MAX_WKSPS as usize;
pub const MAX_LINKS: usize = sys::FD_TOPO_MAX_LINKS as usize;
pub const MAX_TILES: usize = sys::FD_TOPO_MAX_TILES as usize;
pub const MAX_OBJECTS: usize = sys::FD_TOPO_MAX_OBJS as usize;
pub const MAX_TILE_IN_LINKS: usize = sys::FD_TOPO_MAX_TILE_IN_LINKS as usize;
pub const MAX_TILE_OUT_LINKS: usize = sys::FD_TOPO_MAX_TILE_OUT_LINKS as usize;
pub const MAX_TILE_OBJECTS: usize = sys::FD_TOPO_MAX_TILE_OBJS as usize;

static INIT: Once = Once::new();

#[inline]
pub unsafe fn init(program_name: &'static CStr) {
    INIT.call_once(|| {
        let program_name = program_name.as_ptr() as *mut i8;
        let mut argv_array = [program_name, core::ptr::null_mut()];
        let mut argc = 1i32;
        let mut argv = argv_array.as_mut_ptr();
        sys::fd_boot(&mut argc, &mut argv);
    });
}

#[inline]
pub unsafe fn shutdown() {
    sys::fd_halt();
}

#[repr(C)]
pub struct Topo {
    inner: *mut _TopoInternal,
    owned: bool,
}

impl Topo {
    /// SAFETY: caller must ensure that:
    /// - ptr is a valid pointer to an initialized `fd_topo_t`
    /// - Memory pointed to by `ptr` remains valid for the lifetime of this instance
    /// - If `owned` is true, this instance will take ownership and free the memory on drop
    #[inline]
    pub unsafe fn from_raw(ptr: *mut _TopoInternal, owned: bool) -> Self {
        Self { inner: ptr, owned }
    }

    /// SAFETY: returned pointer is valid only as long as this instance exists.
    #[inline]
    pub fn as_ptr(&self) -> *const _TopoInternal {
        self.inner
    }

    /// SAFETY: returned pointer is valid only as long as this instance exists.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut _TopoInternal {
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

    /// Initialize all objects in the topo wksps.
    /// Must be called after wksps are created but before fill()
    pub fn init_objects(&mut self) -> Result<()> {
        self.init_objects_with_config(&ObjectInitConfig::default())
    }

    /// Initialize all objects in the topo wksps with a custom config.
    /// Must be called after wksps are created but before fill()
    pub fn init_objects_with_config(&mut self, config: &ObjectInitConfig) -> Result<()> {
        unsafe {
            let topo = &*self.inner;
            for i in 0..topo.obj_cnt {
                let obj = &topo.objs[i as usize];
                let obj_laddr = sys::fd_topo_obj_laddr(self.inner, obj.id);

                if obj_laddr.is_null() {
                    continue;
                }

                let obj_name = CStr::from_ptr(obj.name.as_ptr());
                let obj_name_str = obj_name.to_str().unwrap_or("");

                match obj_name_str {
                    "mcache" => {
                        let mut depth = 1024u64;
                        for j in 0..topo.link_cnt {
                            let link = &topo.links[j as usize];
                            if link.mcache_obj_id == obj.id {
                                depth = link.depth;
                                break;
                            }
                        }
                        sys::fd_mcache_new(
                            obj_laddr,
                            depth,
                            config.mcache_app_sz,
                            config.initial_seq,
                        );
                    }
                    "dcache" => {
                        let mut def_data_sz = 1024u64 * 1024u64;
                        for j in 0..topo.link_cnt {
                            let link = &topo.links[j as usize];
                            if link.dcache_obj_id == obj.id {
                                def_data_sz =
                                    sys::fd_dcache_req_data_sz(link.mtu, link.depth, link.burst, 1);
                                break;
                            }
                        }
                        sys::fd_dcache_new(obj_laddr, def_data_sz, config.dcache_app_sz);
                    }
                    "fseq" => {
                        sys::fd_fseq_new(obj_laddr, config.initial_seq);
                    }
                    "metrics" => {
                        let mut in_link_cnt = 0u64;
                        let mut out_link_cnt = 0u64;
                        for j in 0..topo.tile_cnt {
                            let tile = &topo.tiles[j as usize];
                            if tile.metrics_obj_id == obj.id {
                                in_link_cnt = tile.in_cnt;
                                out_link_cnt = tile.out_cnt;
                                break;
                            }
                        }
                        sys::fd_metrics_new(obj_laddr, in_link_cnt, out_link_cnt);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
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
    pub fn run_tile(
        &mut self,
        tile_id: usize,
        uid: u32,
        gid: u32,
        tile_runner: Option<&_TileRunnerInternal>,
    ) -> Result<()> {
        if tile_id >= self.tile_cnt() {
            return Err(TopoError::NotFound);
        }

        unsafe {
            let tile_ptr = &mut (*self.inner).tiles[tile_id] as *mut _TileInternal;
            let runner_ptr = tile_runner
                .map(|r| r as *const _TileRunnerInternal)
                .unwrap_or(core::ptr::null());

            sys::fd_topo_run_tile(
                self.inner,
                tile_ptr,
                1, // sandbox
                0, // no controlling terminal
                0, // no dump
                uid,
                gid,
                -1,                    // no special fd
                core::ptr::null_mut(), // no wait
                core::ptr::null_mut(), // no debugger
                runner_ptr as *mut _TileRunnerInternal,
            );
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[inline]
    pub fn run_all_tiles(
        &mut self,
        uid: u32,
        gid: u32,
        tile_registry: &TileRunnerRegistry,
    ) -> Result<()> {
        for tile_id in 0..self.tile_cnt() {
            let tile_name = unsafe {
                let tile_ptr = &(*self.inner).tiles[tile_id];
                CStr::from_ptr(tile_ptr.name.as_ptr())
                    .to_str()
                    .unwrap_or("")
            };

            let runner = tile_registry.find_runner(tile_name);
            if runner.is_none() {
                continue;
            }

            self.run_tile(tile_id, uid, gid, runner)?;
        }
        Ok(())
    }

    #[inline]
    pub fn join_tile_wksps(&mut self, tile_id: usize) -> Result<()> {
        if tile_id >= self.tile_cnt() {
            return Err(TopoError::NotFound);
        }

        unsafe {
            let tile_ptr = &mut (*self.inner).tiles[tile_id] as *mut _TileInternal;
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
            let tile_ptr = &mut (*self.inner).tiles[tile_id] as *mut _TileInternal;
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
