//! Safe Rust bindings for Firedancer log utility
//!
//! This crate provides a safe, idiomatic Rust API for the Firedancer logging system.
//! It wraps the unsafe FFI bindings provided by `libfdlog-sys`.

use std::ffi::{CStr, CString};
use std::os::raw::c_int;
use std::sync::{Mutex, Once};

// Global state for logger initialization
static LOGGER_INIT: Once = Once::new();
static LOGGER_INIT_RESULT: Mutex<Option<Result<(), LogError>>> = Mutex::new(None);

/// Error type for logging operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogError {
    InvalidPath(String),
    InvalidFd(i32),
    NulError(String),
    InitializationFailed(String),
}

impl std::fmt::Display for LogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogError::InvalidPath(path) => write!(f, "Invalid log path: {}", path),
            LogError::InvalidFd(fd) => write!(f, "Invalid file descriptor: {}", fd),
            LogError::NulError(msg) => write!(f, "String contains null byte: {}", msg),
            LogError::InitializationFailed(msg) => write!(f, "Log initialization failed: {}", msg),
        }
    }
}

impl std::error::Error for LogError {}

/// Configuration for custom log initialization
#[derive(Debug, Clone)]
pub struct LogConfig {
    pub app_id: Option<u64>,
    pub app: Option<String>,
    pub thread_id: Option<u64>,
    pub thread: Option<String>,
    pub host_id: Option<u64>,
    pub host: Option<String>,
    pub cpu_id: Option<u64>,
    pub cpu: Option<String>,
    pub group_id: Option<u64>,
    pub group: Option<String>,
    pub tid: Option<u64>,
    pub user_id: Option<u64>,
    pub user: Option<String>,
    pub dedup: bool,
    pub colorize: bool,
    pub level_logfile: LogLevel,
    pub level_stderr: LogLevel,
    pub level_flush: LogLevel,
    pub level_core: LogLevel,
    pub log_fd: Option<i32>,
    pub log_path: Option<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            app_id: None,
            app: None,
            thread_id: None,
            thread: None,
            host_id: None,
            host: None,
            cpu_id: None,
            cpu: None,
            group_id: None,
            group: None,
            tid: None,
            user_id: None,
            user: None,
            dedup: true,
            colorize: false,
            level_logfile: LogLevel::Info,
            level_stderr: LogLevel::Notice,
            level_flush: LogLevel::Warning,
            level_core: LogLevel::Critical,
            log_fd: None,
            log_path: None,
        }
    }
}

/// A builder for configuring the Firedancer logger
///
/// This provides a fluent API similar to `tracing_subscriber::EnvSubscriber::builder()`
/// for configuring logging parameters. Settings are applied immediately, so you can
/// start using the global logging macros right away.
///
/// # Examples
///
/// ```rust
/// use fd_log::{FdLogBuilder, LogLevel, info, debug};
///
/// // Basic configuration - logging works immediately!
/// FdLogBuilder::new()
///     .with_logfile_level(LogLevel::Debug)
///     .with_colorize(true);
///
/// info!("This works right away!");
/// debug!("Debug messages are now enabled");
///
/// // For file logging, you need to call init()
/// FdLogBuilder::new()
///     .with_logfile_level(LogLevel::Debug)
///     .with_colorize(true)
///     .with_file("/tmp/my_app.log")
///     .init()
///     .expect("Failed to initialize file logging");
/// ```
#[derive(Debug, Clone)]
pub struct FdLogBuilder {
    config: LogConfig,
}

impl Default for FdLogBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FdLogBuilder {
    /// Create a new logger builder with default configuration
    pub fn new() -> Self {
        Self {
            config: LogConfig::default(),
        }
    }

    pub fn with_logfile_level(mut self, level: LogLevel) -> Self {
        self.config.level_logfile = level;
        FdLog::set_level_logfile(level);
        self
    }

