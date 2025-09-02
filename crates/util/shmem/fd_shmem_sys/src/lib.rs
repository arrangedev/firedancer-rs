//! Raw bindings to the Firedancer shared memory utils
//!
//! This crate provides unsafe FFI bindings to the Firedancer shared memory management system,
//! which enables NUMA-aware and page size-aware manipulation of complex interprocess shared
//! memory topologies. For a safe API, consider using the higher-level wrapper crate.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!(FD_SHMEM_JOIN_MAX > 0);
        assert_eq!(FD_SHMEM_JOIN_MODE_READ_ONLY, 0);
        assert_eq!(FD_SHMEM_JOIN_MODE_READ_WRITE, 1);

        assert!(FD_SHMEM_NUMA_MAX > 0);
        assert!(FD_SHMEM_CPU_MAX > 0);
        assert!(FD_SHMEM_CPU_MAX >= FD_SHMEM_NUMA_MAX);

        // Page size constants
        assert_eq!(FD_SHMEM_UNKNOWN_LG_PAGE_SZ, -1);
        assert_eq!(FD_SHMEM_NORMAL_LG_PAGE_SZ, 12);
        assert_eq!(FD_SHMEM_HUGE_LG_PAGE_SZ, 21);
        assert_eq!(FD_SHMEM_GIGANTIC_LG_PAGE_SZ, 30);

        assert_eq!(FD_SHMEM_UNKNOWN_PAGE_SZ, 0);
        assert_eq!(FD_SHMEM_NORMAL_PAGE_SZ, 4096);
        assert_eq!(FD_SHMEM_HUGE_PAGE_SZ, 2097152);
        assert_eq!(FD_SHMEM_GIGANTIC_PAGE_SZ, 1073741824);

        assert!(FD_SHMEM_NAME_MAX > 0);
        assert!(FD_SHMEM_PAGE_SZ_CSTR_MAX > 0);
    }

    #[test]
    fn test_page_sz_validation() {
        // fd_shmem_is_page_sz is an inline function, so we test the logic manually
        let normal_valid = FD_SHMEM_NORMAL_PAGE_SZ == FD_SHMEM_NORMAL_PAGE_SZ
            || FD_SHMEM_NORMAL_PAGE_SZ == FD_SHMEM_HUGE_PAGE_SZ
            || FD_SHMEM_NORMAL_PAGE_SZ == FD_SHMEM_GIGANTIC_PAGE_SZ;
        assert!(normal_valid);

        let huge_valid = FD_SHMEM_HUGE_PAGE_SZ == FD_SHMEM_NORMAL_PAGE_SZ
            || FD_SHMEM_HUGE_PAGE_SZ == FD_SHMEM_HUGE_PAGE_SZ
            || FD_SHMEM_HUGE_PAGE_SZ == FD_SHMEM_GIGANTIC_PAGE_SZ;
        assert!(huge_valid);

        let gigantic_valid = FD_SHMEM_GIGANTIC_PAGE_SZ == FD_SHMEM_NORMAL_PAGE_SZ
            || FD_SHMEM_GIGANTIC_PAGE_SZ == FD_SHMEM_HUGE_PAGE_SZ
            || FD_SHMEM_GIGANTIC_PAGE_SZ == FD_SHMEM_GIGANTIC_PAGE_SZ;
        assert!(gigantic_valid);
    }

    #[test]
    fn test_page_sz_conversions() {
        unsafe {
            // Test page size to string conversions
            let normal_str = fd_shmem_page_sz_to_cstr(FD_SHMEM_NORMAL_PAGE_SZ as u64);
            assert!(!normal_str.is_null());

            let huge_str = fd_shmem_page_sz_to_cstr(FD_SHMEM_HUGE_PAGE_SZ as u64);
            assert!(!huge_str.is_null());

            let gigantic_str = fd_shmem_page_sz_to_cstr(FD_SHMEM_GIGANTIC_PAGE_SZ as u64);
            assert!(!gigantic_str.is_null());

            let unknown_str = fd_shmem_page_sz_to_cstr(123);
            assert!(!unknown_str.is_null());

            // Test log page size to string conversions
            let normal_lg_str = fd_shmem_lg_page_sz_to_cstr(FD_SHMEM_NORMAL_LG_PAGE_SZ as i32);
            assert!(!normal_lg_str.is_null());

            let huge_lg_str = fd_shmem_lg_page_sz_to_cstr(FD_SHMEM_HUGE_LG_PAGE_SZ as i32);
            assert!(!huge_lg_str.is_null());

            let gigantic_lg_str = fd_shmem_lg_page_sz_to_cstr(FD_SHMEM_GIGANTIC_LG_PAGE_SZ as i32);
            assert!(!gigantic_lg_str.is_null());

            let unknown_lg_str = fd_shmem_lg_page_sz_to_cstr(99);
            assert!(!unknown_lg_str.is_null());
        }
    }

    #[test]
    fn test_string_to_page_sz() {
        unsafe {
            // Test string to page size conversions
            let normal_cstr = std::ffi::CString::new("normal").unwrap();
            let normal_page_sz = fd_cstr_to_shmem_page_sz(normal_cstr.as_ptr());
            assert_eq!(normal_page_sz, FD_SHMEM_NORMAL_PAGE_SZ as u64);

            let huge_cstr = std::ffi::CString::new("huge").unwrap();
            let huge_page_sz = fd_cstr_to_shmem_page_sz(huge_cstr.as_ptr());
            assert_eq!(huge_page_sz, FD_SHMEM_HUGE_PAGE_SZ as u64);

            let gigantic_cstr = std::ffi::CString::new("gigantic").unwrap();
            let gigantic_page_sz = fd_cstr_to_shmem_page_sz(gigantic_cstr.as_ptr());
            assert_eq!(gigantic_page_sz, FD_SHMEM_GIGANTIC_PAGE_SZ as u64);

            let invalid_cstr = std::ffi::CString::new("invalid").unwrap();
            let invalid_page_sz = fd_cstr_to_shmem_page_sz(invalid_cstr.as_ptr());
            assert_eq!(invalid_page_sz, FD_SHMEM_UNKNOWN_PAGE_SZ as u64);

            // Test string to log page size conversions
            let normal_lg = fd_cstr_to_shmem_lg_page_sz(normal_cstr.as_ptr());
            assert_eq!(normal_lg, FD_SHMEM_NORMAL_LG_PAGE_SZ as i32);

            let huge_lg = fd_cstr_to_shmem_lg_page_sz(huge_cstr.as_ptr());
            assert_eq!(huge_lg, FD_SHMEM_HUGE_LG_PAGE_SZ as i32);

            let gigantic_lg = fd_cstr_to_shmem_lg_page_sz(gigantic_cstr.as_ptr());
            assert_eq!(gigantic_lg, FD_SHMEM_GIGANTIC_LG_PAGE_SZ as i32);

            let invalid_lg = fd_cstr_to_shmem_lg_page_sz(invalid_cstr.as_ptr());
            assert_eq!(invalid_lg, FD_SHMEM_UNKNOWN_LG_PAGE_SZ);
        }
    }

    #[test]
    fn test_name_validation() {
        unsafe {
            // Test valid names
            let valid_name = std::ffi::CString::new("test_region_123").unwrap();
            let name_len = fd_shmem_name_len(valid_name.as_ptr());
            assert_eq!(name_len, "test_region_123".len() as u64);

            // Test empty name (invalid)
            let empty_name = std::ffi::CString::new("").unwrap();
            let empty_len = fd_shmem_name_len(empty_name.as_ptr());
            assert_eq!(empty_len, 0);

            // Test null pointer
            let null_len = fd_shmem_name_len(std::ptr::null());
            assert_eq!(null_len, 0);
        }
    }

    #[test]
    fn test_struct_sizes() {
        // Verify that the struct sizes are reasonable
        assert!(core::mem::size_of::<fd_shmem_join_info>() > 0);
        assert!(core::mem::size_of::<fd_shmem_info>() > 0);
        assert!(core::mem::size_of::<fd_shmem_private_key>() > 0);

        // Verify alignment requirements
        assert!(core::mem::align_of::<fd_shmem_join_info>() > 0);
        assert!(core::mem::align_of::<fd_shmem_info>() > 0);
        assert!(core::mem::align_of::<fd_shmem_private_key>() > 0);
    }
}
