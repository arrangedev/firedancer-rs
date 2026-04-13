use core::ffi::c_void;
use core::fmt;
use core::ptr::NonNull;
use fd_clock_sys as sys;
use std::alloc::{self, Layout};

pub const CLOCK_ALIGN: usize = sys::FD_CLOCK_ALIGN as usize;
pub const CLOCK_FOOTPRINT: usize = sys::FD_CLOCK_FOOTPRINT as usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClockError {
    XClockMisbehaved,
    YClockMisbehaved,
    InvalidParams(&'static str),
    InitFailed,
    JoinFailed,
    ShmemOpen(i32),
    ShmemTruncate(i32),
    ShmemMmap(i32),
}

impl ClockError {
    fn from_ffi(code: i32) -> Self {
        match code {
            sys::FD_CLOCK_ERR_X => ClockError::XClockMisbehaved,
            sys::FD_CLOCK_ERR_Y => ClockError::YClockMisbehaved,
            _ => ClockError::XClockMisbehaved,
        }
    }
}

impl fmt::Display for ClockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClockError::XClockMisbehaved => write!(f, "x-clock not well-behaved"),
            ClockError::YClockMisbehaved => write!(f, "y-clock not well-behaved"),
            ClockError::InvalidParams(msg) => write!(f, "invalid parameters: {}", msg),
            ClockError::InitFailed => write!(f, "clock initialization failed"),
            ClockError::JoinFailed => write!(f, "clock join failed"),
            ClockError::ShmemOpen(errno) => write!(f, "shm_open failed (errno {})", errno),
            ClockError::ShmemTruncate(errno) => {
                write!(f, "ftruncate failed (errno {})", errno)
            }
            ClockError::ShmemMmap(errno) => write!(f, "mmap failed (errno {})", errno),
        }
    }
}

impl core::error::Error for ClockError {}

pub type ClockFunc = sys::fd_clock_func_t;

pub struct ClockConfig {
    pub recal_avg: i64,
    pub recal_jit: i64,
    pub recal_hist: f64,
    pub recal_frac: f64,
}

impl ClockConfig {
    pub fn new(recal_avg: i64) -> Self {
        Self {
            recal_avg,
            recal_jit: 0,
            recal_hist: 0.0,
            recal_frac: 0.0,
        }
    }

    pub fn with_jitter(mut self, jit: i64) -> Self {
        self.recal_jit = jit;
        self
    }

    pub fn with_history(mut self, hist: f64) -> Self {
        self.recal_hist = hist;
        self
    }

    pub fn with_frac(mut self, frac: f64) -> Self {
        self.recal_frac = frac;
        self
    }
}

pub struct JointReadResult {
    pub x: i64,
    pub y: i64,
    pub dx: i64,
}

/// Jointly reads two clocks "simultaneously", returning the closest
/// time pair observation. `clock_x` is typically the fast local clock
/// and `clock_y` the slow reference clock.
pub fn joint_read(
    clock_x: ClockFunc,
    args_x: *const c_void,
    clock_y: ClockFunc,
    args_y: *const c_void,
) -> Result<JointReadResult, ClockError> {
    let mut x: i64 = 0;
    let mut y: i64 = 0;
    let mut dx: i64 = 0;

    let err = unsafe {
        sys::fd_clock_joint_read(clock_x, args_x, clock_y, args_y, &mut x, &mut y, &mut dx)
    };

    if err != sys::FD_CLOCK_SUCCESS as i32 {
        return Err(ClockError::from_ffi(err));
    }

    Ok(JointReadResult { x, y, dx })
}

pub fn strerror(err: i32) -> &'static str {
    unsafe {
        let ptr = sys::fd_clock_strerror(err);
        core::ffi::CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
    }
}

enum ShmemBacking {
    /// Caller-provided pointer that was initialized via `fd_clock_new`.
    /// On drop: `fd_clock_delete`, but no memory deallocation.
    OwnedPtr,

    /// Caller-provided pointer to already-initialized shared memory.
    /// On drop: nothing (caller owns both lifecycle and memory).
    BorrowedPtr,

