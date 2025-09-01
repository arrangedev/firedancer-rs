//! Safe Rust bindings for Firedancer log utility
//!
//! This crate provides a safe, idiomatic Rust API for the Firedancer logging system.
//! It wraps the unsafe FFI bindings provided by `libfdlog-sys`.

use std::ffi::{CStr, CString};
use std::os::raw::c_int;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Notice = 2,
    Warning = 3,
    Error = 4,
    Critical = 5,
    Alert = 6,
    Emergency = 7,
}

pub struct FdLog;

impl FdLog {
    pub fn app_id() -> u64 {
        unsafe { libfd_log_sys::fd_log_app_id() }
    }

    pub fn app() -> &'static str {
        unsafe {
            let ptr = libfd_log_sys::fd_log_app();
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }

    pub fn thread_id() -> u64 {
        unsafe { libfd_log_sys::fd_log_thread_id() }
    }

    pub fn thread() -> &'static str {
        unsafe {
            let ptr = libfd_log_sys::fd_log_thread();
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }

    pub fn set_thread(name: &str) {
        let c_name = std::ffi::CString::new(name).unwrap();
        unsafe {
            libfd_log_sys::fd_log_thread_set(c_name.as_ptr());
        }
    }

    pub fn host_id() -> u64 {
        unsafe { libfd_log_sys::fd_log_host_id() }
    }

    pub fn host() -> &'static str {
        unsafe {
            let ptr = libfd_log_sys::fd_log_host();
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }

    pub fn cpu_id() -> u64 {
        unsafe { libfd_log_sys::fd_log_cpu_id() }
    }

    pub fn cpu() -> &'static str {
        unsafe {
            let ptr = libfd_log_sys::fd_log_cpu();
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }

    pub fn set_cpu(name: &str) {
        let c_name = std::ffi::CString::new(name).unwrap();
        unsafe {
            libfd_log_sys::fd_log_cpu_set(c_name.as_ptr());
        }
    }

    pub fn group_id() -> u64 {
        unsafe { libfd_log_sys::fd_log_group_id() }
    }

    pub fn group() -> &'static str {
        unsafe {
            let ptr = libfd_log_sys::fd_log_group();
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }

    pub fn tid() -> u64 {
        unsafe { libfd_log_sys::fd_log_tid() }
    }

    pub fn user_id() -> u64 {
        unsafe { libfd_log_sys::fd_log_user_id() }
    }

    pub fn user() -> &'static str {
        unsafe {
            let ptr = libfd_log_sys::fd_log_user();
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }

    /// Get the current wallclock time in nanos since unix epoch
    pub fn wallclock() -> i64 {
        unsafe { libfd_log_sys::fd_log_wallclock() }
    }

    /// Get the host wallclock time in nanos since unix epoch
    pub fn wallclock_host() -> i64 {
        unsafe { libfd_log_sys::fd_log_wallclock_host(std::ptr::null()) }
    }

    /// Sleep for a given duration in nanos
    pub fn sleep(dt: i64) -> i64 {
        unsafe { libfd_log_sys::fd_log_sleep(dt) }
    }

    pub fn wait_until(then: i64) -> i64 {
        unsafe { libfd_log_sys::fd_log_wait_until(then) }
    }

    /// Manually flush the log buffer
    pub fn flush() {
        unsafe { libfd_log_sys::fd_log_flush() }
    }

    pub fn colorize() -> bool {
        unsafe { libfd_log_sys::fd_log_colorize() != 0 }
    }

    pub fn set_colorize(enabled: bool) {
        unsafe { libfd_log_sys::fd_log_colorize_set(if enabled { 1 } else { 0 }) }
    }

    pub fn level_logfile() -> LogLevel {
        let level = unsafe { libfd_log_sys::fd_log_level_logfile() };
        LogLevel::from_int(level)
    }

    pub fn set_level_logfile(level: LogLevel) {
        unsafe { libfd_log_sys::fd_log_level_logfile_set(level as c_int) }
    }

    pub fn level_stderr() -> LogLevel {
        let level = unsafe { libfd_log_sys::fd_log_level_stderr() };
        LogLevel::from_int(level)
    }

    pub fn set_level_stderr(level: LogLevel) {
        unsafe { libfd_log_sys::fd_log_level_stderr_set(level as c_int) }
    }

    pub fn level_flush() -> LogLevel {
        let level = unsafe { libfd_log_sys::fd_log_level_flush() };
        LogLevel::from_int(level)
    }

    pub fn set_level_flush(level: LogLevel) {
        unsafe { libfd_log_sys::fd_log_level_flush_set(level as c_int) }
    }

