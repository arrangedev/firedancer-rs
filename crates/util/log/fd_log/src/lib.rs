//! Safe API for `fd_log_sys`

use std::ffi::{CStr, CString};
use std::os::raw::c_int;
use std::sync::{Mutex, Once};

static LOGGER_INIT: Once = Once::new();
static LOGGER_INIT_RESULT: Mutex<Option<Result<(), LogError>>> = Mutex::new(None);

pub struct SystemLogger;

impl SystemLogger {
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

    /// fd_log_wallclock reads the log's timesource to get the ns since the
    /// UNIX epoch GMT.  By default, this is fd_log_wallclock_host but the
    /// thread group can be configures this to use an alternative time source
    /// if desired.
    pub fn wallclock() -> i64 {
        unsafe { fd_log_sys::fd_log_wallclock() }
    }

    /// fd_log_wallclock_host reads the host's wallclock as ns since
    /// the UNIX epoch GMT.  On x86, this uses clock_gettime/CLOCK_REALTIME
    /// under the hood and is reasonably cheap (~25-50 ns nowadays).  But it
    /// still may involve system calls under the hood and is much slower
    /// than, say, RTSDC.
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

    pub fn boot_with_logfile(log_path: &str) -> Result<(), LogError> {
        let config = LogConfig {
            log_path: Some(log_path.to_string()),
            ..Default::default()
        };

        Self::boot_custom(config)
    }

    pub fn boot_with_fd(log_fd: i32) -> Result<(), LogError> {
        if log_fd < 0 {
            return Err(LogError::InvalidFd(log_fd));
        }

        let config = LogConfig {
            log_fd: Some(log_fd),
            ..Default::default()
        };

        Self::boot_custom(config)
    }