    /// POSIX shared memory created via `shm_open(O_CREAT)` and
    /// initialized via `fd_clock_new`. The owner of the clock.
    /// On drop: `fd_clock_delete`, `munmap`, `close`, `shm_unlink`.
    OwnedPosixShm {
        fd: libc::c_int,
        map_ptr: *mut c_void,
        map_len: usize,
        name: [u8; 256],
    },

    /// POSIX shared memory attached via `shm_open` (no `O_CREAT`).
    /// An observer that does not own the clock lifecycle.
    /// On drop: `munmap`, `close` only.
    BorrowedPosixShm {
        fd: libc::c_int,
        map_ptr: *mut c_void,
        map_len: usize,
    },
}

/// Handle to a `fd_clock` shared memory region.
///
/// Constructors split into two categories:
///
/// **Owners** (create + initialize the clock, manage its full lifecycle):
/// - [`init`](ClockShmem::init): creates a POSIX shared memory segment
///   and initializes a new `fd_clock` inside it.
/// - [`init_from_ptr`](ClockShmem::init_from_ptr): initializes a new
///   `fd_clock` in a caller-provided memory region.
///
/// **Observers** (attach to an already-initialized clock):
/// - [`open`](ClockShmem::open): attaches to an existing POSIX shared
///   memory segment by name.
/// - [`from_ptr`](ClockShmem::from_ptr): wraps a pointer to an
///   already-initialized `fd_clock`.
///
/// Only owners call `fd_clock_delete` and (for POSIX shm) `shm_unlink`
/// on drop. Observers only release their local mapping.
pub struct ClockShmem {
    shclock: NonNull<c_void>,
    backing: ShmemBacking,
}

unsafe impl Send for ClockShmem {}
unsafe impl Sync for ClockShmem {}

fn validate_shm_name(name: &str) -> Result<[u8; 256], ClockError> {
    if name.is_empty() || name.len() >= 255 {
        return Err(ClockError::InvalidParams("shm name too long or empty"));
    }
    let mut buf = [0u8; 256];
    buf[..name.len()].copy_from_slice(name.as_bytes());
    Ok(buf)
}

fn last_os_errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
}

unsafe fn posix_shm_mmap(
    name_ptr: *const libc::c_char,
    flags: libc::c_int,
    len: usize,
) -> Result<(libc::c_int, *mut c_void), ClockError> {
    let fd = libc::shm_open(name_ptr, flags, 0o600);
    if fd < 0 {
        return Err(ClockError::ShmemOpen(last_os_errno()));
    }

    if flags & libc::O_CREAT != 0 {
        if libc::ftruncate(fd, len as libc::off_t) != 0 {
            let errno = last_os_errno();
            libc::close(fd);
            return Err(ClockError::ShmemTruncate(errno));
        }
    }

    let map_ptr = libc::mmap(
        core::ptr::null_mut(),
        len,
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_SHARED,
        fd,
        0,
    );
    if map_ptr == libc::MAP_FAILED {
        let errno = last_os_errno();
        libc::close(fd);
        return Err(ClockError::ShmemMmap(errno));
    }

    Ok((fd, map_ptr))
}

impl ClockShmem {
    /// Creates a POSIX shared memory segment and initializes a new
    /// `fd_clock` inside it. This is the **owner** constructor — on
    /// drop, the clock is deleted and the segment unlinked.
    ///
    /// `name` must be a valid `shm_open` name (typically `/some_name`).
    pub fn init(
        name: &str,
        config: &ClockConfig,
        init_x0: i64,
        init_y0: i64,
        init_w: f64,
    ) -> Result<Self, ClockError> {
        let name_buf = validate_shm_name(name)?;
        let map_len = CLOCK_FOOTPRINT;

        unsafe {
            let (fd, map_ptr) = posix_shm_mmap(
                name_buf.as_ptr() as *const libc::c_char,
                libc::O_CREAT | libc::O_RDWR,
                map_len,
            )?;

            let result = sys::fd_clock_new(
                map_ptr,
                config.recal_avg,
                config.recal_jit,
                config.recal_hist,
                config.recal_frac,
                init_x0,
                init_y0,
                init_w,
            );

            match NonNull::new(result) {
                Some(shclock) => Ok(Self {
                    shclock,
                    backing: ShmemBacking::OwnedPosixShm {
                        fd,
                        map_ptr,
                        map_len,
                        name: name_buf,
                    },
                }),
                None => {
                    libc::munmap(map_ptr, map_len);
                    libc::close(fd);
                    libc::shm_unlink(name_buf.as_ptr() as *const libc::c_char);
                    Err(ClockError::InitFailed)
                }
            }
        }
    }

