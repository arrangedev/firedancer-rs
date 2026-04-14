use core::ffi::c_void;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fd_clock::{joint_read, ClockConfig, ClockEpoch, ClockShmem};

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

fn make_calibrated_clock(name: &str) -> ClockShmem {
    let jr0 = joint_read(
        Some(tickcount),
        core::ptr::null(),
        Some(wallclock_ns),
        core::ptr::null(),
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(20));

    let jr1 = joint_read(
        Some(tickcount),
        core::ptr::null(),
        Some(wallclock_ns),
        core::ptr::null(),
    )
    .unwrap();

    let init_w = (jr1.x - jr0.x) as f64 / (jr1.y - jr0.y) as f64;
    let config = ClockConfig::new(10_000_000);

    ClockShmem::init(name, &config, jr1.x, jr1.y, init_w).unwrap()
}

fn observer_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("observer");

    let shmem = make_calibrated_clock("/fd_clock_bench_obs\0");
    let join = shmem.join(Some(tickcount), core::ptr::null()).unwrap();

    g.bench_function("fd_clock_now", |b| {
        b.iter(|| black_box(join.now()));
    });

    g.bench_function("clock_gettime_realtime", |b| {
        b.iter(|| {
            black_box(|| {
                let mut ts = core::mem::MaybeUninit::<libc::timespec>::uninit();
                unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, ts.as_mut_ptr()) };
                let ts = unsafe { ts.assume_init() };
                ts.tv_sec * 1_000_000_000 + ts.tv_nsec
            })
        });
    });

    g.bench_function("clock_gettime_monotonic_raw", |b| {
        b.iter(|| {
            black_box(|| {
                let mut ts = core::mem::MaybeUninit::<libc::timespec>::uninit();
                unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC_RAW, ts.as_mut_ptr()) };
                let ts = unsafe { ts.assume_init() };
                ts.tv_sec * 1_000_000_000 + ts.tv_nsec
            })
        });
    });

    g.bench_function("std::SystemTime::now", |b| {
        b.iter(|| {
            black_box(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos(),
            )
        });
    });

    #[cfg(target_arch = "x86_64")]
    g.bench_function("rdtsc", |b| {
        b.iter(|| black_box(unsafe { core::arch::x86_64::_rdtsc() }));
    });

    g.finish();
}

fn epoch_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("epoch");

    let shmem = make_calibrated_clock("/fd_clock_bench_epoch\0");
    let mut join = shmem.join(Some(tickcount), core::ptr::null()).unwrap();

    let jr = joint_read(
        Some(tickcount),
        core::ptr::null(),
        Some(wallclock_ns),
        core::ptr::null(),
    )
    .unwrap();
    join.recal(jr.x, jr.y);

    let shclock_ptr = join.shclock_ptr();
    let epoch = unsafe { ClockEpoch::init(shclock_ptr) };

    g.bench_function("epoch_estimate_y", |b| {
        b.iter(|| {
            let x = unsafe { tickcount(core::ptr::null()) };
            black_box(epoch.estimate_y(black_box(x)))
        });
    });

    g.bench_function("epoch_estimate_y_no_xclock", |b| {
        let x_fixed = unsafe { tickcount(core::ptr::null()) };
        b.iter(|| black_box(epoch.estimate_y(black_box(x_fixed))));
    });

    g.finish();
}

fn calibration_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("calibration");

    let shmem = make_calibrated_clock("/fd_clock_bench_cal\0");
    let mut join = shmem.join(Some(tickcount), core::ptr::null()).unwrap();

    g.bench_function("joint_read", |b| {
        b.iter(|| {
            black_box(
                joint_read(
                    Some(tickcount),
                    core::ptr::null(),
                    Some(wallclock_ns),
                    core::ptr::null(),
                )
                .unwrap(),
            )
        });
    });

    g.bench_function("recal", |b| {
        b.iter(|| {
            let jr = joint_read(
                Some(tickcount),
                core::ptr::null(),
                Some(wallclock_ns),
                core::ptr::null(),
            )
            .unwrap();
            black_box(join.recal(jr.x, jr.y))
        });
    });

    g.bench_function("step", |b| {
        b.iter(|| {
            let jr = joint_read(
                Some(tickcount),
                core::ptr::null(),
                Some(wallclock_ns),
                core::ptr::null(),
            )
            .unwrap();
            black_box(join.step(jr.x, jr.y, 100.0))
        });
    });

    g.finish();
}

fn lifecycle_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("lifecycle");

    let shmem = make_calibrated_clock("/fd_clock_bench_life\0");

    g.bench_function("join_leave", |b| {
        b.iter(|| {
            let join = shmem.join(Some(tickcount), core::ptr::null()).unwrap();
            black_box(&join);
            drop(join);
        });
    });

    g.finish();
}

criterion_group!(
    benches,
    observer_benches,
    epoch_benches,
    calibration_benches,
    lifecycle_benches,
);
criterion_main!(benches);
