//! Raw FFI bindings to Firedancer bit manipulation utilities
//!
//! This crate provides low-level, unsafe bindings to the Firedancer bit manipulation utilities:
//! - Bit finding operations (MSB, LSB)
//! - Byte order conversion (endian swapping)
//! - Integer square root and cube root functions
//! - Floating point utilities
//! - Saturating arithmetic
//! - Wide integer arithmetic (128-bit on platforms without native support)
//!
//! For a safe Rust API, consider using the higher-level wrapper crate.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt_functions() {
        // Test the square root functions that are actually implemented
        unsafe {
            assert_eq!(fd_ulong_floor_sqrt(100), 10);
            assert_eq!(fd_ulong_ceil_sqrt(99), 10);
            assert_eq!(fd_ulong_round_sqrt(100), 10);

            assert_eq!(fd_ulong_floor_sqrt(1024), 32);
            assert_eq!(fd_ulong_ceil_sqrt(1000), 32);
        }
    }

    #[test]
    fn test_cube_root_functions() {
        // Test the cube root functions
        unsafe {
            assert_eq!(fd_ulong_floor_cbrt(8), 2);
            assert_eq!(fd_ulong_ceil_cbrt(7), 2);
            assert_eq!(fd_ulong_round_cbrt(8), 2);

            assert_eq!(fd_ulong_floor_cbrt(1000), 10);
        }
    }

    #[test]
    fn test_bindings_exist() {
        // Basic smoke test to ensure bindings are generated and compile
        // We test the sqrt functions since they're the main non-inline functions
        unsafe {
            let _sqrt = fd_ulong_floor_sqrt(42);
            let _cbrt = fd_ulong_floor_cbrt(27);
        }
    }
}
