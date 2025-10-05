//! Safe API for `fd_wksp_sys`

use core::ffi::CStr;
use core::ptr::NonNull;
use fd_wksp_sys::{self as sys, ulong};

use std::ffi::CString;

#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceError {
    InvalidInput(&'static str),
    SystemFailure(&'static str),
    MemoryCorruption(&'static str),
    AllocationFailed,
    InvalidAlignment,
    InvalidSize,
    InsufficientSpace,
    InvalidWorkspace,
    InvalidGlobalAddress,
    InvalidLocalAddress,
    CreationFailed,
    JoinFailed,
    OperationFailed,
    InvalidTag,
    CheckpointFailed,
    RestoreFailed,
}

impl core::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WorkspaceError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            WorkspaceError::SystemFailure(msg) => write!(f, "System failure: {}", msg),
            WorkspaceError::MemoryCorruption(msg) => write!(f, "Memory corruption: {}", msg),
            WorkspaceError::AllocationFailed => write!(f, "Allocation failed"),
            WorkspaceError::InvalidAlignment => write!(f, "Invalid alignment - must be power of 2"),
            WorkspaceError::InvalidSize => write!(f, "Invalid size - must be > 0"),
            WorkspaceError::InsufficientSpace => write!(f, "Insufficient workspace space"),
            WorkspaceError::InvalidWorkspace => write!(f, "Invalid workspace handle"),
            WorkspaceError::InvalidGlobalAddress => write!(f, "Invalid global address"),
            WorkspaceError::InvalidLocalAddress => write!(f, "Invalid local address"),
            WorkspaceError::CreationFailed => write!(f, "Workspace creation failed"),
            WorkspaceError::JoinFailed => write!(f, "Workspace join failed"),
            WorkspaceError::OperationFailed => write!(f, "Operation failed"),
            WorkspaceError::InvalidTag => write!(f, "Invalid tag - must be positive"),
            WorkspaceError::CheckpointFailed => write!(f, "Checkpoint failed"),
            WorkspaceError::RestoreFailed => write!(f, "Restore failed"),
        }
    }
}

impl core::error::Error for WorkspaceError {}

fn convert_error(code: i32) -> WorkspaceError {
    match code {
        0 => unreachable!("Success should not be converted to error"), // FD_WKSP_SUCCESS
        -1 => WorkspaceError::InvalidInput("Invalid parameters"),      // FD_WKSP_ERR_INVAL
        -2 => WorkspaceError::SystemFailure("System limitation"),      // FD_WKSP_ERR_FAIL
        -3 => WorkspaceError::MemoryCorruption("Corruption detected"), // FD_WKSP_ERR_CORRUPT
        _ => WorkspaceError::OperationFailed,
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalAddress(pub ulong);

impl GlobalAddress {
    pub const NULL: Self = Self(0);

    pub fn is_null(self) -> bool {
        self.0 == 0
    }

    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    pub gaddr_lo: GlobalAddress,
    pub gaddr_hi: GlobalAddress,
    pub tag: ulong,
}

#[derive(Debug, Clone)]
pub struct WorkspaceUsage {
    pub total_max: usize,
    pub total_cnt: usize,
    pub total_sz: usize,
    pub free_cnt: usize,
    pub free_sz: usize,
    pub used_cnt: usize,
    pub used_sz: usize,
}

#[repr(C)]
pub struct WorkspaceAllocation<'a> {
    workspace: &'a Workspace,
    gaddr: GlobalAddress,
    size: usize,
    laddr: NonNull<u8>,
}

impl<'a> WorkspaceAllocation<'a> {
    fn new(
        workspace: &'a Workspace,
        gaddr: GlobalAddress,
        size: usize,
    ) -> Result<Self, WorkspaceError> {
        let laddr_ptr = unsafe { sys::fd_wksp_laddr(workspace.handle.as_ptr(), gaddr.0) };
        if laddr_ptr.is_null() {
            return Err(WorkspaceError::InvalidGlobalAddress);
        }

        let laddr = NonNull::new(laddr_ptr as *mut u8).unwrap();

        Ok(Self {
            workspace,
            gaddr,
            size,
            laddr,
        })
    }

    #[inline]
    pub fn global_address(&self) -> GlobalAddress {
        self.gaddr
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut u8 {
        self.laddr.as_ptr()
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.laddr.as_ptr(), self.size) }
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.laddr.as_ptr(), self.size) }
    }

    #[inline]
    pub fn clear(&mut self) {
        unsafe {
            sys::fd_wksp_memset(self.workspace.handle.as_ptr(), self.gaddr.0, 0);
        }
    }

    #[inline]
    pub fn fill(&mut self, value: u8) {
        unsafe {
            sys::fd_wksp_memset(self.workspace.handle.as_ptr(), self.gaddr.0, value as i32);
        }
    }

    #[inline]
    pub fn tag(&self) -> ulong {
        unsafe { sys::fd_wksp_tag(self.workspace.handle.as_ptr(), self.gaddr.0) }
    }

    /// Convert to a raw global address, consuming the allocation
    /// SAFETY: The caller is responsible for freeing the allocation
    pub fn into_raw(self) -> GlobalAddress {
        let gaddr = self.gaddr;
        core::mem::forget(self);
        gaddr
    }
}