    pub fn with_stderr_level(mut self, level: LogLevel) -> Self {
        self.config.level_stderr = level;
        FdLog::set_level_stderr(level);
        self
    }

    pub fn with_flush_level(mut self, level: LogLevel) -> Self {
        self.config.level_flush = level;
        FdLog::set_level_flush(level);
        self
    }

    pub fn with_core_level(mut self, level: LogLevel) -> Self {
        self.config.level_core = level;
        FdLog::set_level_core(level);
        self
    }

    pub fn with_colorize(mut self, colorize: bool) -> Self {
        self.config.colorize = colorize;
        FdLog::set_colorize(colorize);
        self
    }

    /// Set the log file path
    ///
    /// Use "-" for stdout, "" to disable file logging
    pub fn with_file<P: AsRef<str>>(mut self, path: P) -> Self {
        self.config.log_path = Some(path.as_ref().to_string());
        self
    }

    /// Set a custom file descriptor for logging
    pub fn with_fd(mut self, fd: i32) -> Self {
        self.config.log_fd = Some(fd);
        self
    }

    /// Enable or disable log deduplication
    pub fn with_dedup(mut self, dedup: bool) -> Self {
        self.config.dedup = dedup;
        self
    }

    /// Set the application name
    pub fn with_app<S: AsRef<str>>(mut self, app: S) -> Self {
        self.config.app = Some(app.as_ref().to_string());
        self
    }

    /// Set the thread name
    pub fn with_thread<S: AsRef<str>>(mut self, thread: S) -> Self {
        let thread_str = thread.as_ref().to_string();
        self.config.thread = Some(thread_str.clone());
        FdLog::set_thread(&thread_str);
        self
    }

    /// Set the CPU name
    pub fn with_cpu<S: AsRef<str>>(mut self, cpu: S) -> Self {
        let cpu_str = cpu.as_ref().to_string();
        self.config.cpu = Some(cpu_str.clone());
        FdLog::set_cpu(&cpu_str);
        self
    }

    /// Set custom IDs
    pub fn with_app_id(mut self, id: u64) -> Self {
        self.config.app_id = Some(id);
        self
    }

    pub fn with_thread_id(mut self, id: u64) -> Self {
        self.config.thread_id = Some(id);
        self
    }

    pub fn with_host_id(mut self, id: u64) -> Self {
        self.config.host_id = Some(id);
        self
    }

    pub fn with_cpu_id(mut self, id: u64) -> Self {
        self.config.cpu_id = Some(id);
        self
    }

    pub fn with_group_id(mut self, id: u64) -> Self {
        self.config.group_id = Some(id);
        self
    }

    pub fn with_tid(mut self, id: u64) -> Self {
        self.config.tid = Some(id);
        self
    }

    pub fn with_user_id(mut self, id: u64) -> Self {
        self.config.user_id = Some(id);
        self
    }

    /// Initialize the global logger with the configured settings
    ///
    /// This method is only needed if you want to set up custom file logging.
    /// Basic logging (to stderr) works immediately after using the builder methods.
    /// This method sets up file logging and other advanced features.
    pub fn init(self) -> Result<(), LogError> {
        // Only initialize file logging if a path or fd was specified
        if self.config.log_path.is_some() || self.config.log_fd.is_some() {
            LOGGER_INIT.call_once(|| {
                let result = FdLog::boot_custom(self.config);
                *LOGGER_INIT_RESULT.lock().unwrap() = Some(result);
            });

            LOGGER_INIT_RESULT.lock().unwrap().as_ref().unwrap().clone()
        } else {
            // No file logging requested, just return success
            Ok(())
        }
    }