    pub fn boot_custom(config: LogConfig) -> Result<(), LogError> {
        let app_cstr = match &config.app {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError)?),
            None => None,
        };
        let thread_cstr = match &config.thread {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError)?),
            None => None,
        };
        let host_cstr = match &config.host {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError)?),
            None => None,
        };
        let cpu_cstr = match &config.cpu {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError)?),
            None => None,
        };
        let group_cstr = match &config.group {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError)?),
            None => None,
        };
        let user_cstr = match &config.user {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError)?),
            None => None,
        };
        let log_path_cstr = match &config.log_path {
            Some(s) => Some(CString::new(s.as_str()).map_err(|_| LogError::NulError)?),
            None => None,
        };

        unsafe {
            fd_log_sys::fd_log_private_boot_custom(
                std::ptr::null_mut(),
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

    /// This is exposed to allow the user to know the expected file descriptor
    /// for filtering and security, it should never be used to actually write
    /// logs and that should be done by the other functions in this crate.
    pub fn logfile_fd() -> Option<i32> {
        let fd = unsafe { fd_log_sys::fd_log_private_logfile_fd() };
        if fd == -1 {
            None
        } else {
            Some(fd)
        }
    }

    /// Log a message with a hexdump of any arbitrary data.
    ///
    /// Would log something like:
    /// ```
    /// WARNING 01-23 04:56:07.890123 75779 f0 0 src/file.c(901): HEXDUMP "bad_pkt" (96 bytes at 0x555555561a4e)
    ///         0000:  30 31 32 33 34 35 36 37 38 39 41 42 43 44 45 46  0123456789ABCDEF
    ///         0010:  47 48 49 4a 4b 4c 4d 4e 4f 50 51 52 53 54 55 56  GHIJKLMNOPQRSTUV
    ///         0020:  57 58 59 5a 61 62 63 64 65 66 67 68 69 6a 6b 6c  WXYZabcdefghijkl
    ///         0030:  6d 6e 6f 70 71 72 73 74 75 76 77 78 79 7a 20 7e  mnopqrstuvwxyz ~
    ///         0040:  21 40 23 24 25 5e 26 2a 28 29 5f 2b 60 2d 3d 5b  !@#$%^&*()_+`-=[
    ///         0050:  5d 5c 3b 27 2c 2e 2f 7b 7d 7c 3a 22 3c 3e 3f 00  ]\;',./{}|:"<>?.
    /// ```
    /// to the ephemeral log typically (and a more detailed message to the
    /// permanent log).  And similarly for the other log levels.  b should be
    /// safe against multiple evaluation.
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

        unsafe {
            match level {
                LogLevel::Debug | LogLevel::Info | LogLevel::Notice | LogLevel::Warning => {
                    fd_log_sys::fd_log_private_1(
                        level as c_int,
                        now,
                        c"hexdump".as_ptr(),
                        0,
                        c"SystemLogger::hexdump".as_ptr(),
                        hex_msg,
                    );
                }
                LogLevel::Error | LogLevel::Critical | LogLevel::Alert | LogLevel::Emergency => {
                    fd_log_sys::fd_log_private_2(
                        level as c_int,
                        now,
                        c"hexdump".as_ptr(),
                        0,
                        c"SystemLogger::hexdump".as_ptr(),
                        hex_msg,
                    );
                }
            }
        }
    }

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

#[derive(Debug, Clone)]
pub struct SystemLogBuilder {
    config: LogConfig,
}

impl Default for SystemLogBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemLogBuilder {
    pub fn new() -> Self {
        Self {
            config: LogConfig::default(),
        }
    }

    pub fn with_logfile_level(mut self, level: LogLevel) -> Self {
        self.config.level_logfile = level;
        SystemLogger::set_level_logfile(level);
        self
    }

    pub fn with_stderr_level(mut self, level: LogLevel) -> Self {
        self.config.level_stderr = level;
        SystemLogger::set_level_stderr(level);
        self
    }

    pub fn with_flush_level(mut self, level: LogLevel) -> Self {
        self.config.level_flush = level;
        SystemLogger::set_level_flush(level);
        self
    }

    pub fn with_core_level(mut self, level: LogLevel) -> Self {
        self.config.level_core = level;
        SystemLogger::set_level_core(level);
        self
    }

    pub fn with_colorize(mut self, colorize: bool) -> Self {
        self.config.colorize = colorize;
        SystemLogger::set_colorize(colorize);
        self
    }

    pub fn with_file<P: AsRef<str>>(mut self, path: P) -> Self {
        self.config.log_path = Some(path.as_ref().to_string());
        self
    }

    pub fn with_fd(mut self, fd: i32) -> Self {
        self.config.log_fd = Some(fd);
        self
    }

    pub fn with_dedup(mut self, dedup: bool) -> Self {
        self.config.dedup = dedup;
        self
    }

    pub fn with_app<S: AsRef<str>>(mut self, app: S) -> Self {
        self.config.app = Some(app.as_ref().to_string());
        self
    }

    pub fn with_thread<S: AsRef<str>>(mut self, thread: S) -> Self {
        let thread_str = thread.as_ref().to_string();
        self.config.thread = Some(thread_str.clone());
        SystemLogger::set_thread(&thread_str);
        self
    }

    pub fn with_cpu<S: AsRef<str>>(mut self, cpu: S) -> Self {
        let cpu_str = cpu.as_ref().to_string();
        self.config.cpu = Some(cpu_str.clone());
        SystemLogger::set_cpu(&cpu_str);
        self
    }

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

    pub fn init(self) -> Result<(), LogError> {
        if self.config.log_path.is_some() || self.config.log_fd.is_some() {
            LOGGER_INIT.call_once(|| {
                let result = SystemLogger::boot_custom(self.config);
                *LOGGER_INIT_RESULT.lock().unwrap() = Some(result);
            });

            LOGGER_INIT_RESULT.lock().unwrap().as_ref().unwrap().clone()
        } else {
            Ok(())
        }
    }

    pub fn try_init(self) -> Result<(), LogError> {
        match self.init() {
            Ok(()) => Ok(()),
            Err(LogError::InitializationFailed) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogError {
    InvalidPath,
    InvalidFd(i32),
    NulError,
    InitializationFailed,
}

impl std::fmt::Display for LogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogError::InvalidPath => write!(f, "Invalid log path"),
            LogError::InvalidFd(fd) => write!(f, "Invalid file descriptor: {}", fd),
            LogError::NulError => write!(f, "String contains null byte"),
            LogError::InitializationFailed => write!(f, "Log initialization failed"),
        }
    }
}

impl std::error::Error for LogError {}

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

/// Generic logging fn. mimics the behavior of `FD_LOG_*`.
///
/// This will do the following:
/// 1. Get a timestamp
/// 2. Format the message
/// 3. Call `fd_log_private_*`
pub fn _fd_log(level: LogLevel, file: &str, line: u32, func: &str, message: &str) {
    let now = unsafe { fd_log_sys::fd_log_wallclock() };
    let c_file = CString::new(file).unwrap_or_else(|_| CString::new("unknown").unwrap());
    let c_func = CString::new(func).unwrap_or_else(|_| CString::new("unknown").unwrap());
    let c_message = CString::new(message).unwrap_or_else(|_| CString::new("invalid_utf8").unwrap());

    // levels 0-3 - `fd_log_private_1` (non-fatal)
    // levels 4+ - `fd_log_private_2` (potentially fatal)
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
macro_rules! cpu {
    ($($arg:tt)*) => {
        $crate::SystemLogger::set_cpu(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! thread {
    ($($arg:tt)*) => {
        $crate::SystemLogger::set_thread(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! debug_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::SystemLogger::hexdump($crate::LogLevel::Debug, $tag, $data)
    };
}

#[macro_export]
macro_rules! info_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::SystemLogger::hexdump($crate::LogLevel::Info, $tag, $data)
    };
}

#[macro_export]
macro_rules! notice_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::SystemLogger::hexdump($crate::LogLevel::Notice, $tag, $data)
    };
}

#[macro_export]
macro_rules! warn_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::SystemLogger::hexdump($crate::LogLevel::Warning, $tag, $data)
    };
}