impl<'a> Drop for WorkspaceAllocation<'a> {
    fn drop(&mut self) {
        unsafe {
            sys::fd_wksp_free(self.workspace.handle.as_ptr(), self.gaddr.0);
        }
    }
}

pub struct Workspace {
    handle: NonNull<sys::fd_wksp_t>,
    is_anonymous: bool,
}

impl Workspace {
    #[inline]
    pub fn attach(name: &str) -> Result<Self, WorkspaceError> {
        let c_name =
            CString::new(name).map_err(|_| WorkspaceError::InvalidInput("Invalid name"))?;

        let handle_ptr = unsafe { sys::fd_wksp_attach(c_name.as_ptr()) };
        if handle_ptr.is_null() {
            return Err(WorkspaceError::JoinFailed);
        }

        let handle = NonNull::new(handle_ptr).unwrap();

        Ok(Self {
            handle,
            is_anonymous: false,
        })
    }

    #[inline]
    pub(crate) fn new_anonymous(
        name: &str,
        page_sz: usize,
        sub_cnt: usize,
        sub_page_cnt: &[usize],
        sub_cpu_idx: &[usize],
        seed: u32,
        opt_part_max: usize,
    ) -> Result<Self, WorkspaceError> {
        if sub_page_cnt.len() != sub_cnt || sub_cpu_idx.len() != sub_cnt {
            return Err(WorkspaceError::InvalidInput("Mismatched array lengths"));
        }

        let c_name =
            CString::new(name).map_err(|_| WorkspaceError::InvalidInput("Invalid name"))?;

        let handle_ptr = unsafe {
            sys::fd_wksp_new_anon(
                c_name.as_ptr(),
                page_sz as ulong,
                sub_cnt as ulong,
                sub_page_cnt.as_ptr() as *const ulong,
                sub_cpu_idx.as_ptr() as *const ulong,
                seed,
                opt_part_max as ulong,
            )
        };

        if handle_ptr.is_null() {
            return Err(WorkspaceError::CreationFailed);
        }

        let handle = NonNull::new(handle_ptr).unwrap();

        Ok(Self {
            handle,
            is_anonymous: true,
        })
    }

    #[inline]
    pub fn name(&self) -> &str {
        unsafe {
            let c_str = sys::fd_wksp_name(self.handle.as_ptr());
            CStr::from_ptr(c_str).to_str().unwrap_or("<invalid>")
        }
    }

    #[inline]
    pub fn seed(&self) -> u32 {
        unsafe { sys::fd_wksp_seed(self.handle.as_ptr()) }
    }

    #[inline]
    pub fn part_max(&self) -> usize {
        unsafe { sys::fd_wksp_part_max(self.handle.as_ptr()) as usize }
    }

    #[inline]
    pub fn data_max(&self) -> usize {
        unsafe { sys::fd_wksp_data_max(self.handle.as_ptr()) as usize }
    }

    #[inline]
    pub fn owner(&self) -> ulong {
        unsafe { sys::fd_wksp_owner(self.handle.as_ptr()) }
    }

    #[inline]
    pub fn allocate(
        &self,
        size: usize,
        align: usize,
        tag: ulong,
    ) -> Result<WorkspaceAllocation<'_>, WorkspaceError> {
        if size == 0 {
            return Err(WorkspaceError::InvalidSize);
        }

