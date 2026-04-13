//! Safe Rust wrapper for Firedancer's TOML parser.
//!
//! This crate provides safe, idiomatic Rust bindings for the Firedancer
//! `fd_toml` parser, which deserializes TOML documents into `fd_pod`
//! key-value hierarchies. All memory is caller-provided — no heap
//! allocations occur in the parsing path.
//!
//! # Type Mapping
//!
//! | TOML type      | Example       | Pod type   | Rust query        |
//! |----------------|---------------|------------|-------------------|
//! | string         | `'hello'`     | cstr       | `query_cstr`      |
//! | integer        | `-3`          | long       | `query_long`      |
//! | float          | `3e-3`        | float      | `query_float`     |
//! | boolean        | `true`        | int        | `query_bool`      |
//! | datetime       | `2022-08-16`  | ulong (ns) | `query_ulong`     |
//! | table          | `[key]`       | subpod     | `query_subpod`    |
//! | inline table   | `x={a=1}`     | subpod     | `query_subpod`    |
//! | array          | `x=[1,2]`     | subpod     | `query_subpod`    |
//!
//! # Example
//!
//! ```rust
//! use fd_toml::{TomlParser, TomlPod};
//!
//! let toml = b"title = \"example\"\n[server]\nport = 8080\n";
//! let mut parser = TomlParser::<4096, 4096>::new();
//! let pod = parser.parse(toml).unwrap();
//!
//! assert_eq!(pod.query_cstr("title"), Some("example"));
//! assert_eq!(pod.query_long("server.port"), Some(8080));
//! ```

use core::ffi::{c_char, c_void, CStr};
use core::fmt;
use fd_toml_sys as sys;

const DEFAULT_SENTINEL_CSTR: *const c_char = core::ptr::null();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TomlError {
    PodFull,
    ScratchFull,
    KeyTooLong,
    DuplicateKey,
    Overflow,
    ParseFailure { line: u64 },
}

impl fmt::Display for TomlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TomlError::PodFull => write!(f, "ran out of output pod space"),
            TomlError::ScratchFull => write!(f, "ran out of scratch space"),
            TomlError::KeyTooLong => write!(f, "key exceeds FD_TOML_PATH_MAX"),
            TomlError::DuplicateKey => write!(f, "duplicate key"),
            TomlError::Overflow => write!(f, "integer overflow"),
            TomlError::ParseFailure { line } => write!(f, "parse failure at line {line}"),
        }
    }
}

impl core::error::Error for TomlError {}

fn translate_error(code: i32, line: u64) -> TomlError {
    match code {
        sys::FD_TOML_ERR_POD => TomlError::PodFull,
        sys::FD_TOML_ERR_SCRATCH => TomlError::ScratchFull,
        sys::FD_TOML_ERR_KEY => TomlError::KeyTooLong,
        sys::FD_TOML_ERR_DUP => TomlError::DuplicateKey,
        sys::FD_TOML_ERR_RANGE => TomlError::Overflow,
        _ => TomlError::ParseFailure { line },
    }
}

/// A TOML parser with statically-sized pod and scratch buffers.
///
/// `POD_SZ` is the maximum byte size of the output pod. Larger TOML
/// documents with many keys require a larger pod. `SCRATCH_SZ` is
/// scratch space used during parsing (4096 is a good default).
pub struct TomlParser<const POD_SZ: usize, const SCRATCH_SZ: usize> {
    pod_mem: [u8; POD_SZ],
    scratch: [u8; SCRATCH_SZ],
}

impl<const POD_SZ: usize, const SCRATCH_SZ: usize> TomlParser<POD_SZ, SCRATCH_SZ> {
    pub fn new() -> Self {
        assert!(POD_SZ >= sys::FD_POD_FOOTPRINT_MIN as usize);
        Self {
            pod_mem: [0u8; POD_SZ],
            scratch: [0u8; SCRATCH_SZ],
        }
    }

    /// Parse a TOML document, returning a reference to the populated pod.
    ///
    /// The returned [`TomlPod`] borrows from this parser and provides
    /// typed query methods.
    pub fn parse(&mut self, toml: &[u8]) -> Result<TomlPod<'_>, TomlError> {
        self.pod_mem.fill(0);

        let pod = unsafe {
            let raw = sys::fd_pod_new(self.pod_mem.as_mut_ptr() as *mut c_void, POD_SZ as u64);
            if raw.is_null() {
                return Err(TomlError::PodFull);
            }
            sys::fd_pod_join(raw)
        };
        if pod.is_null() {
            return Err(TomlError::PodFull);
        }

        let mut err_info = sys::fd_toml_err_info { line: 0 };
        let (data, len) = if toml.is_empty() {
            (core::ptr::null(), 0u64)
        } else {
            (toml.as_ptr() as *const c_void, toml.len() as u64)
        };

        let result = unsafe {
            sys::fd_toml_parse(
                data,
                len,
                pod,
                self.scratch.as_mut_ptr(),
                self.scratch.len() as u64,
                &mut err_info,
            )
        };