#[macro_export]
macro_rules! err_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::SystemLogger::hexdump($crate::LogLevel::Error, $tag, $data)
    };
}

#[macro_export]
macro_rules! crit_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::SystemLogger::hexdump($crate::LogLevel::Critical, $tag, $data)
    };
}

#[macro_export]
macro_rules! alert_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::SystemLogger::hexdump($crate::LogLevel::Alert, $tag, $data)
    };
}

#[macro_export]
macro_rules! emergency_hexdump {
    ($tag:expr, $data:expr) => {
        $crate::SystemLogger::hexdump($crate::LogLevel::Emergency, $tag, $data)
    };
}

/// Alias for `fd_log_debug` (FD_LOG_DEBUG)
///
/// Mainly for homogeneity with crates like `log` and `tracing`.
/// If collisions need to be avoided, use `fd_log_debug!` directly, or
/// reference this macro via `fd_log::debug!`.
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        $crate::fd_log_debug!($($arg)*)
    };
}

/// Alias for `fd_log_debug` (FD_LOG_DEBUG)
///
/// Mainly for homogeneity with crates like `log` and `tracing`.
/// If collisions need to be avoided, use `fd_log_debug!` directly, or
/// reference this macro via `fd_log::debug!`.
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::fd_log_debug!($($arg)*)
    };
}

/// Alias for `fd_log_info` (FD_LOG_INFO)
///
/// Mainly for homogeneity with crates like `log` and `tracing`.
/// If collisions need to be avoided, use `fd_log_info!` directly, or
/// reference this macro via `fd_log::info!`.
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::fd_log_info!($($arg)*)
    };
}

