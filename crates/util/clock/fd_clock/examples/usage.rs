use core::ffi::c_void;
use fd_clock::{joint_read, ClockConfig, ClockEpoch, ClockShmem};

const SHM_NAME: &str = "/fd_clock_example\0";

unsafe extern "C" fn tickcount(_args: *const c_void) -> i64 {
    #[cfg(target_arch = "x86_64")]
    {
        core::arch::x86_64::_rdtsc() as i64
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let mut ts = core::mem::MaybeUninit::<libc::timespec>::uninit();
        libc::clock_gettime(libc::CLOCK_MONOTONIC_RAW, ts.as_mut_ptr());
        let ts = ts.assume_init();
        ts.tv_sec * 1_000_000_000 + ts.tv_nsec
    }
}

unsafe extern "C" fn wallclock_ns(_args: *const c_void) -> i64 {
    let mut ts = core::mem::MaybeUninit::<libc::timespec>::uninit();
    libc::clock_gettime(libc::CLOCK_REALTIME, ts.as_mut_ptr());
    let ts = ts.assume_init();
    ts.tv_sec * 1_000_000_000 + ts.tv_nsec
}

fn calibrate_initial_rate() -> (i64, i64, f64) {
    let jr0 = joint_read(
        Some(tickcount),
        core::ptr::null(),
        Some(wallclock_ns),
        core::ptr::null(),
    )
    .expect("joint_read failed");

    std::thread::sleep(std::time::Duration::from_millis(50));

    let jr1 = joint_read(
        Some(tickcount),
        core::ptr::null(),
        Some(wallclock_ns),
        core::ptr::null(),
    )
    .expect("joint_read failed");

    let w = (jr1.x - jr0.x) as f64 / (jr1.y - jr0.y) as f64;
    (jr1.x, jr1.y, w)
}

fn run_ctl() {
    println!("[ctl] calibrating initial rate...");
    let (x0, y0, w) = calibrate_initial_rate();
    println!("[ctl] rate: {:.6} x-ticks/ns", w);

    let config = ClockConfig::new(10_000_000);
    let shmem = ClockShmem::init(SHM_NAME, &config, x0, y0, w).expect("ClockShmem::init failed");

    let mut join = shmem
        .join(Some(tickcount), core::ptr::null())
        .expect("join failed");

    println!("[ctl] clock created on {}", SHM_NAME.trim_end_matches('\0'));
    println!("[ctl] spawning workers...\n");

    let handles: Vec<_> = (0..3)
        .map(|id| std::thread::spawn(move || run_worker(id)))
        .collect();

    // Give workers time to attach before we start calibrating
    std::thread::sleep(std::time::Duration::from_millis(5));

    let mut recal_count = 0u32;
    let mut recal_next = join.recal_next();

    loop {
        let now = join.now();
        if now >= recal_next {
            let jr = joint_read(
                Some(tickcount),
                core::ptr::null(),
                Some(wallclock_ns),
                core::ptr::null(),
            )
            .expect("recal joint_read failed");
            recal_next = join.recal(jr.x, jr.y);
            recal_count += 1;

            if recal_count >= 50 {
                break;
            }
        }
        std::hint::spin_loop();
    }

    println!(
        "\n[ctl] {} recalibrations complete, shutting down",
        recal_count
    );

    for h in handles {
        let _ = h.join();
    }
    // shmem dropped here → fd_clock_delete + shm_unlink
}

fn run_worker(id: u32) {
    let shmem = ClockShmem::open(SHM_NAME).expect("ClockShmem::open failed");

    // Observer API: ~25 ns per call, no syscall
    let join = shmem
        .join(Some(tickcount), core::ptr::null())
        .expect("join failed");

    let mut obs_count: u64 = 0;
    let mut max_err: i64 = 0;
    let start = join.now();

    while join.now() - start < 200_000_000 {
        let est = join.now();
        let actual = unsafe { wallclock_ns(core::ptr::null()) };
        max_err = max_err.max((actual - est).abs());
        obs_count += 1;
    }

    println!(
        "[worker-{}] observer API:  {:>10} obs, max |err| = {} ns",
        id, obs_count, max_err,
    );

    // Epoch API: ~1 ns per estimate (pure arithmetic, no fences)
    let shclock_ptr = join.shclock_ptr();
    let mut epoch = unsafe { ClockEpoch::init(shclock_ptr) };

    let mut epoch_count: u64 = 0;
    let mut epoch_max_err: i64 = 0;
    let start = unsafe { tickcount(core::ptr::null()) };

    for i in 0u64..2_000_000 {
        let x = unsafe { tickcount(core::ptr::null()) };
        let est = epoch.estimate_y(x);
        let actual = unsafe { wallclock_ns(core::ptr::null()) };
        epoch_max_err = epoch_max_err.max((actual - est).abs());
        epoch_count += 1;

        if i % 500_000 == 0 && i > 0 {
            unsafe { epoch.refresh(shclock_ptr) };
        }
    }

    let end = unsafe { tickcount(core::ptr::null()) };
    let ticks_per = (end - start) as f64 / epoch_count as f64;

    println!(
        "[worker-{}] epoch API:     {:>10} obs, max |err| = {} ns, {:.1} ticks/est",
        id, epoch_count, epoch_max_err, ticks_per,
    );
    // shmem dropped here → munmap + close only, no unlink
}

fn main() {
    println!("  ctl      → ClockShmem::init() → owns + calibrates");
    println!("  workers  → ClockShmem::open()  → observe only\n");

    run_ctl();
}