    /// Try to initialize the global logger, ignoring errors if already initialized
    ///
    /// This is useful for libraries that want to set up logging but don't want
    /// to fail if the application has already initialized logging.
    pub fn try_init(self) -> Result<(), LogError> {
        match self.init() {
            Ok(()) => Ok(()),
            Err(LogError::InitializationFailed(msg)) if msg.contains("already initialized") => {
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

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
        unsafe { fd_log_sys::fd_log_app_id() }
    }

    pub fn app() -> &'static str {
        unsafe {
            let ptr = fd_log_sys::fd_log_app();
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }

    pub fn thread_id() -> u64 {
        unsafe { fd_log_sys::fd_log_thread_id() }
    }

    pub fn thread() -> &'static str {
        unsafe {
            let ptr = fd_log_sys::fd_log_thread();
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }

    pub fn set_thread(name: &str) {
        let c_name = std::ffi::CString::new(name).unwrap();
        unsafe {
            fd_log_sys::fd_log_thread_set(c_name.as_ptr());
        }
    }

    pub fn host_id() -> u64 {
        unsafe { fd_log_sys::fd_log_host_id() }
    }

    pub fn host() -> &'static str {
        unsafe {
            let ptr = fd_log_sys::fd_log_host();
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }

    pub fn cpu_id() -> u64 {
        unsafe { fd_log_sys::fd_log_cpu_id() }
    }

    pub fn cpu() -> &'static str {
        unsafe {
            let ptr = fd_log_sys::fd_log_cpu();
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }

    pub fn set_cpu(name: &str) {
        let c_name = std::ffi::CString::new(name).unwrap();
        unsafe {
            fd_log_sys::fd_log_cpu_set(c_name.as_ptr());
        }
    }

    pub fn group_id() -> u64 {
        unsafe { fd_log_sys::fd_log_group_id() }
    }

    pub fn group() -> &'static str {
        unsafe {
            let ptr = fd_log_sys::fd_log_group();
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }

    pub fn tid() -> u64 {
        unsafe { fd_log_sys::fd_log_tid() }
    }

    pub fn user_id() -> u64 {
        unsafe { fd_log_sys::fd_log_user_id() }
    }

    pub fn user() -> &'static str {
        unsafe {
            let ptr = fd_log_sys::fd_log_user();
            CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }

    /// Get the current wallclock time in nanos since unix epoch
    pub fn wallclock() -> i64 {
        unsafe { fd_log_sys::fd_log_wallclock() }
    }

    /// Get the host wallclock time in nanos since unix epoch
    pub fn wallclock_host() -> i64 {
        unsafe { fd_log_sys::fd_log_wallclock_host(std::ptr::null()) }
    }

    /// Sleep for a given duration in nanos
    pub fn sleep(dt: i64) -> i64 {
        unsafe { fd_log_sys::fd_log_sleep(dt) }
    }

    pub fn wait_until(then: i64) -> i64 {
        unsafe { fd_log_sys::fd_log_wait_until(then) }
    }

    /// Manually flush the log buffer
    pub fn flush() {
        unsafe { fd_log_sys::fd_log_flush() }
    }

    pub fn colorize() -> bool {
        unsafe { fd_log_sys::fd_log_colorize() != 0 }
    }

    pub fn set_colorize(enabled: bool) {
        unsafe { fd_log_sys::fd_log_colorize_set(if enabled { 1 } else { 0 }) }
    }

    pub fn level_logfile() -> LogLevel {
        let level = unsafe { fd_log_sys::fd_log_level_logfile() };
        LogLevel::from_int(level)
    }

    pub fn set_level_logfile(level: LogLevel) {
        unsafe { fd_log_sys::fd_log_level_logfile_set(level as c_int) }
    }

    pub fn level_stderr() -> LogLevel {
        let level = unsafe { fd_log_sys::fd_log_level_stderr() };
        LogLevel::from_int(level)
    }

    pub fn set_level_stderr(level: LogLevel) {
        unsafe { fd_log_sys::fd_log_level_stderr_set(level as c_int) }
    }

    pub fn level_flush() -> LogLevel {
        let level = unsafe { fd_log_sys::fd_log_level_flush() };
        LogLevel::from_int(level)
    }

    pub fn set_level_flush(level: LogLevel) {
        unsafe { fd_log_sys::fd_log_level_flush_set(level as c_int) }
    }

    pub fn level_core() -> LogLevel {
        let level = unsafe { fd_log_sys::fd_log_level_core() };
        LogLevel::from_int(level)
    }

    pub fn set_level_core(level: LogLevel) {
        unsafe { fd_log_sys::fd_log_level_core_set(level as c_int) }
    }

    pub fn enable_unclean_exit() {
        unsafe { fd_log_sys::fd_log_enable_unclean_exit() }
    }

    /// Initialize logging with a custom log file path
    pub fn boot_with_logfile(log_path: &str) -> Result<(), LogError> {
        let mut config = LogConfig::default();
        config.log_path = Some(log_path.to_string());
        Self::boot_custom(config)
    }

    /// Initialize logging with a custom file descriptor
    pub fn boot_with_fd(log_fd: i32) -> Result<(), LogError> {
        if log_fd < 0 {
            return Err(LogError::InvalidFd(log_fd));
        }
        let mut config = LogConfig::default();
        config.log_fd = Some(log_fd);
        Self::boot_custom(config)
    }

    /// Initialize logging with custom configuration
    pub fn boot_custom(config: LogConfig) -> Result<(), LogError> {
        // Convert strings to C strings
        let app_cstr = match &config.app {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError(s.clone()))?),
            None => None,
        };
        let thread_cstr = match &config.thread {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError(s.clone()))?),
            None => None,
        };
        let host_cstr = match &config.host {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError(s.clone()))?),
            None => None,
        };
        let cpu_cstr = match &config.cpu {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError(s.clone()))?),
            None => None,
        };
        let group_cstr = match &config.group {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError(s.clone()))?),
            None => None,
        };
        let user_cstr = match &config.user {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError(s.clone()))?),
            None => None,
        };
        let log_path_cstr = match &config.log_path {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError(s.clone()))?),
            None => None,
        };

        unsafe {
            fd_log_sys::fd_log_private_boot_custom(
                std::ptr::null_mut(), // lock - using null for default
                config.app_id.unwrap_or(0),
                app_cstr.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                config.thread_id.unwrap_or(0),
                thread_cstr
                    .as_ref()
                    .map_or(std::ptr::null(), |s| s.as_ptr()),
                config.host_id.unwrap_or(0),
                host_cstr.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                config.cpu_id.unwrap_or(0),
                cpu_cstr.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                config.group_id.unwrap_or(0),
                group_cstr.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                config.tid.unwrap_or(0),
                config.user_id.unwrap_or(0),
                user_cstr.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                if config.dedup { 1 } else { 0 },
                if config.colorize { 1 } else { 0 },
                config.level_logfile as c_int,
                config.level_stderr as c_int,
                config.level_flush as c_int,
                config.level_core as c_int,
                config.log_fd.unwrap_or(-1),
                log_path_cstr
                    .as_ref()
                    .map_or(std::ptr::null(), |s| s.as_ptr()),
            );
        }
        Ok(())
    }

    /// Get the current log file descriptor (if any)
    pub fn logfile_fd() -> Option<i32> {
        let fd = unsafe { fd_log_sys::fd_log_private_logfile_fd() };
        if fd == -1 {
            None
        } else {
            Some(fd)
        }
    }

    /// Log a hex dump at the specified level
    pub fn hexdump(level: LogLevel, tag: &str, data: &[u8]) {
        let c_tag = CString::new(tag).unwrap_or_else(|_| CString::new("invalid_tag").unwrap());
        let now = unsafe { fd_log_sys::fd_log_wallclock() };

        let hex_msg = unsafe {
            fd_log_sys::fd_log_private_hexdump_msg(
                c_tag.as_ptr(),
                data.as_ptr() as *const std::ffi::c_void,
                data.len() as u64,
            )
        };

        // Use the appropriate logging function based on level
        unsafe {
            match level {
                LogLevel::Debug | LogLevel::Info | LogLevel::Notice | LogLevel::Warning => {
                    fd_log_sys::fd_log_private_1(
                        level as c_int,
                        now,
                        b"hexdump\0".as_ptr() as *const i8,
                        0,
                        b"FdLog::hexdump\0".as_ptr() as *const i8,
                        hex_msg,
                    );
                }
                LogLevel::Error | LogLevel::Critical | LogLevel::Alert | LogLevel::Emergency => {
                    fd_log_sys::fd_log_private_2(
                        level as c_int,
                        now,
                        b"hexdump\0".as_ptr() as *const i8,
                        0,
                        b"FdLog::hexdump\0".as_ptr() as *const i8,
                        hex_msg,
                    );
                }
            }
        }
    }

    /// Pretty print wallclock time
    pub fn wallclock_cstr(timestamp: i64) -> String {
        let mut buf = [0u8; 37]; // FD_LOG_WALLCLOCK_CSTR_BUF_SZ
        unsafe {
            fd_log_sys::fd_log_wallclock_cstr(timestamp, buf.as_mut_ptr() as *mut i8);
            CStr::from_ptr(buf.as_ptr() as *const i8)
                .to_string_lossy()
                .into_owned()
        }
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
    let now = unsafe { fd_log_sys::fd_log_wallclock() };
    let c_file = CString::new(file).unwrap_or_else(|_| CString::new("unknown").unwrap());
    let c_func = CString::new(func).unwrap_or_else(|_| CString::new("unknown").unwrap());
    let c_message = CString::new(message).unwrap_or_else(|_| CString::new("invalid_utf8").unwrap());

    // levels 0-3 use `fd_log_private_1` (non-fatal)
    // levels 4+ use `fd_log_private_2` (potentially fatal)
    unsafe {
        match level {
            LogLevel::Debug | LogLevel::Info | LogLevel::Notice | LogLevel::Warning => {
                fd_log_sys::fd_log_private_1(
                    level as c_int,
                    now,
                    c_file.as_ptr(),
                    line as c_int,
                    c_func.as_ptr(),
                    c_message.as_ptr(),
                );
            }
            LogLevel::Error | LogLevel::Critical | LogLevel::Alert | LogLevel::Emergency => {
                fd_log_sys::fd_log_private_2(
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

// Hexdump macros

/// Log a hex dump at DEBUG level
#[macro_export]
macro_rules! debug_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::FdLog::hexdump($crate::LogLevel::Debug, $tag, $data)
    };
}

/// Log a hex dump at INFO level
#[macro_export]
macro_rules! info_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::FdLog::hexdump($crate::LogLevel::Info, $tag, $data)
    };
}

/// Log a hex dump at NOTICE level
#[macro_export]
macro_rules! notice_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::FdLog::hexdump($crate::LogLevel::Notice, $tag, $data)
    };
}

