//! Raw FFI bindings for `/util/wksp`

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use core::ffi::CStr;

    #[test]
    fn test_wksp_footprint_calc() {
        unsafe {
            let align = fd_wksp_align();
            assert_eq!(align, FD_WKSP_ALIGN as u64);

            let footprint = fd_wksp_footprint(100, 1024 * 1024);
            assert!(footprint > 0);
        }
    }

    #[test]
    fn test_wksp_est_fns() {
        unsafe {
            let footprint = 16 * 1024 * 1024;
            let sz_typical = 64 * 1024;

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
    fn test_anon_wksp_lifecycle() {
        unsafe {
            let name = CStr::from_bytes_with_nul(b"test-anon-wksp\0").unwrap();
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
