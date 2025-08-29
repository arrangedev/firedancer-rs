use libfd_log::{fd_log_debug, fd_log_info, fd_log_notice, fd_log_warning, FdLog, LogLevel};

fn main() {
    fd_log_info!("  app_id: {}", FdLog::app_id());
    fd_log_info!("  thread_id: {}", FdLog::thread_id());
    fd_log_info!("  host_id: {}", FdLog::host_id());
    fd_log_info!("  cpu_id: {}", FdLog::cpu_id());
    fd_log_info!("  group_id: {}", FdLog::group_id());
    fd_log_info!("  tid: {}", FdLog::tid());
    fd_log_info!("  user_id: {}", FdLog::user_id());
    fd_log_info!("  app: {}", FdLog::app());
    fd_log_info!("  thread: {}", FdLog::thread());
    fd_log_info!("  host: {}", FdLog::host());
    fd_log_info!("  cpu: {}", FdLog::cpu());
    fd_log_info!("  group: {}", FdLog::group());
    fd_log_info!("  user: {}", FdLog::user());
    fd_log_info!("  wallclock_host: {}", FdLog::wallclock_host());
    fd_log_info!("  wallclock: {}", FdLog::wallclock());

    fd_log_info!("  logfile_level: {:?}", FdLog::level_logfile());
    fd_log_info!("  stderr_level: {:?}", FdLog::level_stderr());
    fd_log_info!("  flush_level: {:?}", FdLog::level_flush());
    fd_log_info!("  core_level: {:?}", FdLog::level_core());
    fd_log_info!("  colorize?: {}", FdLog::colorize());

    FdLog::set_thread("demo-thread");
    fd_log_info!("  new_thread_name: {}", FdLog::thread());
    println!();

    FdLog::set_cpu("demo-cpu");
    fd_log_info!("  new_cpu_name: {}", FdLog::cpu());
    println!();

    FdLog::set_level_stderr(LogLevel::Warning);
    fd_log_info!("  new_stderr_level: {:?}", FdLog::level_stderr());
    println!();

    let current_colorize = FdLog::colorize();
    FdLog::set_colorize(!current_colorize);
    fd_log_info!(
        "  set_colorize: {} -> {}",
        current_colorize,
        FdLog::colorize()
    );
    println!();

    fd_log_debug!("debug message from crabland! format: {}", 42069);
    fd_log_notice!("notice from crabland! format: {}", 42069);
    fd_log_warning!("warning from crabland! format: {}", 42069);

    fd_log_info!("check stderr");
}