    /// Attaches to an **existing** POSIX shared memory segment that was
    /// previously initialized by an owner via [`init`](ClockShmem::init).
    ///
    /// This is the **observer** constructor — on drop, the local mapping
    /// is released but the segment is not deleted or unlinked.
    pub fn open(name: &str) -> Result<Self, ClockError> {
        let name_buf = validate_shm_name(name)?;
        let map_len = CLOCK_FOOTPRINT;

        unsafe {
            let (fd, map_ptr) = posix_shm_mmap(
                name_buf.as_ptr() as *const libc::c_char,
                libc::O_RDWR,
                map_len,
            )?;

            let shclock = NonNull::new(map_ptr).ok_or_else(|| {
                libc::munmap(map_ptr, map_len);
                libc::close(fd);
                ClockError::InitFailed
            })?;

            Ok(Self {
                shclock,
                backing: ShmemBacking::BorrowedPosixShm {
                    fd,
                    map_ptr,
                    map_len,
                },
            })
        }
    }

    /// Initializes a new `fd_clock` in a caller-provided memory region.
    /// This is an **owner** constructor — on drop, `fd_clock_delete` is
    /// called but the underlying memory is not freed.
    ///
    /// # Safety
    ///
    /// - `shmem` must point to a region of at least [`CLOCK_FOOTPRINT`]
    ///   bytes with [`CLOCK_ALIGN`] alignment.
    /// - The caller retains ownership of the underlying memory and must
    ///   not free it until after this `ClockShmem` is dropped.
    pub unsafe fn init_from_ptr(
        shmem: *mut c_void,
        config: &ClockConfig,
        init_x0: i64,
        init_y0: i64,
        init_w: f64,
    ) -> Result<Self, ClockError> {
        let result = sys::fd_clock_new(
            shmem,
            config.recal_avg,
            config.recal_jit,
            config.recal_hist,
            config.recal_frac,
            init_x0,
            init_y0,
            init_w,
        );

        let shclock = NonNull::new(result).ok_or(ClockError::InitFailed)?;

        Ok(Self {
            shclock,
            backing: ShmemBacking::OwnedPtr,
        })
    }

    /// Wraps a pointer to an **already-initialized** `fd_clock` shared
    /// memory region. This is an **observer** constructor — on drop,
    /// nothing is done to the underlying memory or clock state.
    ///
    /// # Safety
    ///
    /// - `shclock` must point to a live, initialized `fd_clock_shmem_t`.
    /// - The caller must ensure the pointed-to memory outlives this
    ///   handle and all [`ClockJoin`]s derived from it.
    pub unsafe fn from_ptr(shclock: *mut c_void) -> Result<Self, ClockError> {
        let shclock = NonNull::new(shclock).ok_or(ClockError::InvalidParams("null pointer"))?;

        Ok(Self {
            shclock,
            backing: ShmemBacking::BorrowedPtr,
        })
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.shclock.as_ptr()
    }

    pub fn join(
        &self,
        clock_x: ClockFunc,
        args_x: *const c_void,
    ) -> Result<ClockJoin<'_>, ClockError> {
        ClockJoin::new(self, clock_x, args_x)
    }
}

