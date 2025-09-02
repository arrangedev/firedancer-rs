//! Safe API for Firedancer environment utilities
//!
//! This wraps the FFI bindings provided by `libfd_env_sys` and provides
//! safer abstractions for their use.
//!
//! ## Structure
//!
//! - `cmdline`: Command line argument parsing and stripping utilities
//! - `env`: Environment variable reading utilities

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

pub mod cmdline {
    use super::*;

    pub struct CommandLine {
        args: Vec<CString>,
        argv: Vec<*mut c_char>,
        argc: i32,
    }

    impl CommandLine {
        pub fn new(args: Vec<String>) -> Self {
            let mut cstring_args: Vec<CString> = args
                .into_iter()
                .map(|s| CString::new(s).unwrap_or_else(|_| CString::new("").unwrap()))
                .collect();

            let mut argv: Vec<*mut c_char> = cstring_args
                .iter_mut()
                .map(|s| s.as_ptr() as *mut c_char)
                .collect();
            argv.push(ptr::null_mut());

            let argc = (argv.len() - 1) as i32;

            Self {
                args: cstring_args,
                argv,
                argc,
            }
        }

        pub fn to_vec(&self) -> Vec<String> {
            (0..self.argc)
                .map(|i| unsafe {
                    CStr::from_ptr(self.argv[i as usize])
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        }

        pub fn len(&self) -> usize {
            self.argc as usize
        }

        pub fn is_empty(&self) -> bool {
            self.argc == 0
        }

        pub fn strip_ulong(&mut self, key: &str, env_key: Option<&str>, default: u64) -> u64 {
            let c_key = CString::new(key).unwrap();
            let c_env_key = env_key.map(|k| CString::new(k).unwrap());
            let env_key_ptr = c_env_key
                .as_ref()
                .map(|k| k.as_ptr())
                .unwrap_or(ptr::null());

            unsafe {
                libfd_env_sys::fd_env_strip_cmdline_ulong(
                    &mut self.argc,
                    &mut self.argv.as_mut_ptr(),
                    c_key.as_ptr(),
                    env_key_ptr,
                    default,
                )
            }
        }

        pub fn strip_uint(&mut self, key: &str, env_key: Option<&str>, default: u32) -> u32 {
            let c_key = CString::new(key).unwrap();
            let c_env_key = env_key.map(|k| CString::new(k).unwrap());
            let env_key_ptr = c_env_key
                .as_ref()
                .map(|k| k.as_ptr())
                .unwrap_or(ptr::null());

            unsafe {
                libfd_env_sys::fd_env_strip_cmdline_uint(
                    &mut self.argc,
                    &mut self.argv.as_mut_ptr(),
                    c_key.as_ptr(),
                    env_key_ptr,
                    default,
                )
            }
        }

        pub fn strip_int(&mut self, key: &str, env_key: Option<&str>, default: i32) -> i32 {
            let c_key = CString::new(key).unwrap();
            let c_env_key = env_key.map(|k| CString::new(k).unwrap());
            let env_key_ptr = c_env_key
                .as_ref()
                .map(|k| k.as_ptr())
                .unwrap_or(ptr::null());

            unsafe {
                libfd_env_sys::fd_env_strip_cmdline_int(
                    &mut self.argc,
                    &mut self.argv.as_mut_ptr(),
                    c_key.as_ptr(),
                    env_key_ptr,
                    default,
                )
            }
        }

        pub fn strip_long(&mut self, key: &str, env_key: Option<&str>, default: i64) -> i64 {
            let c_key = CString::new(key).unwrap();
            let c_env_key = env_key.map(|k| CString::new(k).unwrap());
            let env_key_ptr = c_env_key
                .as_ref()
                .map(|k| k.as_ptr())
                .unwrap_or(ptr::null());

            unsafe {
                libfd_env_sys::fd_env_strip_cmdline_long(
                    &mut self.argc,
                    &mut self.argv.as_mut_ptr(),
                    c_key.as_ptr(),
                    env_key_ptr,
                    default,
                )
            }
        }

        /// Strip a key-value pair from the command line and return the string
        pub fn strip_cstr(&mut self, key: &str, env_key: Option<&str>, default: &str) -> String {
            let c_key = CString::new(key).unwrap();
            let c_env_key = env_key.map(|k| CString::new(k).unwrap());
            let c_default = CString::new(default).unwrap();
            let env_key_ptr = c_env_key
                .as_ref()
                .map(|k| k.as_ptr())
                .unwrap_or(ptr::null());

            unsafe {
                let result = libfd_env_sys::fd_env_strip_cmdline_cstr(
                    &mut self.argc,
                    &mut self.argv.as_mut_ptr(),
                    c_key.as_ptr(),
                    env_key_ptr,
                    c_default.as_ptr(),
                );

                if result.is_null() {
                    default.to_string()
                } else {
                    CStr::from_ptr(result).to_string_lossy().into_owned()
                }
            }
        }

        /// Strip a key-value pair from the command line and return the parsed float
        pub fn strip_float(&mut self, key: &str, env_key: Option<&str>, default: f32) -> f32 {
            let c_key = CString::new(key).unwrap();
            let c_env_key = env_key.map(|k| CString::new(k).unwrap());
            let env_key_ptr = c_env_key
                .as_ref()
                .map(|k| k.as_ptr())
                .unwrap_or(ptr::null());

            unsafe {
                libfd_env_sys::fd_env_strip_cmdline_float(
                    &mut self.argc,
                    &mut self.argv.as_mut_ptr(),
                    c_key.as_ptr(),
                    env_key_ptr,
                    default,
                )
            }
        }

        /// Check if the command line contains a specific key and remove it, returning
        /// `true` if the key was found and removed, `false` otherwise.
        pub fn contains_and_strip(&mut self, key: &str) -> bool {
            let c_key = CString::new(key).unwrap();

            unsafe {
                libfd_env_sys::fd_env_strip_cmdline_contains(
                    &mut self.argc,
                    &mut self.argv.as_mut_ptr(),
                    c_key.as_ptr(),
                ) != 0
            }
        }
    }

    impl From<Vec<String>> for CommandLine {
        fn from(args: Vec<String>) -> Self {
            Self::new(args)
        }
    }

    impl From<CommandLine> for Vec<String> {
        fn from(cmdline: CommandLine) -> Self {
            cmdline.to_vec()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cmdline::CommandLine;

    #[test]
    fn test_cmd_strip_ulong() {
        let args = vec![
            "program".to_string(),
            "--count".to_string(),
            "42".to_string(),
            "--other".to_string(),
        ];
        let mut cmdline = CommandLine::new(args);

        let result = cmdline.strip_ulong("--count", None, 0);
        assert_eq!(result, 42);
        assert_eq!(cmdline.len(), 2);
        assert_eq!(cmdline.to_vec(), vec!["program", "--other"]);
    }

    #[test]
    fn test_cmd_strip_cstr() {
        let args = vec![
            "program".to_string(),
            "--name".to_string(),
            "test_name".to_string(),
        ];
        let mut cmdline = CommandLine::new(args);

        let result = cmdline.strip_cstr("--name", None, "default");
        assert_eq!(result, "test_name");
        assert_eq!(cmdline.len(), 1);
    }

    #[test]
    fn test_cmd_contains_and_strip() {
        let args = vec![
            "program".to_string(),
            "--verbose".to_string(),
            "--other".to_string(),
        ];
        let mut cmdline = CommandLine::new(args);

        let found = cmdline.contains_and_strip("--verbose");
        assert!(found);
        assert_eq!(cmdline.len(), 2);
        assert_eq!(cmdline.to_vec(), vec!["program", "--other"]);

        let not_found = cmdline.contains_and_strip("--missing");
        assert!(!not_found);
        assert_eq!(cmdline.len(), 2);
    }

    #[test]
    fn test_cmd_from_vec() {
        let args = vec!["program".to_string(), "--test".to_string()];
        let cmdline: CommandLine = args.clone().into();
        assert_eq!(cmdline.to_vec(), args);
    }

    #[test]
    fn test_cmd_to_vec() {
        let args = vec!["program".to_string(), "--test".to_string()];
        let cmdline = CommandLine::new(args.clone());
        let result: Vec<String> = cmdline.into();
        assert_eq!(result, args);
    }
}
