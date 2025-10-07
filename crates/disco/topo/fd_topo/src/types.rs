use fd_topo_sys as sys;

pub type ActiveTopology = _TopoInternal;
pub type ActiveTopoCallbacks = _TopoCallbacksInternal;
pub type ActiveTileRunner = _TileRunnerInternal;
pub type ActiveLink = _LinkInternal;
pub type ActiveObject = _ObjectInternal;
pub type ActiveTile = _TileInternal;
pub type ActiveWorkspace = _WorkspaceInternal;
pub type ActiveCpus = _CpusInternal;

pub(crate) type _TopoInternal = sys::fd_topo_t;
pub(crate) type _TileInternal = sys::fd_topo_tile_t;
pub(crate) type _WorkspaceInternal = sys::fd_topo_wksp_t;
pub(crate) type _ObjectInternal = sys::fd_topo_obj_t;
pub(crate) type _LinkInternal = sys::fd_topo_link_t;
pub(crate) type _TopoCallbacksInternal = sys::fd_topo_obj_callbacks_t;
pub(crate) type _TileRunnerInternal = sys::fd_topo_run_tile_t;
pub(crate) type _CpusInternal = sys::fd_topo_cpus_t;

pub type TopoCallbackFn<R> =
    unsafe extern "C" fn(topo: *const _TopoInternal, obj: *const _ObjectInternal) -> R;
pub type TileAnonymousFn<R> = unsafe extern "C" fn() -> R;
pub type TileRunnerFn<A, R> = unsafe extern "C" fn(A) -> R;
pub type TileContextFn = unsafe extern "C" fn(*mut _TopoInternal, *mut _TileInternal);

pub type PopulateAllowedFdsFn = unsafe extern "C" fn(
    topo: *const crate::types::_TopoInternal,
    tile: *const _TileInternal,
    out_fds_sz: u64,
    out_fds: *mut i32,
) -> u64;

pub type PopulateAllowedSeccompFn = unsafe extern "C" fn(
    topo: *const crate::types::_TopoInternal,
    tile: *const _TileInternal,
    out_cnt: u64,
    out: *mut fd_topo_sys::sock_filter,
) -> u64;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSize {
    Normal = sys::FD_SHMEM_NORMAL_PAGE_SZ,
    Huge = sys::FD_SHMEM_HUGE_PAGE_SZ,
    Gigantic = sys::FD_SHMEM_GIGANTIC_PAGE_SZ,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileExecutionMode {
    /// Run a single tile in its own process with full sandboxing
    Single,
    /// Run multiple tiles, each in its own separate process with full sandboxing
    Isolated,
}

impl Default for TileExecutionMode {
    fn default() -> Self {
        Self::Isolated
    }
}

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// enable full sandboxing (default: false)
    pub enabled: bool,
    /// keep the controlling terminal (default: false)
    pub keep_controlling_terminal: bool,
    /// the process should be dumpable (default: false)
    pub dumpable: bool,
    /// additional file descriptors to allow in the sandbox
    pub allowed_fds: Vec<i32>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            keep_controlling_terminal: false,
            dumpable: false,
            allowed_fds: Vec::new(),
        }
    }
}

impl SandboxConfig {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn enabled() -> Self {
        Self {
            enabled: true,
            keep_controlling_terminal: false,
            dumpable: false,
            allowed_fds: Vec::new(),
        }
    }

    /// allow stdin/stdout/stderr in the sandbox
    pub fn with_stdio(mut self) -> Self {
        self.allowed_fds.extend_from_slice(&[0, 1, 2]);
        self
    }

    /// keep the controlling terminal (for things like Ctrl+C to work)
    pub fn with_controlling_terminal(mut self) -> Self {
        self.keep_controlling_terminal = true;
        self
    }

    /// make the process dumpable (core dumps and debugging)
    pub fn with_dumpable(mut self) -> Self {
        self.dumpable = true;
        self
    }

    /// additional allowed file descriptors
    pub fn with_allowed_fds(mut self, fds: &[i32]) -> Self {
        self.allowed_fds.extend_from_slice(fds);
        self
    }
}

// #[repr(transparent)]
// pub struct ActiveTopology(*mut _TopoInternal);

// impl ActiveTopology {
//     #[inline]
//     pub fn inner(&self) -> Topo {
//         unsafe { Topo::from_raw(self.0, false) }
//     }
// }

// #[repr(transparent)]
// pub struct ActiveLink(*mut _LinkInternal);

// impl ActiveLink {
//     #[inline]
//     pub fn inner(&self) -> Link {
//         unsafe { Link::from_raw(self.0) }
//     }
// }

// #[repr(transparent)]
// pub struct ActiveObject(*mut _ObjectInternal);

// impl ActiveObject {
//     #[inline]
//     pub fn inner(&self) -> Object {
//         unsafe { Object::from_raw(self.0) }
//     }
// }

// #[repr(transparent)]
// pub struct ActiveWorkspace(*mut _WorkspaceInternal);

// impl ActiveWorkspace {
//     #[inline]
//     pub fn inner(&self) -> Workspace {
//         unsafe { Workspace::from_raw(self.0) }
//     }
// }

// #[repr(transparent)]
// pub struct ActiveTile(*mut _TileInternal);

// impl ActiveTile {
//     #[inline]
//     pub fn inner(&self) -> Tile {
//         unsafe { Tile::from_raw(self.0) }
//     }
// }
