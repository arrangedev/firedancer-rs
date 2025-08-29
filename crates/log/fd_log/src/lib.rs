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

/// Generic logging function that mimics the behavior of the `FD_LOG_*` C macros.
///
/// Like `FD_LOG_*`, this will do the following:
/// 1. Get the current timestamp
/// 2. Format the message
/// 3. Call the appropriate `fd_log_private_*` function
pub fn fd_log_impl(level: LogLevel, file: &str, line: u32, func: &str, message: &str) {
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
        $crate::fd_log_impl(
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
        $crate::fd_log_impl(
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
        $crate::fd_log_impl(
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
        $crate::fd_log_impl(
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
        $crate::fd_log_impl(
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
        $crate::fd_log_impl(
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
        $crate::fd_log_impl(
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
        $crate::fd_log_impl(
            $crate::LogLevel::Emergency,
            file!(),
            line!(),
            module_path!(),
            &format!($($arg)*),
        )
    };
}

/// Alias for `fd_log_error` (FD_LOG_ERR)
#[macro_export]
macro_rules! fd_log_err {
    ($($arg:tt)*) => {
        fd_log_error!($($arg)*)
    };
}

/// Alias for `fd_log_emergency` (FD_LOG_EMERG)
#[macro_export]
macro_rules! fd_log_emerg {
    ($($arg:tt)*) => {
        fd_log_emergency!($($arg)*)
    };
}

/// Alias for `fd_log_critical` (FD_LOG_CRIT)
#[macro_export]
macro_rules! fd_log_crit {
    ($($arg:tt)*) => {
        fd_log_critical!($($arg)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_levels() {
        assert_eq!(LogLevel::Debug as i32, 0);
        assert_eq!(LogLevel::Info as i32, 1);
        assert_eq!(LogLevel::Emergency as i32, 7);
    }

    #[test]
    fn test_log_functions() {
        let _app_id = FdLog::app_id();
        let _thread_id = FdLog::thread_id();
        let _host_id = FdLog::host_id();
        let _cpu_id = FdLog::cpu_id();
        let _group_id = FdLog::group_id();
        let _tid = FdLog::tid();
        let _user_id = FdLog::user_id();

        let _wallclock = FdLog::wallclock_host();
        let _colorize = FdLog::colorize();
    }

    #[test]
    fn test_log_impl() {
        fd_log_impl(
            LogLevel::Info,
            "test.rs",
            42,
            "test_function",
            "This is a test message",
        );
    }

    #[test]
    fn test_macros() {
        // no critical/alert/emergency since they'll nuke the process
        fd_log_debug!("This is a debug message");
        fd_log_info!("This is an info message with value: {}", 42);
        fd_log_notice!("This is a notice");
        fd_log_warning!("This is a warning");
        fd_log_err!("This is an error message");
    }

    #[test]
    fn test_fd_danger_levels() {
        fd_log_crit!("This is a critical message");
        fd_log_alert!("This is an alert message");
        fd_log_emerg!("This is an emergency message");
    }
}
