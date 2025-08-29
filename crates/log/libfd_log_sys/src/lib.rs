//! Raw bindings to the Firedancer log utils
//!
//! For a safe API, consider using the higher-level wrapper crate.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bindings_exist() {
        unsafe {
            let _wallclock = fd_log_wallclock_host(std::ptr::null());
        }
    }
}