        if tag == 0 {
            return Err(WorkspaceError::InvalidTag);
        }

        if align > 0 && !align.is_power_of_two() {
            return Err(WorkspaceError::InvalidAlignment);
        }

        let gaddr =
            unsafe { sys::fd_wksp_alloc(self.handle.as_ptr(), align as ulong, size as ulong, tag) };

        if gaddr == 0 {
            return Err(WorkspaceError::AllocationFailed);
        }

        WorkspaceAllocation::new(self, GlobalAddress(gaddr), size)
    }

    #[inline]
    pub fn allocate_at_least(
        &self,
        size: usize,
        align: usize,
        tag: ulong,
    ) -> Result<(WorkspaceAllocation<'_>, GlobalAddress, GlobalAddress), WorkspaceError> {
        if size == 0 {
            return Err(WorkspaceError::InvalidSize);
        }

        if tag == 0 {
            return Err(WorkspaceError::InvalidTag);
        }

        if align > 0 && !align.is_power_of_two() {
            return Err(WorkspaceError::InvalidAlignment);
        }

        let mut lo = 0u64;
        let mut hi = 0u64;

        let gaddr = unsafe {
            sys::fd_wksp_alloc_at_least(
                self.handle.as_ptr(),
                align as ulong,
                size as ulong,
                tag,
                &mut lo,
                &mut hi,
            )
        };

        if gaddr == 0 {
            return Err(WorkspaceError::AllocationFailed);
        }

        let allocation = WorkspaceAllocation::new(self, GlobalAddress(gaddr), size)?;
        Ok((allocation, GlobalAddress(lo), GlobalAddress(hi)))
    }

    #[inline]
    pub fn free_gaddr(&self, gaddr: GlobalAddress) {
        unsafe {
            sys::fd_wksp_free(self.handle.as_ptr(), gaddr.0);
        }
    }

    #[inline]
    pub fn free(&self, allocation: WorkspaceAllocation) -> Result<(), WorkspaceError> {
        let gaddr = allocation.into_raw();
        self.free_gaddr(gaddr);
        Ok(())
    }

    #[inline]
    pub fn gaddr_to_laddr(&self, gaddr: GlobalAddress) -> Result<*mut u8, WorkspaceError> {
        let laddr = unsafe { sys::fd_wksp_laddr(self.handle.as_ptr(), gaddr.0) };
        if laddr.is_null() {
            Err(WorkspaceError::InvalidGlobalAddress)
        } else {
            Ok(laddr as *mut u8)
        }
    }

    #[inline]
    pub fn laddr_to_gaddr(&self, laddr: *const u8) -> Result<GlobalAddress, WorkspaceError> {
        let gaddr =
            unsafe { sys::fd_wksp_gaddr(self.handle.as_ptr(), laddr as *const core::ffi::c_void) };
        if gaddr == 0 {
            Err(WorkspaceError::InvalidLocalAddress)
        } else {
            Ok(GlobalAddress(gaddr))
        }
    }

    #[inline]
    pub fn get_tag(&self, gaddr: GlobalAddress) -> ulong {
        unsafe { sys::fd_wksp_tag(self.handle.as_ptr(), gaddr.0) }
    }

    pub fn query_by_tag(&self, tags: &[ulong]) -> Result<Vec<AllocationInfo>, WorkspaceError> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }

        let count = unsafe {
            sys::fd_wksp_tag_query(
                self.handle.as_ptr(),
                tags.as_ptr(),
                tags.len() as ulong,
                core::ptr::null_mut(),
                0,
            )
        };

        if count == 0 {
            return Ok(Vec::new());
        }

        let mut info_buf = vec![
            sys::fd_wksp_tag_query_info {
                gaddr_lo: 0,
                gaddr_hi: 0,
                tag: 0
            };
            count as usize
        ];

        let actual_count = unsafe {
            sys::fd_wksp_tag_query(
                self.handle.as_ptr(),
                tags.as_ptr(),
                tags.len() as ulong,
                info_buf.as_mut_ptr(),
                count,
            )
        };

        info_buf.truncate(actual_count as usize);

        Ok(info_buf
            .into_iter()
            .map(|info| AllocationInfo {
                gaddr_lo: GlobalAddress(info.gaddr_lo),
                gaddr_hi: GlobalAddress(info.gaddr_hi),
                tag: info.tag,
            })
            .collect())
    }

    #[inline]
    pub fn free_by_tag(&self, tags: &[ulong]) {
        if !tags.is_empty() {
            unsafe {
                sys::fd_wksp_tag_free(self.handle.as_ptr(), tags.as_ptr(), tags.len() as ulong);
            }
        }
    }

    #[inline]
    pub fn clear_allocation(&self, gaddr: GlobalAddress) {
        unsafe {
            sys::fd_wksp_memset(self.handle.as_ptr(), gaddr.0, 0);
        }
    }

    #[inline]
    pub fn reset(&self, seed: u32) {
        unsafe {
            sys::fd_wksp_reset(self.handle.as_ptr(), seed);
        }
    }

    #[inline]
    pub fn usage(&self, tags: &[ulong]) -> WorkspaceUsage {
        let mut usage = sys::fd_wksp_usage {
            total_max: 0,
            total_cnt: 0,
            total_sz: 0,
            free_cnt: 0,
            free_sz: 0,
            used_cnt: 0,
            used_sz: 0,
        };

        unsafe {
            sys::fd_wksp_usage(
                self.handle.as_ptr(),
                if tags.is_empty() {
                    core::ptr::null()
                } else {
                    tags.as_ptr()
                },
                tags.len() as ulong,
                &mut usage,
            );
        }

        WorkspaceUsage {
            total_max: usage.total_max as usize,
            total_cnt: usage.total_cnt as usize,
            total_sz: usage.total_sz as usize,
            free_cnt: usage.free_cnt as usize,
            free_sz: usage.free_sz as usize,
            used_cnt: usage.used_cnt as usize,
            used_sz: usage.used_sz as usize,
        }
    }

    #[inline]
    pub fn verify(&self) -> Result<(), WorkspaceError> {
        let result = unsafe { sys::fd_wksp_verify(self.handle.as_ptr()) };
        if result != 0 {
            // FD_WKSP_SUCCESS
            Err(convert_error(result))
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn rebuild(&self, seed: u32) -> Result<(), WorkspaceError> {
        let result = unsafe { sys::fd_wksp_rebuild(self.handle.as_ptr(), seed) };
        if result != 0 {
            // FD_WKSP_SUCCESS
            Err(convert_error(result))
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn checkpoint(
        &self,
        path: &str,
        mode: u64,
        style: i32,
        user_info: Option<&str>,
    ) -> Result<(), WorkspaceError> {
        let c_path =
            CString::new(path).map_err(|_| WorkspaceError::InvalidInput("Invalid path"))?;
        let c_info = user_info.map(|s| CString::new(s).ok()).flatten();

        let result = unsafe {
            sys::fd_wksp_checkpt(
                self.handle.as_ptr(),
                c_path.as_ptr(),
                mode as ulong,
                style,
                c_info.as_ref().map_or(core::ptr::null(), |s| s.as_ptr()),
            )
        };

        if result != 0 {
            // FD_WKSP_SUCCESS
            Err(WorkspaceError::CheckpointFailed)
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn restore(&self, path: &str, seed: u32) -> Result<(), WorkspaceError> {
        let c_path =
            CString::new(path).map_err(|_| WorkspaceError::InvalidInput("Invalid path"))?;

        let result = unsafe { sys::fd_wksp_restore(self.handle.as_ptr(), c_path.as_ptr(), seed) };

        if result != 0 {
            // FD_WKSP_SUCCESS
            Err(WorkspaceError::RestoreFailed)
        } else {
            Ok(())
        }
    }

    pub fn as_raw(&self) -> *mut sys::fd_wksp_t {
        self.handle.as_ptr()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        if self.is_anonymous {
            unsafe {
                sys::fd_wksp_delete_anon(self.handle.as_ptr());
            }
        } else {
            unsafe {
                sys::fd_wksp_detach(self.handle.as_ptr());
            }
        }
    }
}

pub struct WorkspaceBuilder<'a> {
    name: Option<&'a str>,
    page_size: usize,
    page_counts: Vec<usize>,
    cpu_indices: Vec<usize>,
    seed: u32,
    opt_part_max: usize,
}

impl<'a> WorkspaceBuilder<'a> {
    pub fn new() -> Self {
        Self {
            name: None,
            page_size: 4096,
            page_counts: vec![64],
            cpu_indices: vec![0],
            seed: 0,
            opt_part_max: 0,
        }
    }

    #[inline]
    pub fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    #[inline]
    pub fn page_size(mut self, size: usize) -> Self {
        self.page_size = size;
        self
    }

    #[inline]
    pub fn page_count(mut self, count: usize) -> Self {
        self.page_counts = vec![count];
        self.cpu_indices = vec![0];
        self
    }

    #[inline]
    pub fn multi_numa(mut self, page_counts: Vec<usize>, cpu_indices: Vec<usize>) -> Self {
        self.page_counts = page_counts;
        self.cpu_indices = cpu_indices;
        self
    }

    #[inline]
    pub fn cpu_index(mut self, cpu_idx: usize) -> Self {
        if self.cpu_indices.len() == 1 {
            self.cpu_indices[0] = cpu_idx;
        }
        self
    }

    #[inline]
    pub fn seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    #[inline]
    pub fn part_max(mut self, part_max: usize) -> Self {
        self.opt_part_max = part_max;
        self
    }

    pub fn build_anonymous(self) -> Result<Workspace, WorkspaceError> {
        let name = self.name.unwrap_or_else(|| "anonymous");

        Workspace::new_anonymous(
            &name,
            self.page_size,
            self.page_counts.len(),
            &self.page_counts,
            &self.cpu_indices,
            self.seed,
            self.opt_part_max,
        )
    }

    pub fn build_named(self) -> Result<Workspace, WorkspaceError> {
        let name = self
            .name
            .ok_or_else(|| WorkspaceError::InvalidInput("Name required for named workspace"))?;

        let c_name =
            CString::new(name).map_err(|_| WorkspaceError::InvalidInput("Invalid name"))?;

        let result = unsafe {
            sys::fd_wksp_new_named(
                c_name.as_ptr(),
                self.page_size as ulong,
                self.page_counts.len() as ulong,
                self.page_counts.as_ptr() as *const ulong,
                self.cpu_indices.as_ptr() as *const ulong,
                0o666,
                self.seed,
                self.opt_part_max as ulong,
            )
        };

        if result != 0 {
            // FD_WKSP_SUCCESS
            return Err(convert_error(result));
        }

        Workspace::attach(&name)
    }
}

impl<'a> Default for WorkspaceBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

pub mod utils {
    use super::*;

    #[inline]
    pub fn footprint(part_max: usize, data_max: usize) -> usize {
        unsafe { sys::fd_wksp_footprint(part_max as ulong, data_max as ulong) as usize }
    }

    #[inline]
    pub fn align() -> usize {
        unsafe { sys::fd_wksp_align() as usize }
    }

    #[inline]
    pub fn part_max_est(footprint: usize, sz_typical: usize) -> usize {
        unsafe { sys::fd_wksp_part_max_est(footprint as ulong, sz_typical as ulong) as usize }
    }

    #[inline]
    pub fn data_max_est(footprint: usize, part_max: usize) -> usize {
        unsafe { sys::fd_wksp_data_max_est(footprint as ulong, part_max as ulong) as usize }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_alloc() {
        let wksp = WorkspaceBuilder::new()
            .name("test")
            .page_size(4096)
            .page_count(64)
            .seed(42)
            .build_anonymous()
            .unwrap();

        let allocation = wksp.allocate(1024, 64, 1).unwrap();
        assert_eq!(allocation.size(), 1024);
        assert_eq!(allocation.as_ptr() as usize % 64, 0);
        assert_eq!(allocation.tag(), 1);

        let gaddr = allocation.global_address();
        let laddr = wksp.gaddr_to_laddr(gaddr).unwrap();
        assert_eq!(laddr, allocation.as_ptr());

        let gaddr_back = wksp.laddr_to_gaddr(laddr).unwrap();
        assert_eq!(gaddr, gaddr_back);
    }

    #[test]
    fn test_multi_alloc() {
        let wksp = WorkspaceBuilder::new()
            .name("test")
            .page_size(4096)
            .page_count(64)
            .seed(42)
            .build_anonymous()
            .unwrap();

        let mut allocation = wksp.allocate(1024, 64, 1).unwrap();

        let data = allocation.as_mut_slice();
        data[0..4].copy_from_slice(b"test");

        let read_data = allocation.as_mut_slice();
        assert_eq!(read_data[0..4], *b"test");
    }

    #[test]
    fn test_tag_queries() {
        let wksp = WorkspaceBuilder::new()
            .name("test")
            .page_size(4096)
            .page_count(64)
            .seed(42)
            .build_anonymous()
            .unwrap();

        let _alloc1 = wksp.allocate(512, 32, 1).unwrap();
        let _alloc2 = wksp.allocate(1024, 64, 2).unwrap();
        let _alloc3 = wksp.allocate(256, 16, 1).unwrap(); // Same tag as alloc1

        let tag1_allocs = wksp.query_by_tag(&[1]).unwrap();
        assert_eq!(tag1_allocs.len(), 2);

        let tag2_allocs = wksp.query_by_tag(&[2]).unwrap();
        assert_eq!(tag2_allocs.len(), 1);

        let all_allocs = wksp.query_by_tag(&[1, 2]).unwrap();
        assert_eq!(all_allocs.len(), 3);

        wksp.free_by_tag(&[1]);
        let remaining_allocs = wksp.query_by_tag(&[1, 2]).unwrap();
        assert_eq!(remaining_allocs.len(), 1);
        assert_eq!(remaining_allocs[0].tag, 2);
    }

    #[test]
    fn test_usage() {
        let wksp = WorkspaceBuilder::new()
            .name("test")
            .page_size(4096)
            .page_count(64)
            .seed(42)
            .build_anonymous()
            .unwrap();

        let initial_usage = wksp.usage(&[]);
        assert!(initial_usage.total_max > 0);
        assert_eq!(initial_usage.used_cnt, 0);

        let _alloc = wksp.allocate(1024, 64, 1).unwrap();

        let usage_after_alloc = wksp.usage(&[]);
        assert!(usage_after_alloc.used_cnt > 0);
        assert!(usage_after_alloc.used_sz >= 1024);

        let tag_usage = wksp.usage(&[1]);
        assert_eq!(tag_usage.used_cnt, 1);
    }

    #[test]
    fn test_reset() {
        let wksp = WorkspaceBuilder::new()
            .name("test-reset")
            .page_size(4096)
            .page_count(64)
            .seed(42)
            .build_anonymous()
            .unwrap();

        let _alloc = wksp.allocate(1024, 64, 1).unwrap();

        let usage_before = wksp.usage(&[]);
        assert!(usage_before.used_cnt > 0);

        wksp.reset(123);

        let usage_after = wksp.usage(&[]);
        assert_eq!(usage_after.used_cnt, 0);
        assert_eq!(wksp.seed(), 123);
    }

    #[test]
    fn test_error_conditions() {
        let wksp = WorkspaceBuilder::new()
            .name("test-errors")
            .page_size(4096)
            .page_count(64)
            .seed(42)
            .build_anonymous()
            .unwrap();

        assert!(matches!(
            wksp.allocate(0, 64, 1),
            Err(WorkspaceError::InvalidSize)
        ));

        assert!(matches!(
            wksp.allocate(1024, 64, 0),
            Err(WorkspaceError::InvalidTag)
        ));

        assert!(matches!(
            wksp.allocate(1024, 63, 1),
            Err(WorkspaceError::InvalidAlignment)
        ));

        assert!(matches!(
            wksp.gaddr_to_laddr(GlobalAddress::NULL),
            Err(WorkspaceError::InvalidGlobalAddress)
        ));
    }

    #[test]
    fn test_utils() {
        let align = utils::align();
        assert!(align > 0);
        assert!(align.is_power_of_two());

        let footprint = utils::footprint(100, 1024 * 1024);
        assert!(footprint > 0);

        let part_max_est = utils::part_max_est(16 * 1024 * 1024, 64 * 1024);
        assert!(part_max_est > 0);

        let data_max_est = utils::data_max_est(16 * 1024 * 1024, part_max_est);
        assert!(data_max_est > 0);
    }
}
