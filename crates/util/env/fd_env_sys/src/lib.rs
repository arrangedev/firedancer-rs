//! Raw FFI bindings to Firedancer environment utilities
//!
//! This crate provides low-level, unsafe bindings to the Firedancer environment utilities:
//! - Command line argument parsing and stripping
//! - Environment variable reading with fallback to command line
//! - Type conversion from strings to various primitive types
//! - Support for modular command line parsing between independent units
//!
//! For a safe Rust API, consider using the higher-level wrapper crate.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr;

    #[test]
    fn test_env_strip_cmdline_contains() {
        unsafe {
            let mut args = vec![
                CString::new("program").unwrap(),
                CString::new("--test").unwrap(),
                CString::new("--other").unwrap(),
                CString::new("value").unwrap(),
            ];

            let mut argv: Vec<*mut i8> = args.iter_mut().map(|s| s.as_ptr() as *mut i8).collect();
            argv.push(ptr::null_mut());

            let mut argc = (argv.len() - 1) as i32;
            let mut argv_ptr = argv.as_mut_ptr();

            let key = CString::new("--test").unwrap();
            let result = fd_env_strip_cmdline_contains(
                &mut argc,
                argv_ptr as *mut *mut *mut core::ffi::c_char,
                key.as_ptr(),
            );

            assert_eq!(result, 1);
            assert_eq!(argc, 3);
        }
    }

    #[test]
    fn test_env_strip_cmdline_ulong() {
        unsafe {
            let mut args = vec![
                CString::new("program").unwrap(),
                CString::new("--count").unwrap(),
                CString::new("42").unwrap(),
                CString::new("--other").unwrap(),
            ];

            let mut argv: Vec<*mut i8> = args.iter_mut().map(|s| s.as_ptr() as *mut i8).collect();
            argv.push(ptr::null_mut());

            let mut argc = (argv.len() - 1) as i32;
            let mut argv_ptr = argv.as_mut_ptr();

            let key = CString::new("--count").unwrap();
            let env_key = ptr::null();
            let default_val = 0u64;

            let result = fd_env_strip_cmdline_ulong(
                &mut argc,
                argv_ptr as *mut *mut *mut core::ffi::c_char,
                key.as_ptr(),
                env_key,
                default_val,
            );

            assert_eq!(result, 42);
            assert_eq!(argc, 2);
        }
    }

    #[test]
    fn test_bindings_exist() {
        unsafe {
            let mut argc = 0i32;
            let mut argv_ptr = ptr::null_mut();
            let key = ptr::null();
            let env_key = ptr::null();

            let _result =
                fd_env_strip_cmdline_ulong(&mut argc, &mut argv_ptr, key, env_key, 123u64);
            let _contains = fd_env_strip_cmdline_contains(
                &mut argc,
                argv_ptr as *mut *mut *mut core::ffi::c_char,
                key,
            );
        }
    }
}