impl Drop for ClockShmem {
    fn drop(&mut self) {
        unsafe {
            match self.backing {
                ShmemBacking::OwnedPtr => {
                    sys::fd_clock_delete(self.shclock.as_ptr());
                }
                ShmemBacking::BorrowedPtr => {}
                ShmemBacking::OwnedPosixShm {
                    fd,
                    map_ptr,
                    map_len,
                    ref name,
                } => {
                    sys::fd_clock_delete(self.shclock.as_ptr());
                    libc::munmap(map_ptr, map_len);
                    libc::close(fd);
                    libc::shm_unlink(name.as_ptr() as *const libc::c_char);
                }
                ShmemBacking::BorrowedPosixShm {
                    fd,
                    map_ptr,
                    map_len,
                } => {
                    libc::munmap(map_ptr, map_len);
                    libc::close(fd);
                }
            }
        }
    }
}

/// A local join to a `fd_clock` shared memory region.
///
/// Provides the observer API ([`now`](ClockJoin::now)) and calibrator
/// API ([`recal`](ClockJoin::recal), [`step`](ClockJoin::step)).
///
/// Borrows the parent [`ClockShmem`] to ensure the shared memory region
/// outlives all joins. The join itself allocates a small process-local
/// `fd_clock_t` to hold the pointer + clock function.
///
/// Thread safety: observer methods (`now`, accessors) take `&self` and
/// are safe to call from any number of threads concurrently. Calibrator
/// methods (`recal`, `step`) take `&mut self`, enforcing single-writer
/// at the Rust level. This matches the C API's concurrency model.
pub struct ClockJoin<'a> {
    lmem: NonNull<u8>,
    layout: Layout,
    _shmem: &'a ClockShmem,
}

unsafe impl Send for ClockJoin<'_> {}
unsafe impl Sync for ClockJoin<'_> {}

impl<'a> ClockJoin<'a> {
    fn new(
        shmem: &'a ClockShmem,
        clock_x: ClockFunc,
        args_x: *const c_void,
    ) -> Result<Self, ClockError> {
        let lmem_layout = Layout::from_size_align(
            core::mem::size_of::<sys::fd_clock_private>(),
            core::mem::align_of::<sys::fd_clock_private>(),
        )
        .map_err(|_| ClockError::JoinFailed)?;

        let lmem =
            NonNull::new(unsafe { alloc::alloc(lmem_layout) }).ok_or(ClockError::JoinFailed)?;

        let handle = unsafe {
            sys::fd_clock_join(
                lmem.as_ptr() as *mut c_void,
                shmem.as_ptr(),
                clock_x,
                args_x,
            )
        };

        if handle.is_null() {
            unsafe { alloc::dealloc(lmem.as_ptr(), lmem_layout) };
            return Err(ClockError::JoinFailed);
        }

        Ok(Self {
            lmem,
            layout: lmem_layout,
            _shmem: shmem,
        })
    }

    fn as_ptr(&self) -> *mut sys::fd_clock_private {
        self.lmem.as_ptr() as *mut sys::fd_clock_private
    }

    fn as_const_ptr(&self) -> *const c_void {
        self.lmem.as_ptr() as *const c_void
    }

    /// Returns an estimate of the y-clock by reading the local x-clock
    /// and converting via the most recently calibrated epoch parameters.
    pub fn now(&self) -> i64 {
        unsafe { sys::fd_clock_now(self.as_const_ptr()) }
    }

    /// Returns the recommended y-clock time for the next recalibration.
    pub fn recal_next(&self) -> i64 {
        unsafe { sys::fd_clock_recal_next(self.as_ptr()) }
    }

    /// Returns the number of clock errors detected since creation or
    /// last reset.
    pub fn err_cnt(&self) -> u64 {
        unsafe { sys::fd_clock_err_cnt(self.as_ptr()) }
    }

    /// Resets the clock error counter.
    pub fn reset_err_cnt(&mut self) {
        unsafe { sys::fd_clock_reset_err_cnt(self.as_ptr()) }
    }