/// Log a hex dump at WARNING level
#[macro_export]
macro_rules! warn_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::FdLog::hexdump($crate::LogLevel::Warning, $tag, $data)
    };
}

/// Log a hex dump at ERROR level
/// This will exit the program with a SIGABRT signal
#[macro_export]
macro_rules! err_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::FdLog::hexdump($crate::LogLevel::Error, $tag, $data)
    };
}

/// Log a hex dump at CRITICAL level
/// This will abort the program with a SIGABRT signal
#[macro_export]
macro_rules! crit_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::FdLog::hexdump($crate::LogLevel::Critical, $tag, $data)
    };
}

/// Log a hex dump at ALERT level
/// This will abort the program with a SIGABRT signal
#[macro_export]
macro_rules! alert_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::FdLog::hexdump($crate::LogLevel::Alert, $tag, $data)
    };
}

/// Log a hex dump at EMERGENCY level
/// This will abort the program with a SIGABRT signal
#[macro_export]
macro_rules! emergency_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::FdLog::hexdump($crate::LogLevel::Emergency, $tag, $data)
    };
}

// Shorter aliases for hexdump macros

/// Alias for `debug_hexdump`
#[macro_export]
macro_rules! hxd_dbg {
    ($tag:expr, $data:expr) => {
        $crate::debug_hexdump!($tag, $data)
    };
}