        if result != sys::FD_TOML_SUCCESS as i32 {
            unsafe {
                sys::fd_pod_delete(sys::fd_pod_leave(pod));
            }
            return Err(translate_error(result, err_info.line));
        }

        Ok(TomlPod {
            pod,
            _lifetime: core::marker::PhantomData,
        })
    }
}

impl<const POD_SZ: usize, const SCRATCH_SZ: usize> Default for TomlParser<POD_SZ, SCRATCH_SZ> {
    fn default() -> Self {
        Self::new()
    }
}

/// A parsed TOML document stored in an fd_pod.
///
/// Provides safe, typed access to the key-value pairs produced by the
/// TOML parser. Paths use `.` as a separator for nested tables
/// (e.g. `"server.port"`).
pub struct TomlPod<'a> {
    pod: *mut sys::uchar,
    _lifetime: core::marker::PhantomData<&'a ()>,
}

impl<'a> TomlPod<'a> {
    /// Number of top-level key-value pairs.
    pub fn len(&self) -> usize {
        unsafe { sys::fd_pod_cnt(self.pod) as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Query a string value. Returns `None` if the path doesn't exist
    /// or isn't a cstr.
    pub fn query_cstr(&self, path: &str) -> Option<&str> {
        let c_path = make_cstr_on_stack(path)?;
        let ptr =
            unsafe { sys::fd_pod_query_cstr(self.pod, c_path.as_ptr(), DEFAULT_SENTINEL_CSTR) };
        if ptr.is_null() {
            return None;
        }
        let cstr = unsafe { CStr::from_ptr(ptr) };
        cstr.to_str().ok()
    }

    /// Query a long (i64) value. Returns `None` if the path doesn't exist.
    pub fn query_long(&self, path: &str) -> Option<i64> {
        let c_path = make_cstr_on_stack(path)?;
        let sentinel = i64::MIN;
        let val = unsafe { sys::fd_pod_query_long(self.pod, c_path.as_ptr(), sentinel) };
        if val == sentinel {
            if self.path_exists(path) {
                Some(val)
            } else {
                None
            }
        } else {
            Some(val)
        }
    }

    /// Query a ulong (u64) value. Returns `None` if the path doesn't exist.
    pub fn query_ulong(&self, path: &str) -> Option<u64> {
        let c_path = make_cstr_on_stack(path)?;
        let sentinel = u64::MAX;
        let val = unsafe { sys::fd_pod_query_ulong(self.pod, c_path.as_ptr(), sentinel) };
        if val == sentinel {
            if self.path_exists(path) {
                Some(val)
            } else {
                None
            }
        } else {
            Some(val)
        }
    }

    /// Query a float (f32) value. Returns `None` if the path doesn't exist.
    pub fn query_float(&self, path: &str) -> Option<f32> {
        let c_path = make_cstr_on_stack(path)?;
        let sentinel = f32::NAN;
        let val = unsafe { sys::fd_pod_query_float(self.pod, c_path.as_ptr(), sentinel) };
        if val.is_nan() {
            if self.path_exists(path) {
                Some(val)
            } else {
                None
            }
        } else {
            Some(val)
        }
    }

    /// Query an int (i32) value. Returns `None` if the path doesn't exist.
    pub fn query_int(&self, path: &str) -> Option<i32> {
        let c_path = make_cstr_on_stack(path)?;
        let sentinel = i32::MIN;
        let val = unsafe { sys::fd_pod_query_int(self.pod, c_path.as_ptr(), sentinel) };
        if val == sentinel {
            if self.path_exists(path) {
                Some(val)
            } else {
                None
            }
        } else {
            Some(val)
        }
    }

    /// Query a boolean value. TOML booleans are stored as int (0 or 1).
    pub fn query_bool(&self, path: &str) -> Option<bool> {
        self.query_int(path).map(|v| v != 0)
    }

    /// Check whether a sub-pod (table/array) exists at the given path.
    pub fn query_subpod(&self, path: &str) -> bool {
        let Some(c_path) = make_cstr_on_stack(path) else {
            return false;
        };
        let ptr = unsafe { sys::fd_pod_query_subpod(self.pod, c_path.as_ptr()) };
        !ptr.is_null()
    }

    /// Check whether any value exists at the given path.
    pub fn path_exists(&self, path: &str) -> bool {
        let Some(c_path) = make_cstr_on_stack(path) else {
            return false;
        };
        let result = unsafe { sys::fd_pod_query(self.pod, c_path.as_ptr(), core::ptr::null_mut()) };
        result == sys::FD_POD_SUCCESS as i32
    }
}

impl Drop for TomlPod<'_> {
    fn drop(&mut self) {
        unsafe {
            sys::fd_pod_delete(sys::fd_pod_leave(self.pod));
        }
    }
}

impl fmt::Debug for TomlPod<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TomlPod")
            .field("entries", &self.len())
            .finish()
    }
}

struct CStrBuf {
    buf: [c_char; 513],
}

impl CStrBuf {
    fn as_ptr(&self) -> *const c_char {
        self.buf.as_ptr()
    }
}