    /// Ends the current epoch and starts a new one from the joint
    /// observation (x1, y1), preserving y-clock estimate monotonicity.
    /// Returns the recommended next recalibration time.
    pub fn recal(&mut self, x1: i64, y1: i64) -> i64 {
        unsafe { sys::fd_clock_recal(self.as_ptr(), x1, y1) }
    }

    /// Ends the current epoch and steps the clock to (x0, y0) with the
    /// given x-tick-per-y-tick rate `w`. Does not preserve monotonicity.
    /// Returns the recommended next recalibration time.
    pub fn step(&mut self, x0: i64, y0: i64, w: f64) -> i64 {
        unsafe { sys::fd_clock_step(self.as_ptr(), x0, y0, w) }
    }

    /// Returns a const pointer to the underlying shared memory, for use
    /// with the advanced [`ClockEpoch`] API.
    pub fn shclock_ptr(&self) -> *const sys::fd_clock_shmem_private {
        unsafe { sys::fd_clock_shclock_const(self.as_ptr()) as *const _ }
    }

    pub fn recal_avg(&self) -> i64 {
        unsafe { sys::fd_clock_recal_avg(self.as_ptr()) }
    }

    pub fn recal_jit(&self) -> i64 {
        unsafe { sys::fd_clock_recal_jit(self.as_ptr()) }
    }

    pub fn recal_hist(&self) -> f64 {
        unsafe { sys::fd_clock_recal_hist(self.as_ptr()) }
    }

    pub fn recal_frac(&self) -> f64 {
        unsafe { sys::fd_clock_recal_frac(self.as_ptr()) }
    }

    pub fn init_x0(&self) -> i64 {
        unsafe { sys::fd_clock_init_x0(self.as_ptr()) }
    }

    pub fn init_y0(&self) -> i64 {
        unsafe { sys::fd_clock_init_y0(self.as_ptr()) }
    }

    pub fn init_w(&self) -> f64 {
        unsafe { sys::fd_clock_init_w(self.as_ptr()) }
    }
}

impl<'a> Drop for ClockJoin<'a> {
    fn drop(&mut self) {
        unsafe {
            sys::fd_clock_leave(self.as_ptr());
            alloc::dealloc(self.lmem.as_ptr(), self.layout);
        }
    }
}

/// Cache of a single clock epoch's synchronization parameters for
/// ultra-low-overhead y-clock estimation in hot paths.
///
/// Usage: call [`refresh`](ClockEpoch::refresh) during periodic
/// housekeeping (much more often than recal_avg), then call
/// [`estimate_y`](ClockEpoch::estimate_y) with an x-clock observation
/// on the critical path.
///
/// `ClockEpoch` is `Send` but intentionally **not** `Sync`. It is a
/// per-thread cache with no internal synchronization. In multi-threaded
/// observers, each thread should create its own `ClockEpoch` from the
/// shared `shclock_ptr()`.
pub struct ClockEpoch {
    inner: sys::fd_clock_epoch_private,
}

unsafe impl Send for ClockEpoch {}

impl ClockEpoch {
    /// Initializes the epoch from the clock's current published epoch.
    ///
    /// # Safety
    ///
    /// `shclock` must point to valid `fd_clock_shmem_t` shared memory
    /// that remains live for the duration of this call.
    pub unsafe fn init(shclock: *const sys::fd_clock_shmem_private) -> Self {
        let mut inner = core::mem::MaybeUninit::<sys::fd_clock_epoch_private>::uninit();
        sys::fd_clock_epoch_init(inner.as_mut_ptr(), shclock);
        Self {
            inner: inner.assume_init(),
        }
    }

    /// Refreshes the cached epoch if a newer epoch has been published.
    ///
    /// # Safety
    ///
    /// `shclock` must point to valid `fd_clock_shmem_t` shared memory
    /// that remains live for the duration of this call.
    pub unsafe fn refresh(&mut self, shclock: *const sys::fd_clock_shmem_private) {
        sys::fd_clock_epoch_refresh(&mut self.inner, shclock);
    }