/// Alias for `fd_log_notice` (FD_LOG_NOTICE)
///
/// Mainly for homogeneity with crates like `log` and `tracing`.
/// If collisions need to be avoided, use `fd_log_notice!` directly, or
/// reference this macro via `fd_log::notice!`.
#[macro_export]
macro_rules! notice {
    ($($arg:tt)*) => {
        $crate::fd_log_notice!($($arg)*)
    };
}

/// Alias for `fd_log_warning` (FD_LOG_WARNING)
///
/// Mainly for homogeneity with crates like `log` and `tracing`.
/// If collisions need to be avoided, use `fd_log_warning!` directly, or
/// reference this macro via `fd_log::warn!`.
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::fd_log_warning!($($arg)*)
    };
}

/// Alias for `fd_log_error` (FD_LOG_ERR)
///
/// ### WARNING:
/// This will exit the program with a SIGABRT signal
///
/// Mainly for homogeneity with crates like `log` and `tracing`.
/// If collisions need to be avoided, use `fd_log_error!` directly, or
/// reference this macro via `fd_log::error!`.
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::fd_log_error!($($arg)*)
    };
}

/// Alias for `fd_log_emergency` (FD_LOG_EMERG)
///
/// ### WARNING:
/// This will exit the program with a SIGABRT signal
///
/// Mainly for homogeneity with crates like `log` and `tracing`.
/// If collisions need to be avoided, use `fd_log_emergency!` directly, or
/// reference this macro via `fd_log::emergency!`.
#[macro_export]
macro_rules! emergency {
    ($($arg:tt)*) => {
        $crate::fd_log_emergency!($($arg)*)
    };
}

/// Alias for `fd_log_alert` (FD_LOG_ALERT)
///
/// ### WARNING:
/// This will exit the program with a SIGABRT signal
///
/// Mainly for homogeneity with crates like `log` and `tracing`.
/// If collisions need to be avoided, use `fd_log_alert!` directly, or
/// reference this macro via `fd_log::alert!`.
#[macro_export]
macro_rules! alert {
    ($($arg:tt)*) => {
        $crate::fd_log_alert!($($arg)*)
    };
}

/// Alias for `fd_log_critical` (FD_LOG_CRIT)
///
/// ### WARNING:
/// This will exit the program with a SIGABRT signal
///
/// Mainly for homogeneity with crates like `log` and `tracing`.
/// If collisions need to be avoided, use `fd_log_emergency!` directly, or
/// reference this macro via `fd_log::emergency!`.
#[macro_export]
macro_rules! critical {
    ($($arg:tt)*) => {
        $crate::fd_log_critical!($($arg)*)
    };
}

#[macro_export]
macro_rules! fd_log_debug {
    ($($arg:tt)*) => {
        $crate::_fd_log_parse_internal!($crate::LogLevel::Debug, $($arg)*)
    };
}

#[macro_export]
macro_rules! fd_log_info {
    ($($arg:tt)*) => {
        $crate::_fd_log_parse_internal!($crate::LogLevel::Info, $($arg)*)
    };
}

#[macro_export]
macro_rules! fd_log_notice {
    ($($arg:tt)*) => {
        $crate::_fd_log_parse_internal!($crate::LogLevel::Notice, $($arg)*)
    };
}

#[macro_export]
macro_rules! fd_log_warning {
    ($($arg:tt)*) => {
        $crate::_fd_log_parse_internal!($crate::LogLevel::Warning, $($arg)*)
    };
}

#[macro_export]
macro_rules! fd_log_error {
    ($($arg:tt)*) => {
        $crate::_fd_log_parse_internal!($crate::LogLevel::Error, $($arg)*)
    };
}

#[macro_export]
macro_rules! fd_log_critical {
    ($($arg:tt)*) => {
        $crate::_fd_log_parse_internal!($crate::LogLevel::Critical, $($arg)*)
    };
}

