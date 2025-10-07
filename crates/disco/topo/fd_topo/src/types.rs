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
