use fd_log::{fd_dbg, fd_info, fd_notice, fd_warn, FdLog, LogLevel};

fn main() {
    fd_info!("  app_id: {}", FdLog::app_id());
    fd_info!("  thread_id: {}", FdLog::thread_id());
    fd_info!("  host_id: {}", FdLog::host_id());
    fd_info!("  cpu_id: {}", FdLog::cpu_id());
    fd_info!("  group_id: {}", FdLog::group_id());
    fd_info!("  tid: {}", FdLog::tid());
    fd_info!("  user_id: {}", FdLog::user_id());
    fd_info!("  app: {}", FdLog::app());
    fd_info!("  thread: {}", FdLog::thread());
    fd_info!("  host: {}", FdLog::host());
    fd_info!("  cpu: {}", FdLog::cpu());
    fd_info!("  group: {}", FdLog::group());
    fd_info!("  user: {}", FdLog::user());
    fd_info!("  wallclock_host: {}", FdLog::wallclock_host());
    fd_info!("  wallclock: {}", FdLog::wallclock());

    fd_info!("  logfile_level: {:?}", FdLog::level_logfile());
    fd_info!("  stderr_level: {:?}", FdLog::level_stderr());
    fd_info!("  flush_level: {:?}", FdLog::level_flush());
    fd_info!("  core_level: {:?}", FdLog::level_core());
    fd_info!("  colorize?: {}", FdLog::colorize());

    FdLog::set_thread("demo-thread");
    fd_info!("  new_thread_name: {}", FdLog::thread());
    println!();

    FdLog::set_cpu("demo-cpu");
    fd_info!("  new_cpu_name: {}", FdLog::cpu());
    println!();

    FdLog::set_level_stderr(LogLevel::Warning);
    fd_info!("  new_stderr_level: {:?}", FdLog::level_stderr());
    println!();

    let current_colorize = FdLog::colorize();
    FdLog::set_colorize(!current_colorize);
    fd_info!(
        "  set_colorize: {} -> {}",
        current_colorize,
        FdLog::colorize()
    );
    println!();

    fd_dbg!("debug message from crabland! format: {}", 42069);
    fd_notice!("notice from crabland! format: {}", 42069);
    fd_warn!("warning from crabland! format: {}", 42069);

    fd_info!("check stderr");
}
