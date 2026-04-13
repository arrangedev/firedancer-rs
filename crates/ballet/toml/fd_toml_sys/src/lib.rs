//! Raw FFI bindings to the Firedancer TOML parser.
//!
//! This crate provides raw, unsafe bindings to the Firedancer `fd_toml` C library,
//! which parses TOML documents into `fd_pod` key-value hierarchies.
//!
//! For safe, idiomatic Rust wrappers, see the `fd_toml` crate.
//!
//! # Safety
//!
//! All functions in this crate are unsafe and require careful handling of:
//! - Proper allocation and initialization of `fd_pod` memory
//! - Scratch buffer sizing (4kB+ recommended)
//! - Pointer validity for TOML input data
//!
//! # Main Operations
//!
//! - `fd_toml_parse`: Parse a TOML document into an fd_pod
//! - `fd_toml_strerror`: Convert error codes to human-readable strings
//! - `fd_pod_new` / `fd_pod_join` / `fd_pod_leave` / `fd_pod_delete`: Pod lifecycle
//! - `fd_pod_query_*`: Query typed values from the pod

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strerror() {
        unsafe {
            let s = fd_toml_strerror(FD_TOML_SUCCESS as i32);
            assert!(!s.is_null());
            let cstr = core::ffi::CStr::from_ptr(s);
            assert_eq!(cstr.to_str().unwrap(), "success");

            let s = fd_toml_strerror(FD_TOML_ERR_PARSE);
            assert!(!s.is_null());
            let cstr = core::ffi::CStr::from_ptr(s);
            assert_eq!(cstr.to_str().unwrap(), "parse failure");
        }
    }

    #[test]
    fn test_pod_lifecycle() {
        unsafe {
            let mut pod_mem = [0u8; 4096];
            let pod = fd_pod_new(pod_mem.as_mut_ptr() as *mut _, 4096);
            assert!(!pod.is_null());
            let pod = fd_pod_join(pod);
            assert!(!pod.is_null());

            let cnt = fd_pod_cnt(pod);
            assert_eq!(cnt, 0);

            fd_pod_delete(fd_pod_leave(pod));
        }
    }

    #[test]
    fn test_parse_empty() {
        unsafe {
            let mut pod_mem = [0u8; 4096];
            let pod = fd_pod_join(fd_pod_new(pod_mem.as_mut_ptr() as *mut _, 4096));
            assert!(!pod.is_null());

            let mut scratch = [0u8; 4096];
            let result = fd_toml_parse(
                core::ptr::null(),
                0,
                pod,
                scratch.as_mut_ptr(),
                scratch.len() as u64,
                core::ptr::null_mut(),
            );
            assert_eq!(result, FD_TOML_SUCCESS as i32);

            fd_pod_delete(fd_pod_leave(pod));
        }
    }

    #[test]
    fn test_parse_simple_kv() {
        unsafe {
            let mut pod_mem = [0u8; 4096];
            let pod = fd_pod_join(fd_pod_new(pod_mem.as_mut_ptr() as *mut _, 4096));

            let toml = b"key = \"value\"\nnum = 42\nbool_val = true\n";
            let mut scratch = [0u8; 4096];
            let mut err_info = fd_toml_err_info { line: 0 };
            let result = fd_toml_parse(
                toml.as_ptr() as *const _,
                toml.len() as u64,
                pod,
                scratch.as_mut_ptr(),
                scratch.len() as u64,
                &mut err_info,
            );
            assert_eq!(result, FD_TOML_SUCCESS as i32);
            assert!(fd_pod_cnt(pod) > 0);

            fd_pod_delete(fd_pod_leave(pod));
        }
    }

    #[test]
    fn test_parse_table() {
        unsafe {
            let mut pod_mem = [0u8; 4096];
            let pod = fd_pod_join(fd_pod_new(pod_mem.as_mut_ptr() as *mut _, 4096));

            let toml = b"[section]\nname = \"test\"\nvalue = 123\n";
            let mut scratch = [0u8; 4096];
            let result = fd_toml_parse(
                toml.as_ptr() as *const _,
                toml.len() as u64,
                pod,
                scratch.as_mut_ptr(),
                scratch.len() as u64,
                core::ptr::null_mut(),
            );
            assert_eq!(result, FD_TOML_SUCCESS as i32);

            fd_pod_delete(fd_pod_leave(pod));
        }
    }

    #[test]
    fn test_parse_error() {
        unsafe {
            let mut pod_mem = [0u8; 4096];
            let pod = fd_pod_join(fd_pod_new(pod_mem.as_mut_ptr() as *mut _, 4096));

            let toml = b"= invalid";
            let mut scratch = [0u8; 4096];
            let mut err_info = fd_toml_err_info { line: 0 };
            let result = fd_toml_parse(
                toml.as_ptr() as *const _,
                toml.len() as u64,
                pod,
                scratch.as_mut_ptr(),
                scratch.len() as u64,
                &mut err_info,
            );
            assert_ne!(result, FD_TOML_SUCCESS as i32);

            fd_pod_delete(fd_pod_leave(pod));
        }
    }
}
