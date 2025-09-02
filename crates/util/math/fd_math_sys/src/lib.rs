//! Raw bindings to Firedancer math utils.
//!
//! - Square roots (fd_sqrt.h)
//! - Statistics (fd_stat.h)
//! - Fixed-point arithmetic (fd_fxp.h)
//!
//! For a safe API, consider using the higher-level wrapper crate `libfd-math`.
//!
//! Note: Many functions in the math module are `static inline` and don't appear
//! in the generated bindings. The `libfd-math` crate provides faithful substitutes
//! for these functions.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_median() {
        unsafe {
            let mut values = [1u32, 3, 2, 5, 4];
            let median = fd_stat_median_uint(values.as_mut_ptr(), 5);
            assert_eq!(median, 3); // median of [1,2,3,4,5] is 3
        }
    }

    #[test]
    fn test_stat_filter() {
        unsafe {
            let input = [1u32, 10, 2, 20, 3];
            let mut output = [0u32; 5];
            let count = fd_stat_filter_uint(output.as_mut_ptr(), input.as_ptr(), 5, 5);
            // should filter values <= 5, so we get [1, 2, 3]
            assert_eq!(count, 3);
            assert_eq!(output[0], 1);
            assert_eq!(output[1], 2);
            assert_eq!(output[2], 3);
        }
    }

    #[test]
    fn test_stat_median_float() {
        unsafe {
            let mut values = [1.0f32, 3.0, 2.0, 5.0, 4.0];
            let median = fd_stat_median_float(values.as_mut_ptr(), 5);
            assert_eq!(median, 3.0); // median of [1.0,2.0,3.0,4.0,5.0] is 3.0
        }
    }
}