    pub fn level_core() -> LogLevel {
        let level = unsafe { libfd_log_sys::fd_log_level_core() };
        LogLevel::from_int(level)
    }

    pub fn set_level_core(level: LogLevel) {
        unsafe { libfd_log_sys::fd_log_level_core_set(level as c_int) }
    }

    pub fn enable_unclean_exit() {
        unsafe { libfd_log_sys::fd_log_enable_unclean_exit() }
    }
}

// FdLog::set_cpu("demo-cpu");
#[macro_export]
macro_rules! cpu {
    ($($arg:tt)*) => {
        $crate::FdLog::set_cpu(&format!($($arg)*))
    };
}

// FdLog::set_thread("demo-thread");
#[macro_export]
macro_rules! thread {
    ($($arg:tt)*) => {
        $crate::FdLog::set_thread(&format!($($arg)*))
    };
}

impl LogLevel {
    fn from_int(level: c_int) -> Self {
        match level {
            0 => LogLevel::Debug,
            1 => LogLevel::Info,
            2 => LogLevel::Notice,
            3 => LogLevel::Warning,
            4 => LogLevel::Error,
            5 => LogLevel::Critical,
            6 => LogLevel::Alert,
            7 => LogLevel::Emergency,
            _ => LogLevel::Info,
        }
    }
}

#[macro_export]
macro_rules! location_level {
    ($logfile:ident, $level:expr) => {
        $crate::FdLog::set_level_logfile($level);
    };
    ($stderr:ident, $level:expr) => {
        $crate::FdLog::set_level_stderr($level);
    };
    ($flush:ident, $level:expr) => {
        $crate::FdLog::set_level_flush($level);
    };
    ($core:ident, $level:expr) => {
        $crate::FdLog::set_level_core($level);
    };
}

/// Generic logging function that mimics the behavior of the `FD_LOG_*` C macros.
///
/// Like `FD_LOG_*`, this will do the following:
/// 1. Get the current timestamp
/// 2. Format the message
/// 3. Call the appropriate `fd_log_private_*` function
pub fn _fd_log(level: LogLevel, file: &str, line: u32, func: &str, message: &str) {
    let now = unsafe { libfd_log_sys::fd_log_wallclock() };
    let c_file = CString::new(file).unwrap_or_else(|_| CString::new("unknown").unwrap());
    let c_func = CString::new(func).unwrap_or_else(|_| CString::new("unknown").unwrap());
    let c_message = CString::new(message).unwrap_or_else(|_| CString::new("invalid_utf8").unwrap());

    // levels 0-3 use `fd_log_private_1` (non-fatal)
    // levels 4+ use `fd_log_private_2` (potentially fatal)
    unsafe {
        match level {
            LogLevel::Debug | LogLevel::Info | LogLevel::Notice | LogLevel::Warning => {
                libfd_log_sys::fd_log_private_1(
                    level as c_int,
                    now,
                    c_file.as_ptr(),
                    line as c_int,
                    c_func.as_ptr(),
                    c_message.as_ptr(),
                );
            }
            LogLevel::Error | LogLevel::Critical | LogLevel::Alert | LogLevel::Emergency => {
                libfd_log_sys::fd_log_private_2(
                    level as c_int,
                    now,
                    c_file.as_ptr(),
                    line as c_int,
                    c_func.as_ptr(),
                    c_message.as_ptr(),
                );
            }
        }
    }
}