/// Alias for `warn_hexdump`
#[macro_export]
macro_rules! hxd_warn {
    ($tag:expr, $data:expr) => {
        $crate::warn_hexdump!($tag, $data)
    };
}

/// Alias for `err_hexdump`
#[macro_export]
macro_rules! hxd_err {
    ($tag:expr, $data:expr) => {
        $crate::err_hexdump!($tag, $data)
    };
}

/// Alias for `crit_hexdump`
#[macro_export]
macro_rules! hxd_crit {
    ($tag:expr, $data:expr) => {
        $crate::crit_hexdump!($tag, $data)
    };
}

/// Alias for `emergency_hexdump`
#[macro_export]
macro_rules! hxd_emerg {
    ($tag:expr, $data:expr) => {
        $crate::emergency_hexdump!($tag, $data)
    };
}

// Global logging macros - these are the main macros users should use
// They provide a clean, simple interface similar to other Rust logging crates

/// Log a message at DEBUG level
///
/// This is the global logging macro that should be used after initializing
/// the logger with `FdLogBuilder::new().init()`.
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::fd_dbg!($($arg)*)
    };
}

/// Log a message at INFO level
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::fd_info!($($arg)*)
    };
}

/// Log a message at NOTICE level
#[macro_export]
macro_rules! notice {
    ($($arg:tt)*) => {
        $crate::fd_log_notice!($($arg)*)
    };
}

