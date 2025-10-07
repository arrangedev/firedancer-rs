use fd_topo_sys as sys;

pub type ActiveTopology = _TopoInternal;
pub type ActiveTile = _TileInternal;
pub type ActiveTopoCallbacks = _TopoCallbacksInternal;
pub type ActiveTileRunner = _TileRunnerInternal;
pub type ActiveWorkspace = _WorkspaceInternal;
pub type ActiveObject = _ObjectInternal;

pub(crate) type _TopoInternal = sys::fd_topo_t;
pub(crate) type _TileInternal = sys::fd_topo_tile_t;
pub(crate) type _WorkspaceInternal = sys::fd_topo_wksp_t;
pub(crate) type _ObjectInternal = sys::fd_topo_obj_t;
pub(crate) type _LinkInternal = sys::fd_topo_link_t;
pub(crate) type _TopoCallbacksInternal = sys::fd_topo_obj_callbacks_t;
pub(crate) type _TileRunnerInternal = sys::fd_topo_run_tile_t;
pub(crate) type _CpusInternal = sys::fd_topo_cpus_t;

pub type TopoCallbackFn<R> =
    unsafe extern "C" fn(topo: *const ActiveTopology, obj: *const _ObjectInternal) -> R;
pub type TileAnonymousFn<R> = unsafe extern "C" fn() -> R;
pub type TileRunnerFn<A, R> = unsafe extern "C" fn(A) -> R;
pub type TileContextFn = unsafe extern "C" fn(*mut ActiveTopology, *mut ActiveTile);