    /// Estimates the y-clock value from an x-clock observation `x`.
    /// O(1) ns overhead with no memory fences.
    pub fn estimate_y(&self, x: i64) -> i64 {
        unsafe { sys::fd_clock_epoch_y(&self.inner, x) }
    }

    pub fn x0(&self) -> i64 {
        unsafe { sys::fd_clock_epoch_x0(&self.inner) }
    }

    pub fn y0(&self) -> i64 {
        unsafe { sys::fd_clock_epoch_y0(&self.inner) }
    }

    pub fn w(&self) -> f64 {
        unsafe { sys::fd_clock_epoch_w(&self.inner) }
    }

    pub fn y0_eff(&self) -> i64 {
        unsafe { sys::fd_clock_epoch_y0_eff(&self.inner) }
    }

    pub fn m(&self) -> f64 {
        unsafe { sys::fd_clock_epoch_m(&self.inner) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};

    static MOCK_TICK: AtomicI64 = AtomicI64::new(1_000_000);

    unsafe extern "C" fn mock_clock_x(_args: *const c_void) -> i64 {
        MOCK_TICK.fetch_add(100, Ordering::Relaxed)
    }

    unsafe extern "C" fn mock_clock_y(_args: *const c_void) -> i64 {
        MOCK_TICK.fetch_add(1, Ordering::Relaxed)
    }

    #[test]
    fn test_strerror() {
        let s = strerror(sys::FD_CLOCK_SUCCESS as i32);
        assert!(!s.is_empty());
        assert_eq!(s, "success");

        let s = strerror(sys::FD_CLOCK_ERR_X);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_init_from_ptr() {
        MOCK_TICK.store(1_000_000, Ordering::Relaxed);

        let config = ClockConfig::new(10_000_000);
        let init_x0: i64 = 1_000_000;
        let init_y0: i64 = 1_000_000;
        let init_w: f64 = 100.0;

        let layout = Layout::from_size_align(CLOCK_FOOTPRINT, CLOCK_ALIGN).unwrap();
        let raw = unsafe { alloc::alloc(layout) };
        assert!(!raw.is_null());

        let shmem = unsafe {
            ClockShmem::init_from_ptr(raw as *mut c_void, &config, init_x0, init_y0, init_w)
                .unwrap()
        };

        let join = shmem.join(Some(mock_clock_x), core::ptr::null()).unwrap();

        assert_eq!(join.init_x0(), init_x0);
        assert_eq!(join.init_y0(), init_y0);
        assert_eq!(join.init_w(), init_w);
        assert_eq!(join.recal_avg(), 10_000_000);
        assert_eq!(join.err_cnt(), 0);

        let now = join.now();
        assert!(now >= init_y0);

        drop(join);
        drop(shmem);
        unsafe { alloc::dealloc(raw, layout) };
    }

    #[test]
    fn test_init_posix_shm() {
        MOCK_TICK.store(1_000_000, Ordering::Relaxed);

        let config = ClockConfig::new(10_000_000);
        let name = "/fd_clock_test_posix\0";

        let shmem = ClockShmem::init(name, &config, 1_000_000, 1_000_000, 100.0).unwrap();

        let join = shmem.join(Some(mock_clock_x), core::ptr::null()).unwrap();

        let now = join.now();
        assert!(now >= 1_000_000);
        assert_eq!(join.recal_avg(), 10_000_000);

        drop(join);
        drop(shmem);
    }

    #[test]
    fn test_recal() {
        MOCK_TICK.store(1_000_000, Ordering::Relaxed);

        let config = ClockConfig::new(10_000_000);
        let name = "/fd_clock_test_recal\0";

        let shmem = ClockShmem::init(name, &config, 1_000_000, 1_000_000, 100.0).unwrap();

        let mut join = shmem.join(Some(mock_clock_x), core::ptr::null()).unwrap();

        let next = join.recal(2_000_000, 1_010_000);
        assert!(next > 1_010_000);
    }

    #[test]
    fn test_step() {
        MOCK_TICK.store(1_000_000, Ordering::Relaxed);

        let config = ClockConfig::new(10_000_000);
        let name = "/fd_clock_test_step\0";

        let shmem = ClockShmem::init(name, &config, 1_000_000, 1_000_000, 100.0).unwrap();

        let mut join = shmem.join(Some(mock_clock_x), core::ptr::null()).unwrap();

        let next = join.step(5_000_000, 5_000_000, 100.0);
        assert!(next > 5_000_000);
    }

    #[test]
    fn test_config_builder() {
        let config = ClockConfig::new(10_000_000)
            .with_jitter(100_000)
            .with_history(5.0)
            .with_frac(0.8);

        assert_eq!(config.recal_avg, 10_000_000);
        assert_eq!(config.recal_jit, 100_000);
        assert_eq!(config.recal_hist, 5.0);
        assert_eq!(config.recal_frac, 0.8);
    }

    #[test]
    fn test_joint_read() {
        MOCK_TICK.store(1_000_000, Ordering::Relaxed);

        let result = joint_read(
            Some(mock_clock_x),
            core::ptr::null(),
            Some(mock_clock_y),
            core::ptr::null(),
        );

        assert!(result.is_ok());
        let jr = result.unwrap();
        assert!(jr.x > 0);
        assert!(jr.y > 0);
        assert!(jr.dx >= 0);
    }

    #[test]
    fn test_multiple_joins() {
        MOCK_TICK.store(1_000_000, Ordering::Relaxed);

        let config = ClockConfig::new(10_000_000);
        let name = "/fd_clock_test_multi\0";

        let shmem = ClockShmem::init(name, &config, 1_000_000, 1_000_000, 100.0).unwrap();

        let join1 = shmem.join(Some(mock_clock_x), core::ptr::null()).unwrap();
        let join2 = shmem.join(Some(mock_clock_x), core::ptr::null()).unwrap();

        let now1 = join1.now();
        let now2 = join2.now();

        assert!(now1 >= 1_000_000);
        assert!(now2 >= 1_000_000);
    }

    #[test]
    fn test_err_cnt_reset() {
        MOCK_TICK.store(1_000_000, Ordering::Relaxed);

        let config = ClockConfig::new(10_000_000);
        let name = "/fd_clock_test_errcnt\0";

        let shmem = ClockShmem::init(name, &config, 1_000_000, 1_000_000, 100.0).unwrap();

        let mut join = shmem.join(Some(mock_clock_x), core::ptr::null()).unwrap();

        assert_eq!(join.err_cnt(), 0);
        join.reset_err_cnt();
        assert_eq!(join.err_cnt(), 0);
    }

    #[test]
    fn test_open_attaches_to_existing() {
        MOCK_TICK.store(1_000_000, Ordering::Relaxed);

        let config = ClockConfig::new(10_000_000);
        let name = "/fd_clock_test_open\0";

        let owner = ClockShmem::init(name, &config, 1_000_000, 1_000_000, 100.0).unwrap();

        let mut owner_join = owner.join(Some(mock_clock_x), core::ptr::null()).unwrap();

        owner_join.recal(2_000_000, 1_010_000);

        let observer = ClockShmem::open(name).unwrap();
        let obs_join = observer
            .join(Some(mock_clock_x), core::ptr::null())
            .unwrap();

        assert_eq!(obs_join.recal_avg(), 10_000_000);
        assert_eq!(obs_join.init_w(), 100.0);

        let now = obs_join.now();
        assert!(now >= 1_000_000);

        drop(obs_join);
        drop(observer);
        drop(owner_join);
        drop(owner);
    }

    #[test]
    fn test_from_ptr_observer() {
        MOCK_TICK.store(1_000_000, Ordering::Relaxed);

        let config = ClockConfig::new(10_000_000);
        let layout = Layout::from_size_align(CLOCK_FOOTPRINT, CLOCK_ALIGN).unwrap();
        let raw = unsafe { alloc::alloc(layout) };
        assert!(!raw.is_null());

        let owner = unsafe {
            ClockShmem::init_from_ptr(raw as *mut c_void, &config, 1_000_000, 1_000_000, 100.0)
                .unwrap()
        };

        let observer = unsafe { ClockShmem::from_ptr(owner.as_ptr()).unwrap() };
        let obs_join = observer
            .join(Some(mock_clock_x), core::ptr::null())
            .unwrap();

        assert_eq!(obs_join.recal_avg(), 10_000_000);
        let now = obs_join.now();
        assert!(now >= 1_000_000);

        drop(obs_join);
        drop(observer);
        drop(owner);
        unsafe { alloc::dealloc(raw, layout) };
    }

    #[test]
    fn test_open_nonexistent_fails() {
        let result = ClockShmem::open("/fd_clock_does_not_exist_xyz\0");
        assert!(matches!(result, Err(ClockError::ShmemOpen(_))));
    }

    #[test]
    fn test_from_ptr_null_fails() {
        let result = unsafe { ClockShmem::from_ptr(core::ptr::null_mut()) };
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_observers_same_segment() {
        MOCK_TICK.store(1_000_000, Ordering::Relaxed);

        let config = ClockConfig::new(10_000_000);
        let name = "/fd_clock_test_multi_obs\0";

        let owner = ClockShmem::init(name, &config, 1_000_000, 1_000_000, 100.0).unwrap();
        let _owner_join = owner.join(Some(mock_clock_x), core::ptr::null()).unwrap();

        let obs1 = ClockShmem::open(name).unwrap();
        let obs2 = ClockShmem::open(name).unwrap();

        let j1 = obs1.join(Some(mock_clock_x), core::ptr::null()).unwrap();
        let j2 = obs2.join(Some(mock_clock_x), core::ptr::null()).unwrap();

        let now1 = j1.now();
        let now2 = j2.now();
        assert!(now1 >= 1_000_000);
        assert!(now2 >= 1_000_000);
    }

    #[test]
    fn test_sync_send_bounds() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<ClockShmem>();
        assert_sync::<ClockShmem>();
        assert_send::<ClockJoin<'_>>();
        assert_sync::<ClockJoin<'_>>();
        assert_send::<ClockEpoch>();
    }

    #[test]
    fn test_shared_join_across_threads() {
        MOCK_TICK.store(1_000_000, Ordering::Relaxed);

        let config = ClockConfig::new(10_000_000);
        let name = "/fd_clock_test_threads\0";

        let shmem = ClockShmem::init(name, &config, 1_000_000, 1_000_000, 100.0).unwrap();
        let join = shmem.join(Some(mock_clock_x), core::ptr::null()).unwrap();

        let join_ref = &join;

        std::thread::scope(|s| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    s.spawn(|| {
                        let mut prev = 0i64;
                        for _ in 0..1000 {
                            let now = join_ref.now();
                            assert!(now >= prev);
                            prev = now;
                        }
                        prev
                    })
                })
                .collect();

            for h in handles {
                let last = h.join().unwrap();
                assert!(last >= 1_000_000);
            }
        });
    }

    #[test]
    fn test_epoch_per_thread() {
        MOCK_TICK.store(1_000_000, Ordering::Relaxed);

        let config = ClockConfig::new(10_000_000);
        let name = "/fd_clock_test_epoch_mt\0";

        let shmem = ClockShmem::init(name, &config, 1_000_000, 1_000_000, 100.0).unwrap();
        let join = shmem.join(Some(mock_clock_x), core::ptr::null()).unwrap();

        let join_ref = &join;

        std::thread::scope(|s| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    s.spawn(|| {
                        let ptr = join_ref.shclock_ptr();
                        let mut epoch = unsafe { ClockEpoch::init(ptr) };

                        for i in 0..1000 {
                            let x = MOCK_TICK.fetch_add(100, Ordering::Relaxed);
                            let _y = epoch.estimate_y(x);

                            if i % 250 == 0 {
                                unsafe { epoch.refresh(ptr) };
                            }
                        }
                    })
                })
                .collect();

            for h in handles {
                h.join().unwrap();
            }
        });
    }
}