/// Log a message at WARNING level
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::fd_log_warning!($($arg)*)
    };
}

/// Log a message at ERROR level
/// This will exit the program with a SIGABRT signal
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::fd_log_error!($($arg)*)
    };
}

/// Log a message at CRITICAL level
/// This will abort the program with a SIGABRT signal
#[macro_export]
macro_rules! critical {
    ($($arg:tt)*) => {
        $crate::fd_log_critical!($($arg)*)
    };
}

/// Log a message at ALERT level
/// This will abort the program with a SIGABRT signal
#[macro_export]
macro_rules! alert {
    ($($arg:tt)*) => {
        $crate::fd_log_alert!($($arg)*)
    };
}

/// Log a message at EMERGENCY level
/// This will abort the program with a SIGABRT signal
#[macro_export]
macro_rules! emergency {
    ($($arg:tt)*) => {
        $crate::fd_log_emergency!($($arg)*)
    };
}

// Global hexdump macros

/// Log a hex dump at DEBUG level
#[macro_export]
macro_rules! debug_hex {
    ($tag:expr, $data:expr) => {
        $crate::debug_hexdump!($tag, $data)
    };
}

/// Log a hex dump at INFO level
#[macro_export]
macro_rules! info_hex {
    ($tag:expr, $data:expr) => {
        $crate::info_hexdump!($tag, $data)
    };
}

/// Log a hex dump at NOTICE level
#[macro_export]
macro_rules! notice_hex {
    ($tag:expr, $data:expr) => {
        $crate::notice_hexdump!($tag, $data)
    };
}

/// Log a hex dump at WARNING level
#[macro_export]
macro_rules! warn_hex {
    ($tag:expr, $data:expr) => {
        $crate::warn_hexdump!($tag, $data)
    };
}

/// Log a hex dump at ERROR level
/// This will exit the program with a SIGABRT signal
#[macro_export]
macro_rules! error_hex {
    ($tag:expr, $data:expr) => {
        $crate::err_hexdump!($tag, $data)
    };
}

/// Log a hex dump at CRITICAL level
/// This will abort the program with a SIGABRT signal
#[macro_export]
macro_rules! critical_hex {
    ($tag:expr, $data:expr) => {
        $crate::crit_hexdump!($tag, $data)
    };
}