fn make_cstr_on_stack(path: &str) -> Option<CStrBuf> {
    if path.len() >= 513 {
        return None;
    }
    let mut result = CStrBuf {
        buf: [0 as c_char; 513],
    };
    for (i, &b) in path.as_bytes().iter().enumerate() {
        result.buf[i] = b as c_char;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(b"").unwrap();
        assert_eq!(pod.len(), 0);
        assert!(pod.is_empty());
    }

    #[test]
    fn test_parse_string() {
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(b"name = \"hello\"").unwrap();
        assert_eq!(pod.query_cstr("name"), Some("hello"));
    }

    #[test]
    fn test_parse_integer() {
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(b"port = 8080").unwrap();
        assert_eq!(pod.query_long("port"), Some(8080));
    }

    #[test]
    fn test_parse_negative_integer() {
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(b"offset = -42").unwrap();
        assert_eq!(pod.query_long("offset"), Some(-42));
    }

    #[test]
    fn test_parse_boolean() {
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(b"enabled = true\ndisabled = false").unwrap();
        assert_eq!(pod.query_bool("enabled"), Some(true));
        assert_eq!(pod.query_bool("disabled"), Some(false));
    }

    #[test]
    fn test_parse_float() {
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(b"val = 1e2").unwrap();
        let val = pod.query_float("val").unwrap();
        assert!((val - 100.0f32).abs() < 0.01);
    }

    #[test]
    fn test_parse_table() {
        let toml = b"[server]\nhost = \"localhost\"\nport = 443\n";
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(toml).unwrap();
        assert!(pod.query_subpod("server"));
        assert_eq!(pod.query_cstr("server.host"), Some("localhost"));
        assert_eq!(pod.query_long("server.port"), Some(443));
    }

    #[test]
    fn test_parse_nested_table() {
        let toml = b"[database]\nname = \"mydb\"\n[database.pool]\nmax = 10\n";
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(toml).unwrap();
        assert_eq!(pod.query_cstr("database.name"), Some("mydb"));
        assert_eq!(pod.query_long("database.pool.max"), Some(10));
    }

    #[test]
    fn test_missing_key() {
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(b"key = 1").unwrap();
        assert_eq!(pod.query_cstr("nonexistent"), None);
        assert_eq!(pod.query_long("nonexistent"), None);
        assert_eq!(pod.query_bool("nonexistent"), None);
        assert!(!pod.path_exists("nonexistent"));
    }

    #[test]
    fn test_parse_error() {
        let mut parser = TomlParser::<4096, 4096>::new();
        let result = parser.parse(b"= bad");
        assert!(result.is_err());
        assert!(matches!(result, Err(TomlError::ParseFailure { .. })));
    }

    #[test]
    fn test_multiline_document() {
        let toml = b"\
title = \"Config\"\n\
version = 2\n\
\n\
[logging]\n\
level = \"info\"\n\
verbose = false\n\
\n\
[network]\n\
bind = \"0.0.0.0\"\n\
port = 9000\n\
";
        let mut parser = TomlParser::<8192, 4096>::new();
        let pod = parser.parse(toml).unwrap();
        assert_eq!(pod.query_cstr("title"), Some("Config"));
        assert_eq!(pod.query_long("version"), Some(2));
        assert_eq!(pod.query_cstr("logging.level"), Some("info"));
        assert_eq!(pod.query_bool("logging.verbose"), Some(false));
        assert_eq!(pod.query_cstr("network.bind"), Some("0.0.0.0"));
        assert_eq!(pod.query_long("network.port"), Some(9000));
    }

    #[test]
    fn test_inline_table() {
        let toml = b"point = {x = 1, y = 2}";
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(toml).unwrap();
        assert!(pod.query_subpod("point"));
        assert_eq!(pod.query_long("point.x"), Some(1));
        assert_eq!(pod.query_long("point.y"), Some(2));
    }

    #[test]
    fn test_array() {
        let toml = b"ports = [80, 443, 8080]";
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(toml).unwrap();
        assert!(pod.query_subpod("ports"));
        assert_eq!(pod.query_long("ports.0"), Some(80));
        assert_eq!(pod.query_long("ports.1"), Some(443));
        assert_eq!(pod.query_long("ports.2"), Some(8080));
    }

    #[test]
    fn test_hex_integer() {
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(b"color = 0xff").unwrap();
        assert_eq!(pod.query_long("color"), Some(255));
    }

    #[test]
    fn test_error_display() {
        let err = TomlError::ParseFailure { line: 5 };
        assert_eq!(err.to_string(), "parse failure at line 5");

        let err = TomlError::DuplicateKey;
        assert_eq!(err.to_string(), "duplicate key");
    }

    #[test]
    fn test_path_exists() {
        let toml = b"a = 1\n[b]\nc = 2\n";
        let mut parser = TomlParser::<4096, 4096>::new();
        let pod = parser.parse(toml).unwrap();
        assert!(pod.path_exists("a"));
        assert!(pod.path_exists("b"));
        assert!(pod.path_exists("b.c"));
        assert!(!pod.path_exists("d"));
    }
}
