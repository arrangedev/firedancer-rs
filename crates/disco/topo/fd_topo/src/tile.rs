use crate::{
    types::{
        ActiveTileRunner, PopulateAllowedFdsFn, PopulateAllowedSeccompFn, TileAnonymousFn,
        TileContextFn, TileRunnerFn, _TileInternal, _TileRunnerInternal,
    },
    Result,
};
use core::ffi::CStr;
use fd_topo_sys as sys;

const SCRATCH_BUF_SZ: usize = 4096;

/// A unique process spawned within a workspace, representing
/// one thread of execution. All tiles are sandboxed to their
/// own process for security reasons.
#[repr(C)]
pub struct Tile {
    inner: *mut _TileInternal,
}

impl Tile {
    /// SAFETY: The caller must ensure that `ptr` is a valid pointer to an initialized
    /// `fd_topo_tile_t` that remains valid for the lifetime of this `Tile`.
    pub unsafe fn from_raw(ptr: *mut _TileInternal) -> Self {
        Self { inner: ptr }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const _TileInternal {
        self.inner
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut _TileInternal {
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

    /// is this an agave tile?
    #[inline]
    pub fn is_agave(&self) -> bool {
        unsafe { (*self.inner).is_agave != 0 }
    }

    /// is the tile allowed to shutdown gracefully?
    #[inline]
    pub fn allow_shutdown(&self) -> bool {
        unsafe { (*self.inner).allow_shutdown != 0 }
    }

    /// cpu index this tile is pinned to -- will return `None` if floating
    #[inline]
    pub fn cpu_idx(&self) -> Option<usize> {
        unsafe {
            let cpu_idx = (*self.inner).cpu_idx;
            if cpu_idx == u64::MAX {
                None
            } else {
                Some(cpu_idx as usize)
            }
        }
    }

    /// number of input links
    #[inline]
    pub fn input_cnt(&self) -> usize {
        unsafe { (*self.inner).in_cnt as usize }
    }

    /// number of output links
    #[inline]
    pub fn output_cnt(&self) -> usize {
        unsafe { (*self.inner).out_cnt as usize }
    }

    #[inline]
    pub fn input_link_ids(&self) -> Vec<usize> {
        unsafe {
            let count = (*self.inner).in_cnt as usize;
            let mut ids = Vec::with_capacity(count);
            for i in 0..count {
                ids.push((*self.inner).in_link_id[i] as usize);
            }
            ids
        }
    }

    #[inline]
    pub fn output_link_ids(&self) -> Vec<usize> {
        unsafe {
            let count = (*self.inner).out_cnt as usize;
            let mut ids = Vec::with_capacity(count);
            for i in 0..count {
                ids.push((*self.inner).out_link_id[i] as usize);
            }
            ids
        }
    }

    /// is a specific input link reliable?
    #[inline]
    pub fn is_input_link_reliable(&self, index: usize) -> Option<bool> {
        unsafe {
            if index < (*self.inner).in_cnt as usize {
                Some((*self.inner).in_link_reliable[index] != 0)
            } else {
                None
            }
        }
    }

    /// is a specific input link polled?
    #[inline]
    pub fn is_input_link_polled(&self, index: usize) -> Option<bool> {
        unsafe {
            if index < (*self.inner).in_cnt as usize {
                Some((*self.inner).in_link_poll[index] != 0)
            } else {
                None
            }
        }
    }

    #[inline]
    pub fn tile_obj_id(&self) -> usize {
        unsafe { (*self.inner).tile_obj_id as usize }
    }

    #[inline]
    pub fn metrics_obj_id(&self) -> usize {
        unsafe { (*self.inner).metrics_obj_id as usize }
    }

    #[inline]
    pub fn keyswitch_obj_id(&self) -> usize {
        unsafe { (*self.inner).keyswitch_obj_id as usize }
    }

    /// number of objects this tile uses
    #[inline]
    pub fn uses_object_cnt(&self) -> usize {
        unsafe { (*self.inner).uses_obj_cnt as usize }
    }

    /// Get input link by index
    #[inline]
    pub fn input_link(&self, topo: &crate::Topo, index: usize) -> Option<crate::Link> {
        if index >= self.input_cnt() {
            return None;
        }
        unsafe {
            let link_id = (*self.inner).in_link_id[index] as usize;
            let topo_ptr = topo.as_ptr() as *mut crate::types::_TopoInternal;
            let link_ptr = &mut (*topo_ptr).links[link_id] as *mut crate::types::_LinkInternal;
            Some(crate::Link::from_raw(link_ptr))
        }
    }

    /// Get output link by index
    #[inline]
    pub fn output_link(&self, topo: &crate::Topo, index: usize) -> Option<crate::Link> {
        if index >= self.output_cnt() {
            return None;
        }
        unsafe {
            let link_id = (*self.inner).out_link_id[index] as usize;
            let topo_ptr = topo.as_ptr() as *mut crate::types::_TopoInternal;
            let link_ptr = &mut (*topo_ptr).links[link_id] as *mut crate::types::_LinkInternal;
            Some(crate::Link::from_raw(link_ptr))
        }
    }

    /// Get mcache pointer for an input link
    #[inline]
    pub unsafe fn input_mcache(&self, topo: &crate::Topo, index: usize) -> Option<*mut u8> {
        let link = self.input_link(topo, index)?;
        let topo_ptr = topo.as_ptr();
        let mcache_obj_id = link.mcache_obj_id();
        let obj = &(*topo_ptr).objs[mcache_obj_id];
        let mcache_laddr = sys::fd_topo_obj_laddr(topo_ptr as *mut _, obj.id);

        if mcache_laddr.is_null() {
            None
        } else {
            Some(mcache_laddr as *mut u8)
        }
    }

    /// Get mcache pointer for an output link
    #[inline]
    pub unsafe fn output_mcache(&self, topo: &crate::Topo, index: usize) -> Option<*mut u8> {
        let link = self.output_link(topo, index)?;
        let topo_ptr = topo.as_ptr();
        let mcache_obj_id = link.mcache_obj_id();
        let obj = &(*topo_ptr).objs[mcache_obj_id];
        let mcache_laddr = fd_topo_sys::fd_topo_obj_laddr(topo_ptr as *mut _, obj.id);

        if mcache_laddr.is_null() {
            None
        } else {
            Some(mcache_laddr as *mut u8)
        }
    }

    /// Get dcache pointer for an input link (if it has one)
    #[inline]
    pub unsafe fn input_dcache(&self, topo: &crate::Topo, index: usize) -> Option<*mut u8> {
        let link = self.input_link(topo, index)?;

        if !link.has_dcache() {
            return None;
        }

        let topo_ptr = topo.as_ptr();
        let dcache_obj_id = link.dcache_object_id();
        let obj = &(*topo_ptr).objs[dcache_obj_id];
        let dcache_laddr = fd_topo_sys::fd_topo_obj_laddr(topo_ptr as *mut _, obj.id);

        if dcache_laddr.is_null() {
            None
        } else {
            Some(dcache_laddr as *mut u8)
        }
    }

    /// Get dcache pointer for an output link (if it has one)
    #[inline]
    pub unsafe fn output_dcache(&self, topo: &crate::Topo, index: usize) -> Option<*mut u8> {
        let link = self.output_link(topo, index)?;

        if !link.has_dcache() {
            return None;
        }

        let topo_ptr = topo.as_ptr();
        let dcache_obj_id = link.dcache_object_id();
        let obj = &(*topo_ptr).objs[dcache_obj_id];
        let dcache_laddr = sys::fd_topo_obj_laddr(topo_ptr as *mut _, obj.id);

        if dcache_laddr.is_null() {
            None
        } else {
            Some(dcache_laddr as *mut u8)
        }
    }

    /// ids of objects this tile uses
    #[inline]
    pub fn uses_obj_ids(&self) -> Vec<usize> {
        unsafe {
            let count = (*self.inner).uses_obj_cnt as usize;
            let mut ids = Vec::with_capacity(count);
            for i in 0..count {
                ids.push((*self.inner).uses_obj_id[i] as usize);
            }
            ids
        }
    }

    /// Send data through an output link, without bounds or ptr checks
    #[inline]
    pub unsafe fn send_unchecked(
        &self,
        topo: &crate::Topo,
        link_index: usize,
        data: &[u8],
    ) -> crate::Result<()> {
        let mcache_ptr = self
            .output_mcache(topo, link_index)
            .ok_or(crate::TopoError::NotFound)?;

        let dcache_ptr = self
            .output_dcache(topo, link_index)
            .ok_or(crate::TopoError::NotFound)?;

        let sync_ptr = fd_topo_sys::fd_mcache_seq_laddr(mcache_ptr as *mut _);
        let seq = fd_topo_sys::fd_mcache_seq_query(sync_ptr);

        let link = self
            .output_link(topo, link_index)
            .ok_or(crate::TopoError::NotFound)?;
        let depth = link.depth() as u64;

        // get chunk from dcache
        let chunk = fd_topo_sys::fd_dcache_compact_chunk0(
            topo.as_ptr() as *const _,
            dcache_ptr as *const _,
        );

        if chunk == 0 {
            return Err(crate::TopoError::SystemError);
        }

        let chunk_addr = (dcache_ptr as u64).wrapping_add(chunk as u64) as *mut u8;
        std::ptr::copy_nonoverlapping(data.as_ptr(), chunk_addr, data.len());

        // control word (som=1, eom=1 for complete message, err=0)
        let ctl = fd_topo_sys::fd_frag_meta_ctl(0, 1, 1, 0);

        fd_topo_sys::fd_mcache_publish(
            mcache_ptr as *mut _,
            depth,
            seq,
            0,                 // sig
            chunk,             // chunk
            data.len() as u64, // sz
            ctl,               // ctl
            0,                 // tsorig
            0,                 // tspub
        );

        Ok(())
    }

    /// Receive data from an input link into a provided buffer
    /// Returns the number of bytes received, or None if no data available
    #[inline]
    pub unsafe fn _recv_internal(
        &self,
        topo: &crate::Topo,
        link_index: usize,
        buffer: &mut [u8],
    ) -> Option<usize> {
        let mcache_ptr = self.input_mcache(topo, link_index)?;
        let dcache_ptr = self.input_dcache(topo, link_index)?;

        let sync_ptr = fd_topo_sys::fd_mcache_seq_laddr(mcache_ptr as *mut _);
        let seq = fd_topo_sys::fd_mcache_seq_query(sync_ptr);
        let link = self.input_link(topo, link_index)?;
        let depth = link.depth() as u64;

        let available_seq = fd_topo_sys::fd_mcache_query(mcache_ptr as *const _, depth, seq);

        if available_seq == seq {
            return None;
        }

        let mcache_line_idx = seq % depth;
        let frag_meta_ptr =
            (mcache_ptr as *const fd_topo_sys::fd_frag_meta_t).offset(mcache_line_idx as isize);

        let chunk = (*frag_meta_ptr).__bindgen_anon_1.chunk;
        let sz = (*frag_meta_ptr).__bindgen_anon_1.sz;

        if sz == 0 || chunk == 0 || sz as usize > buffer.len() {
            return None;
        }

        let chunk_addr = (dcache_ptr as u64).wrapping_add(chunk as u64) as *const u8;

        core::ptr::copy_nonoverlapping(chunk_addr, buffer.as_mut_ptr(), sz as usize);
        fd_topo_sys::fd_mcache_seq_update(sync_ptr, seq.wrapping_add(1));

        Some(sz as usize)
    }

    #[inline]
    pub unsafe fn recv_unchecked(&self, topo: &crate::Topo, link_index: usize) -> Option<Vec<u8>> {
        let mut buffer = [0u8; SCRATCH_BUF_SZ];
        let bytes_received = self._recv_internal(topo, link_index, &mut buffer)?;
        Some(buffer[..bytes_received].to_vec())
    }

    #[inline]
    pub fn send<T>(&self, topo: &crate::Topo, link_index: usize, data: &T) -> crate::Result<()> {
        let bytes = unsafe {
            std::slice::from_raw_parts(data as *const T as *const u8, std::mem::size_of::<T>())
        };
        unsafe { self.send_unchecked(topo, link_index, bytes) }
    }

    #[inline]
    pub unsafe fn recv_into<T>(
        &self,
        topo: &crate::Topo,
        link_index: usize,
        output: &mut T,
    ) -> bool {
        let buffer =
            std::slice::from_raw_parts_mut(output as *mut T as *mut u8, std::mem::size_of::<T>());

        match self._recv_internal(topo, link_index, buffer) {
            Some(bytes_received) if bytes_received == std::mem::size_of::<T>() => true,
            _ => false,
        }
    }

    #[inline]
    pub fn recv<T>(&self, topo: &crate::Topo, link_index: usize) -> Option<T> {
        let data = unsafe { self.recv_unchecked(topo, link_index)? };

        if data.len() != std::mem::size_of::<T>() {
            return None; // sz_mismatch
        }

        Some(unsafe { std::ptr::read(data.as_ptr() as *const T) })
    }
}

unsafe impl Send for Tile {}
unsafe impl Sync for Tile {}

#[repr(C)]
pub struct TileRunner {
    /// Name of the tile type this runner handles
    pub name: &'static CStr,
    /// Initialize the tile with privileges (before sandboxing)
    pub privileged_init: Option<TileContextFn>,
    /// Initialize the tile without privileges (after sandboxing)
    pub unprivileged_init: Option<TileContextFn>,
    /// Main execution function for the tile
    pub run: TileContextFn,
    /// Calculate scratch memory alignment
    pub scratch_align: Option<TileAnonymousFn<u64>>,
    /// Calculate scratch memory footprint
    pub scratch_footprint: Option<TileRunnerFn<*const _TileInternal, u64>>,
    /// Calculate loose memory footprint
    pub loose_footprint: Option<TileRunnerFn<*const _TileInternal, u64>>,
    /// Populate allowed file descriptors for sandboxing
    pub populate_allowed_fds: Option<PopulateAllowedFdsFn>,
    /// Populate allowed seccomp filters for sandboxing
    pub populate_allowed_seccomp: Option<PopulateAllowedSeccompFn>,
}

impl TileRunner {
    pub fn new(name: &'static CStr, run: TileContextFn) -> Self {
        Self {
            name,
            privileged_init: None,
            unprivileged_init: None,
            run,
            scratch_align: None,
            scratch_footprint: None,
            loose_footprint: None,
            populate_allowed_fds: None,
            populate_allowed_seccomp: None,
        }
    }

    /// Set the function to populate allowed file descriptors for sandboxing
    pub fn with_allowed_fds(mut self, populate_fds: PopulateAllowedFdsFn) -> Self {
        self.populate_allowed_fds = Some(populate_fds);
        self
    }

    /// Set the function to populate allowed seccomp filters for sandboxing
    pub fn with_seccomp(mut self, populate_seccomp: PopulateAllowedSeccompFn) -> Self {
        self.populate_allowed_seccomp = Some(populate_seccomp);
        self
    }

    pub fn with_privileged_init(mut self, init: TileContextFn) -> Self {
        self.privileged_init = Some(init);
        self
    }

    pub fn with_unprivileged_init(mut self, init: TileContextFn) -> Self {
        self.unprivileged_init = Some(init);
        self
    }

    pub fn with_scratch_align(mut self, align: TileAnonymousFn<u64>) -> Self {
        self.scratch_align = Some(align);
        self
    }

    pub fn with_scratch_footprint(
        mut self,
        footprint: TileRunnerFn<*const _TileInternal, u64>,
    ) -> Self {
        self.scratch_footprint = Some(footprint);
        self
    }

    pub fn with_loose_footprint(
        mut self,
        footprint: TileRunnerFn<*const _TileInternal, u64>,
    ) -> Self {
        self.loose_footprint = Some(footprint);
        self
    }
}

#[repr(C)]
pub struct TileRunnerRegistry {
    runners: Vec<TileRunner>,
    c_runners: Vec<_TileRunnerInternal>,
}

impl TileRunnerRegistry {
    pub fn new() -> Self {
        Self {
            runners: Vec::new(),
            c_runners: Vec::new(),
        }
    }

    pub fn add_runner(&mut self, runner: TileRunner) -> Result<()> {
        self.runners.push(runner);
        Ok(())
    }

    pub fn finalize(&mut self) -> Result<&[ActiveTileRunner]> {
        self.c_runners.clear();

        for runner in &self.runners {
            let c_runner = _TileRunnerInternal {
                name: runner.name.as_ptr(),
                keep_host_networking: 0,
                allow_connect: 0,
                rlimit_file_cnt: 1024,
                rlimit_address_space: 0,
                rlimit_data: 0,
                for_tpool: 0,
                populate_allowed_seccomp: runner.populate_allowed_seccomp,
                populate_allowed_fds: runner.populate_allowed_fds,
                scratch_align: runner.scratch_align,
                scratch_footprint: runner.scratch_footprint,
                loose_footprint: runner.loose_footprint,
                privileged_init: runner.privileged_init,
                unprivileged_init: runner.unprivileged_init,
                run: Some(runner.run),
                rlimit_file_cnt_fn: None,
            };
            self.c_runners.push(c_runner);
        }

        Ok(&self.c_runners)
    }

    pub fn find_runner(&self, tile_name: &str) -> Option<&ActiveTileRunner> {
        if self.c_runners.is_empty() {
            return None;
        }

        for (i, runner) in self.runners.iter().enumerate() {
            if let Ok(name) = runner.name.to_str() {
                if name == tile_name {
                    return self.c_runners.get(i);
                }
            }
        }
        None
    }
}

impl Default for TileRunnerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