#[macro_export]
macro_rules! fd_log_alert {
    ($($arg:tt)*) => {
        $crate::_fd_log_parse_internal!($crate::LogLevel::Alert, $($arg)*)
    };
}

#[macro_export]
macro_rules! fd_log_emergency {
    ($($arg:tt)*) => {
        $crate::_fd_log_parse_internal!($crate::LogLevel::Emergency, $($arg)*)
    };
}

#[macro_export]
macro_rules! _fd_log_parse_internal {
    (@parse $level:expr, [], $($rest:tt)*) => {
        $crate::_fd_log_internal!(@simple $level, $($rest)*)
    };
    (@parse $level:expr, [ $($key:ident = $value:expr),+ ], $($rest:tt)*) => {
        $crate::_fd_log_internal!(@structured $level, [ $($key = $value),* ], $($rest)*)
    };
    ($level:expr, $key:ident = $value:expr, $($rest:tt)*) => {
        $crate::_fd_log_parse_internal!(@accumulate $level, [ $key = $value ], $($rest)*)
    };
    ($level:expr, $($rest:tt)*) => {
        $crate::_fd_log_internal!(@simple $level, $($rest)*)
    };
    (@accumulate $level:expr, [ $($keys:ident = $values:expr),* ], $key:ident = $value:expr, $($rest:tt)*) => {
        $crate::_fd_log_parse_internal!(@accumulate $level, [ $($keys = $values,)* $key = $value ], $($rest)*)
    };
    (@accumulate $level:expr, [ $($keys:ident = $values:expr),* ], $($rest:tt)*) => {
        $crate::_fd_log_internal!(@structured $level, [ $($keys = $values),* ], $($rest)*)
    };
}

#[macro_export]
macro_rules! _fd_log_format_internal {
    (@format $key:ident) => {
        concat!(stringify!($key), " = {:?}")
    };
    (@format $first:ident, $($rest:ident),+) => {
        concat!(
            stringify!($first), " = {:?}",
            $(", ", stringify!($rest), " = {:?}"),+
        )
    };
}

