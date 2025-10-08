use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tempfile::NamedTempFile;

// Firedancer
use fd_log::{debug, error, info, warn, LogLevel, SystemLogBuilder};

// Alternatives
use log::{debug as log_debug, error as log_error, info as log_info, warn as log_warn};
use tracing::{
    debug as tracing_debug, error as tracing_error, info as tracing_info, warn as tracing_warn,
};

fn setup_fd_log() -> NamedTempFile {
    let temp_file = NamedTempFile::new().unwrap();
    SystemLogBuilder::default()
        .with_file(temp_file.path().to_str().unwrap())
        .with_stderr_level(LogLevel::Emergency)
        .with_logfile_level(LogLevel::Debug)
        .init()
        .expect("Failed to initialize fd_log");
    temp_file
}

fn setup_env_logger() -> NamedTempFile {
    let temp_file = NamedTempFile::new().unwrap();
    let target = Box::new(std::fs::File::create(temp_file.path()).unwrap());

    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(target))
        .filter_level(log::LevelFilter::Debug)
        .init();

    temp_file
}

fn setup_tracing() -> NamedTempFile {
    let temp_file = NamedTempFile::new().unwrap();
    let file = std::fs::File::create(temp_file.path()).unwrap();

    let subscriber = tracing_subscriber::fmt()
        .with_writer(Arc::new(file))
        .with_max_level(tracing::Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    temp_file
}

fn bench_logging_simple_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("logging_simple_messages");
    let _fd_temp = setup_fd_log();

    group.bench_function("fd_log_info", |b| {
        b.iter(|| {
            info!("Simple info message");
            black_box(())
        })
    });

    group.bench_function("fd_log_warn", |b| {
        b.iter(|| {
            warn!("Simple warning message");
            black_box(())
        })
    });

    group.bench_function("fd_log_debug", |b| {
        b.iter(|| {
            debug!("Simple debug message");
            black_box(())
        })
    });

    group.finish();
}

fn bench_logging_formatted_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("logging_formatted_messages");

    let user_id = 12345;
    let operation = "user_login";
    let timestamp = 1634567890;

    group.bench_function("fd_log_formatted", |b| {
        b.iter(|| {
            info!("User {} performed {} at {}", user_id, operation, timestamp);
            black_box(())
        })
    });

    group.bench_function("fd_log_structured", |b| {
        b.iter(|| {
            info!(
                user_id = user_id,
                operation = operation,
                timestamp = timestamp,
                "User operation completed"
            );
            black_box(())
        })
    });

    group.finish();
}

fn bench_logging_with_context(c: &mut Criterion) {
    let mut group = c.benchmark_group("logging_with_context");

    let request_id = "req_123456789";
    let user_agent = "Mozilla/5.0 (compatible; benchmark)";
    let ip_address = "192.168.1.100";
    let response_time_ms = 42;

    group.bench_function("fd_log_complex_structured", |b| {
        b.iter(|| {
            info!(
                request_id = request_id,
                user_agent = user_agent,
                ip_address = ip_address,
                response_time_ms = response_time_ms,
                status_code = 200,
                "HTTP request processed successfully"
            );
            black_box(())
        })
    });

    group.bench_function("fd_log_complex_formatted", |b| {
        b.iter(|| {
            info!(
                "HTTP request processed: {} from {} (UA: {}) - {}ms - 200 OK",
                request_id, ip_address, user_agent, response_time_ms
            );
            black_box(())
        })
    });

    group.finish();
}

fn bench_logging_disabled_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("logging_disabled_levels");

    group.bench_function("fd_log_debug_disabled", |b| {
        b.iter(|| {
            debug!("This debug message should be filtered out");
            black_box(())
        })
    });

    group.bench_function("fd_log_info_enabled", |b| {
        b.iter(|| {
            info!("This info message should be logged");
            black_box(())
        })
    });

    group.finish();
}

fn bench_logging_hexdump(c: &mut Criterion) {
    let mut group = c.benchmark_group("logging_hexdump");

    let small_data = vec![0x41, 0x42, 0x43, 0x44]; // "ABCD"
    let medium_data = vec![0u8; 256];
    let large_data = vec![0u8; 4096];

    group.bench_function("fd_log_hexdump_small", |b| {
        b.iter(|| {
            fd_log::info_hexdump!("small_packet", &small_data);
            black_box(())
        })
    });

    group.bench_function("fd_log_hexdump_medium", |b| {
        b.iter(|| {
            fd_log::info_hexdump!("medium_packet", &medium_data);
            black_box(())
        })
    });

    group.bench_function("fd_log_hexdump_large", |b| {
        b.iter(|| {
            fd_log::info_hexdump!("large_packet", &large_data);
            black_box(())
        })
    });

    group.finish();
}

fn bench_logging_wallclock(c: &mut Criterion) {
    let mut group = c.benchmark_group("logging_wallclock");

    group.bench_function("fd_log_wallclock", |b| {
        b.iter(|| black_box(fd_log::SystemLogger::wallclock()))
    });

    group.bench_function("fd_log_wallclock_host", |b| {
        b.iter(|| black_box(fd_log::SystemLogger::wallclock_host()))
    });

    group.bench_function("std_system_time", |b| {
        b.iter(|| black_box(std::time::SystemTime::now()))
    });

    group.finish();
}

// Separate benchmarks for other logging frameworks would need to be run
// in separate processes due to global state conflicts
fn bench_log_crate_comparison(c: &mut Criterion) {
    // This would be run separately with env_logger initialized
    let mut group = c.benchmark_group("log_crate_comparison");

    // These benchmarks would be uncommented and run separately:
    /*
    group.bench_function("log_crate_info", |b| {
        b.iter(|| {
            log_info!("Simple info message");
            black_box(())
        })
    });

    group.bench_function("log_crate_formatted", |b| {
        b.iter(|| {
            log_info!("User {} performed {} at {}", 12345, "login", 1634567890);
            black_box(())
        })
    });
    */

    group.finish();
}

fn bench_tracing_comparison(c: &mut Criterion) {
    // This would be run separately with tracing initialized
    let mut group = c.benchmark_group("tracing_comparison");

    // These benchmarks would be uncommented and run separately:
    /*
    group.bench_function("tracing_info", |b| {
        b.iter(|| {
            tracing_info!("Simple info message");
            black_box(())
        })
    });

    group.bench_function("tracing_structured", |b| {
        b.iter(|| {
            tracing_info!(
                user_id = 12345,
                operation = "login",
                timestamp = 1634567890,
                "User operation completed"
            );
            black_box(())
        })
    });
    */

    group.finish();
}

criterion_group!(
    logging_benches,
    bench_logging_simple_messages,
    bench_logging_formatted_messages,
    bench_logging_with_context,
    bench_logging_disabled_levels,
    bench_logging_hexdump,
    bench_logging_wallclock,
    bench_log_crate_comparison,
    bench_tracing_comparison
);
criterion_main!(logging_benches);
