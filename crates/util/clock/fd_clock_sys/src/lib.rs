#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_and_footprint() {
        unsafe {
            let align = fd_clock_align();
            let footprint = fd_clock_footprint();

            assert_eq!(align, FD_CLOCK_ALIGN as u64);
            assert_eq!(footprint, FD_CLOCK_FOOTPRINT as u64);
            assert!(align > 0);
            assert!(align.is_power_of_two());
            assert!(footprint > 0);
        }
    }

    #[test]
    fn test_strerror() {
        unsafe {
            let s = fd_clock_strerror(FD_CLOCK_SUCCESS as i32);
            assert!(!s.is_null());

            let s = fd_clock_strerror(FD_CLOCK_ERR_X);
            assert!(!s.is_null());

            let s = fd_clock_strerror(FD_CLOCK_ERR_Y);
            assert!(!s.is_null());
        }
    }
}