#[macro_export]
macro_rules! fd_log_debug {
    ($($arg:tt)*) => {
        $crate::_fd_log(
            $crate::LogLevel::Debug,
            file!(),
            line!(),
            module_path!(),
            &format!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! fd_log_info {
    ($($arg:tt)*) => {
        $crate::_fd_log(
            $crate::LogLevel::Info,
            file!(),
            line!(),
            module_path!(),
            &format!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! fd_log_notice {
    ($($arg:tt)*) => {
        $crate::_fd_log(
            $crate::LogLevel::Notice,
            file!(),
            line!(),
            module_path!(),
            &format!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! fd_log_warning {
    ($($arg:tt)*) => {
        $crate::_fd_log(
            $crate::LogLevel::Warning,
            file!(),
            line!(),
            module_path!(),
            &format!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! fd_log_error {
    ($($arg:tt)*) => {
        $crate::_fd_log(
            $crate::LogLevel::Error,
            file!(),
            line!(),
            module_path!(),
            &format!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! fd_log_critical {
    ($($arg:tt)*) => {
        $crate::_fd_log(
            $crate::LogLevel::Critical,
            file!(),
            line!(),
            module_path!(),
            &format!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! fd_log_alert {
    ($($arg:tt)*) => {
        $crate::_fd_log(
            $crate::LogLevel::Alert,
            file!(),
            line!(),
            module_path!(),
            &format!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! fd_log_emergency {
    ($($arg:tt)*) => {
        $crate::_fd_log(
            $crate::LogLevel::Emergency,
            file!(),
            line!(),
            module_path!(),
            &format!($($arg)*),
        )
    };
}

/// Alias for `fd_log_debug` (FD_LOG_DEBUG)
#[macro_export]
macro_rules! fd_dbg {
    ($($arg:tt)*) => {
        $crate::fd_log_debug!($($arg)*)
    };
}

/// Alias for `fd_log_info` (FD_LOG_INFO)
#[macro_export]
macro_rules! fd_info {
    ($($arg:tt)*) => {
        $crate::fd_log_info!($($arg)*)
    };
}

/// Alias for `fd_log_notice` (FD_LOG_NOTICE)
#[macro_export]
macro_rules! fd_notice {
    ($($arg:tt)*) => {
        $crate::fd_log_notice!($($arg)*)
    };
}

/// Alias for `fd_log_warning` (FD_LOG_WARNING)
#[macro_export]
macro_rules! fd_warn {
    ($($arg:tt)*) => {
        $crate::fd_log_warning!($($arg)*)
    };
}

/// Alias for `fd_log_error` (FD_LOG_ERR)
///
/// This will exit the program with a SIGABRT signal
#[macro_export]
macro_rules! fd_err {
    ($($arg:tt)*) => {
        $crate::fd_log_error!($($arg)*)
    };
}

/// Alias for `fd_log_emergency` (FD_LOG_EMERG)
///
/// This will abort the program with a SIGABRT signal
#[macro_export]
macro_rules! fd_emerg {
    ($($arg:tt)*) => {
        $crate::fd_log_emergency!($($arg)*)
    };
}

/// Alias for `fd_log_alert` (FD_LOG_ALERT)
///
/// This will abort the program with a SIGABRT signal
#[macro_export]
macro_rules! fd_alert {
    ($($arg:tt)*) => {
        $crate::fd_log_alert!($($arg)*)
    };
}

/// Alias for `fd_log_critical` (FD_LOG_CRIT)
///
/// This will abort the program with a SIGABRT signal
#[macro_export]
macro_rules! fd_crit {
    ($($arg:tt)*) => {
        $crate::fd_log_critical!($($arg)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    fn abort_handler() {
        panic!("Abort handler called.");
    }

    fn sighandler() {
        unsafe {
            libc::signal(libc::SIGABRT, abort_handler as usize);
        }
    }

    #[test]
    fn test_ids() {
        let _app_id = FdLog::app_id();
        let _thread_id = FdLog::thread_id();
        let _host_id = FdLog::host_id();
        let _cpu_id = FdLog::cpu_id();
        let _group_id = FdLog::group_id();
        let _tid = FdLog::tid();
        let _user_id = FdLog::user_id();

        assert_eq!(_app_id, 0);
        assert_eq!(_thread_id, 0);
        assert_eq!(_host_id, 0);
        assert_eq!(_group_id, 0);

        assert_ne!(_cpu_id, 0);
        assert_ne!(_tid, 0);
        assert_ne!(_user_id, 0);

        let _wallclock = FdLog::wallclock_host();
        assert_ne!(_wallclock, 0);
    }

    /// no `fd_log_private_2` level usage here since they'll nuke the process
    #[test_case(true; "test_recoverable_colorized")]
    fn test_recoverable(colorize: bool) {
        FdLog::set_colorize(colorize);

        fd_dbg!("Debug message; low prio");
        fd_info!("Info message; value={}", 42);
        fd_notice!("Notice message; medium priority");
        fd_warn!("Warning message; medium-high priority");
    }

    #[test_case(LogLevel::Error, "ERROR! SOMETHING HAPPENED"; "test_error")]
    #[test_case(LogLevel::Critical, "CRITICAL! SOMETHING IS SERIOUSLY WRONG"; "test_critical")]
    #[test_case(LogLevel::Alert, "RED ALERT! SOMETHING IS CRITICALLY WRONG"; "test_alert")]
    #[test_case(LogLevel::Emergency, "EMERGENCY! SOMETHING IS CRITICALLY WRONG"; "test_emergency")]
    #[should_panic]
    fn test_unrecoverable(level: LogLevel, message: &str) {
        sighandler();

        match level {
            LogLevel::Emergency => fd_emerg!("{}", message),
            LogLevel::Error => fd_err!("{}", message),
            LogLevel::Alert => fd_alert!("{}", message),
            LogLevel::Critical => fd_crit!("{}", message),
            _ => (),
        }
    }
}