/// Log a hex dump at ALERT level
/// This will abort the program with a SIGABRT signal
#[macro_export]
macro_rules! alert_hex {
    ($tag:expr, $data:expr) => {
        $crate::alert_hexdump!($tag, $data)
    };
}

/// Log a hex dump at EMERGENCY level
/// This will abort the program with a SIGABRT signal
#[macro_export]
macro_rules! emergency_hex {
    ($tag:expr, $data:expr) => {
        $crate::emergency_hexdump!($tag, $data)
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

    #[test]
    fn test_extended() {
        let _fd = FdLog::logfile_fd();

        let now = FdLog::wallclock();
        let _time_str = FdLog::wallclock_cstr(now);

        let test_data = b"Hello, World! This is test data for hexdump.";
        FdLog::hexdump(LogLevel::Info, "test_data", test_data);

        info_hexdump!("macro_test", test_data);
        debug_hexdump!(
            "debug_test",
            &[0x00, 0x01, 0x02, 0x03, 0xff, 0xfe, 0xfd, 0xfc]
        );
    }

    #[test]
    fn test_log_cfg() {
        let config = LogConfig {
            log_path: Some("/tmp/test.log".to_string()),
            colorize: true,
            level_logfile: LogLevel::Debug,
            ..Default::default()
        };

        assert_eq!(config.log_path, Some("/tmp/test.log".to_string()));
        assert_eq!(config.colorize, true);
        assert_eq!(config.level_logfile, LogLevel::Debug);
        assert_eq!(config.dedup, true);
    }

    #[test]
    fn test_builder_config() {
        let builder = FdLogBuilder::new()
            .with_stderr_level(LogLevel::Debug)
            .with_colorize(true)
            .with_file("/tmp/builder_test.log")
            .with_dedup(false)
            .with_app("test_app")
            .with_thread("test_thread");

        assert_eq!(builder.config.level_logfile, LogLevel::Debug);
        assert_eq!(builder.config.colorize, true);
        assert_eq!(
            builder.config.log_path,
            Some("/tmp/builder_test.log".to_string())
        );
        assert_eq!(builder.config.dedup, false);
        assert_eq!(builder.config.app, Some("test_app".to_string()));
        assert_eq!(builder.config.thread, Some("test_thread".to_string()));

        assert_eq!(builder.config.level_stderr, LogLevel::Notice);
        assert_eq!(builder.config.level_flush, LogLevel::Warning);
    }

    #[test]
    fn test_builder_api() {
        FdLogBuilder::new()
            .with_logfile_level(LogLevel::Debug)
            .with_colorize(true)
            .with_file("/tmp/builder_test.log")
            .with_dedup(false)
            .with_app("test_app")
            .with_thread("test_thread");

        debug!("Debug message via global macro");
        info!("Info message: value = {}", 42);
        notice!("Notice message");
        warn!("Warning message");

        let test_data = b"test";
        debug_hex!("test_tag", test_data);
        info_hex!("info_tag", test_data);
    }

    #[test]
    fn test_logfile_writing() {
        use std::fs;
        use std::path::Path;

        let log_path = "./fd_log_test_output.log";
        let _ = fs::remove_file(log_path);

        FdLogBuilder::new()
            .with_logfile_level(LogLevel::Info)
            .with_stderr_level(LogLevel::Debug)
            .with_colorize(true)
            .with_file(log_path)
            .with_app("test-logfile")
            .with_thread("test-thread")
            .with_cpu_id(69)
            .with_user_id(90210)
            .with_app_id(01)
            .with_group_id(01234567)
            .init()
            .unwrap();

        notice!("Notice message to logfile");
        info!("Test message to logfile");
        warn!("Warning message to logfile");

        FdLog::flush();

        if Path::new(log_path).exists() {
            match fs::read_to_string(log_path) {
                Ok(_) => {
                    info!("Log file created successfully!");
                }
                Err(e) => warn!("Could not read log file: {}", e),
            }
        } else {
            warn!("Log file was not created");
        }
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
