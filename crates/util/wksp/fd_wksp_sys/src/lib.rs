//! Low-level FFI bindings to Firedancer's fd_wksp module.
//!
//! This crate provides raw, unsafe bindings to the Firedancer workspace (wksp) API.
//! For safe, idiomatic Rust wrappers, see the `fd_wksp` crate.
//!
//! # Safety
//!
//! All functions in this crate are unsafe and require careful handling of:
//! - Memory management and lifetime guarantees
//! - Thread safety and concurrency
//! - Proper initialization and cleanup
//!
//! # Example
//!
//! ```rust,no_run
//! use fd_wksp_sys::*;
//! use std::ffi::CString;
//! use std::ptr;
//!
//! unsafe {
//!     // create a new anonymous wksp
//!     let name = CString::new("test-wksp").unwrap();
//!     let wksp = fd_wksp_new_anon(
//!         name.as_ptr(),
//!         4096,  // page_sz
//!         1,     // sub_cnt
//!         &256,  // sub_page_cnt
//!         &0,    // sub_cpu_idx
//!         42,    // seed
//!         0,     // opt_part_max (0 = auto)
//!     );
//!
//!     if !wksp.is_null() {
//!         // allocate
//!         let gaddr = fd_wksp_alloc(wksp, 0, 1024, 1);
//!         if gaddr != 0 {
//!             // convert to localaddr
//!             let laddr = fd_wksp_laddr(wksp, gaddr);
//!             // devs do something...
//!             fd_wksp_free(wksp, gaddr);
//!         }
//!         
//!         // cleanup
//!         fd_wksp_delete_anon(wksp);
//!     }
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_wksp_constants() {
        assert_eq!(FD_WKSP_SUCCESS, 0);
        assert!(FD_WKSP_ERR_INVAL < 0);
        assert!(FD_WKSP_ERR_FAIL < 0);
        assert!(FD_WKSP_ERR_CORRUPT < 0);
        assert!(FD_WKSP_ALIGN > 0);
        assert!(FD_WKSP_ALIGN_DEFAULT >= 16);
    }

    #[test]
    fn test_wksp_footprint_calculation() {
        unsafe {
            let align = fd_wksp_align();
            assert_eq!(align, FD_WKSP_ALIGN as u64);

            let footprint = fd_wksp_footprint(100, 1024 * 1024);
            assert!(footprint > 0);
        }
    }

    #[test]
    fn test_wksp_estimation_functions() {
        unsafe {
            let footprint = 16 * 1024 * 1024; // 16MB
            let sz_typical = 64 * 1024; // 64KB

            let part_max_est = fd_wksp_part_max_est(footprint, sz_typical);
            assert!(part_max_est > 0);

            let data_max_est = fd_wksp_data_max_est(footprint, part_max_est);
            assert!(data_max_est > 0);
        }
    }

    #[test]
    fn test_wksp_strerror() {
        unsafe {
            let success_str = fd_wksp_strerror(FD_WKSP_SUCCESS as i32);
            assert!(!success_str.is_null());

            let inval_str = fd_wksp_strerror(FD_WKSP_ERR_INVAL);
            assert!(!inval_str.is_null());
        }
    }

    #[test]
    #[ignore]
    fn test_anonymous_workspace_lifecycle() {
        unsafe {
            let name = CString::new("test-anon-wksp").unwrap();
            let page_cnt = 64;
            let cpu_idx = 0;

            let wksp = fd_wksp_new_anon(
                name.as_ptr(),
                4096,      // page_sz
                1,         // sub_cnt
                &page_cnt, // sub_page_cnt
                &cpu_idx,  // sub_cpu_idx
                42,        // seed
                0,         // opt_part_max (0 = auto)
            );

            if !wksp.is_null() {
                let wksp_name = fd_wksp_name(wksp);
                assert!(!wksp_name.is_null());

                let seed = fd_wksp_seed(wksp);
                assert_eq!(seed, 42);

                let part_max = fd_wksp_part_max(wksp);
                assert!(part_max > 0);

                let data_max = fd_wksp_data_max(wksp);
                assert!(data_max > 0);

                let gaddr = fd_wksp_alloc(wksp, 0, 1024, 1);
                if gaddr != 0 {
                    let laddr = fd_wksp_laddr(wksp, gaddr);
                    assert!(!laddr.is_null());

                    let gaddr_back = fd_wksp_gaddr(wksp, laddr);
                    assert_eq!(gaddr, gaddr_back);

                    let tag = fd_wksp_tag(wksp, gaddr);
                    assert_eq!(tag, 1);

                    fd_wksp_free(wksp, gaddr);
                }

                fd_wksp_delete_anon(wksp);
            }
        }
    }
}