//  bold/italic attributes (much like `log` or `tracing`), could be done with:
//  $( "\x1b[1;3m", stringify!($key), "\x1b[0m=\x1b[3m{:?}\x1b[0m, ", )*
#[macro_export]
macro_rules! _fd_log_internal {
    (@structured $level:expr, [ $($key:ident = $value:expr),* ], $fmt:expr, $($args:expr),*) => {
        $crate::_fd_log(
            $level,
            file!(),
            line!(),
            module_path!(),
            &format!(
                concat!(
                    $fmt,
                    " {{ ",
                    $crate::_fd_log_format_internal!(@format $($key),*),
                    " }}"
                ),
                $($args,)* $($value,)*
            ),
        )
    };
    (@structured $level:expr, [ $($key:ident = $value:expr),* ], $fmt:expr) => {
        $crate::_fd_log(
            $level,
            file!(),
            line!(),
            module_path!(),
            &format!(
                concat!(
                    $fmt,
                    " {{ ",
                    $crate::_fd_log_format_internal!(@format $($key),*),
                    " }}"
                ),
                $($value,)*
            ),
        )
    };
    (@simple $level:expr, $fmt:expr $(, $args:expr)*) => {
        $crate::_fd_log(
            $level,
            file!(),
            line!(),
            module_path!(),
            &format!($fmt $(, $args)*),
        )
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
        let _app_id = SystemLogger::app_id();
        let _thread_id = SystemLogger::thread_id();
        let _host_id = SystemLogger::host_id();
        let _cpu_id = SystemLogger::cpu_id();
        let _group_id = SystemLogger::group_id();
        let _tid = SystemLogger::tid();
        let _user_id = SystemLogger::user_id();

        assert_eq!(_app_id, 0);
        assert_eq!(_thread_id, 0);
        assert_eq!(_host_id, 0);
        assert_eq!(_group_id, 0);

        assert_ne!(_cpu_id, 0);
        assert_ne!(_tid, 0);
        assert_ne!(_user_id, 0);

        let _wallclock = SystemLogger::wallclock_host();
        assert_ne!(_wallclock, 0);
    }

    /// omit `fd_log_private_2` level usage here since they'll nuke the process
    #[test_case(true; "test_recoverable_colorized")]
    fn test_recoverable(colorize: bool) {
        SystemLogger::set_colorize(colorize);

        debug!("Debug message; low prio");
        info!("Info message; value={}", 42);
        notice!("Notice message; medium priority");
        warn!("Warning message; medium-high priority");
    }

    #[test]
    fn test_builder() {
        SystemLogBuilder::default()
            .with_colorize(true)
            .with_app("tachyon")
            .with_file("./tachyon-fd-log.log")
            .with_stderr_level(LogLevel::Info)
            .with_logfile_level(LogLevel::Debug)
            .init()
            .expect("Failed to init logging");

        let num = 10;
        const SOME_NUM: i32 = 42;

        info!("This is a test log from the builder!");
        info!("Log file descriptor: {:?}", SystemLogger::logfile_fd());
        info!(
            attribute_1 = 12,
            thing = "something",
            "This is a test log from the builder!"
        );

        let cpuid = SystemLogger::cpu_id();
        let hostid = SystemLogger::host_id();

        warn!("Inline formatting {num}");
        debug!("Const inline: {SOME_NUM}");
        notice!(
            event_id = 0x05,
            signature = [0u8; 16],
            "An event occurred at CPU {}, Host {}",
            cpuid,
            hostid
        );

        SystemLogger::flush();

        info!("Flushed to file './tachyon-fd-log.log'");
    }

    #[ignore]
    #[test_case(LogLevel::Error, "ERROR! SOMETHING HAPPENED"; "test_error")]
    #[test_case(LogLevel::Critical, "CRITICAL! SOMETHING IS SERIOUSLY WRONG"; "test_critical")]
    #[test_case(LogLevel::Alert, "RED ALERT! SOMETHING IS CRITICALLY WRONG"; "test_alert")]
    #[test_case(LogLevel::Emergency, "EMERGENCY! SOMETHING IS CRITICALLY WRONG"; "test_emergency")]
    #[should_panic]
    fn test_unrecoverable(level: LogLevel, message: &str) {
        // This will make sure we panic rather than abort immediately, so the
        // test will pass correctly.
        sighandler();

        match level {
            LogLevel::Emergency => emergency!("{}", message),
            LogLevel::Error => error!("{}", message),
            LogLevel::Alert => alert!("{}", message),
            LogLevel::Critical => critical!("{}", message),
            _ => (),
        }
    }

    #[test]
    #[should_panic]
    fn test_everything() {
        sighandler();

        SystemLogBuilder::default()
            .with_colorize(true)
            .with_app("tachyon")
            .with_file("./tachyon-fd-log.log")
            .with_stderr_level(LogLevel::Info)
            .with_logfile_level(LogLevel::Debug)
            .init()
            .expect("Failed to init logging");

        info!(
            attribute_1 = 12,
            thing = "something",
            "This is a test log from the builder!"
        );

        let cpuid = SystemLogger::cpu_id();
        let hostid = SystemLogger::host_id();
        let ts = SystemLogger::wallclock();

        notice!(
            event_id = 0x05,
            signature = [0u8; 16],
            timestamp = ts,
            "An event occurred at CPU {}, Host {}",
            cpuid,
            hostid
        );

        // need `fd_yield` for this one
        //SystemLogger::wait_until(ts + 5_000);

        warn!(
            ts = SystemLogger::wallclock_host(),
            "Something might be going wrong?"
        );

        alert_hexdump!("SOMETHING IS WRONG!", &[0u8; 64]);
    }
}
