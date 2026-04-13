//! Low-level FFI bindings to Firedancer's fd_shred module.
//!
//! This crate provides raw, unsafe bindings to the Firedancer shred API,
//! including shred parsing, validation, and deshredding operations.
//!
//! For safe, idiomatic Rust wrappers, see the `fd_shred` crate.
//!
//! # Safety
//!
//! All functions in this crate are unsafe and require careful handling of:
//! - Memory management and lifetime guarantees
//! - Proper buffer size requirements (shreds are 1228 bytes max)
//! - Thread safety considerations
//! - Proper initialization of bmtree memory for merkle operations
//!
//! # Shred Types
//!
//! The main shred types available:
//! - `FD_SHRED_TYPE_LEGACY_DATA`: Legacy data shred
//! - `FD_SHRED_TYPE_LEGACY_CODE`: Legacy coding shred  
//! - `FD_SHRED_TYPE_MERKLE_DATA`: Merkle data shred
//! - `FD_SHRED_TYPE_MERKLE_CODE`: Merkle coding shred
//! - `FD_SHRED_TYPE_MERKLE_DATA_CHAINED`: Chained merkle data shred
//! - `FD_SHRED_TYPE_MERKLE_CODE_CHAINED`: Chained merkle coding shred
//! - `FD_SHRED_TYPE_MERKLE_DATA_CHAINED_RESIGNED`: Resigned chained merkle data shred
//! - `FD_SHRED_TYPE_MERKLE_CODE_CHAINED_RESIGNED`: Resigned chained merkle coding shred
//!
//! # Main Operations
//!
//! The main operations available:
//! - `fd_shred_parse`: Parse and validate a shred from raw bytes
//! - `fd_shred_type`: Extract the shred type from variant field
//! - `fd_shred_variant`: Create variant field from type and merkle count
//! - `fd_shred_sz`: Get the size of a shred
//! - `fd_shred_payload_sz`: Get the payload size of a shred
//! - `fd_shred_merkle_root`: Reconstruct merkle root from shred
//! - `fd_shred_data_payload`: Get pointer to data shred payload
//! - `fd_shred_code_payload`: Get pointer to coding shred payload
//!
//! # Example
//!
//! ```rust,no_run
//! use fd_shred_sys::*;
//! use std::mem::MaybeUninit;
//!
//! unsafe {
//!     // Parse a shred from raw bytes
//!     let shred_bytes: [u8; FD_SHRED_MAX_SZ as usize] = [0; FD_SHRED_MAX_SZ as usize];
//!     let shred = fd_shred_parse(shred_bytes.as_ptr(), shred_bytes.len() as u64);
//!     
//!     if !shred.is_null() {
//!         // Get shred type
//!         let shred_type = fd_shred_type((*shred).variant);
//!         
//!         // Check if it's a data shred
//!         if fd_shred_is_data(shred_type as u64) != 0 {
//!             // Get payload
//!             let payload = fd_shred_data_payload(shred);
//!             let payload_size = fd_shred_payload_sz(shred);
//!             println!("Data shred with payload size: {}", payload_size);
//!         }
//!     }
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// Constants that bindgen doesn't pick up due to their uchar cast format
pub const FD_SHRED_TYPE_LEGACY_DATA: u8 = 0xA0;
pub const FD_SHRED_TYPE_LEGACY_CODE: u8 = 0x50;
pub const FD_SHRED_TYPE_MERKLE_DATA: u8 = 0x80;
pub const FD_SHRED_TYPE_MERKLE_CODE: u8 = 0x40;
pub const FD_SHRED_TYPE_MERKLE_DATA_CHAINED: u8 = 0x90;
pub const FD_SHRED_TYPE_MERKLE_CODE_CHAINED: u8 = 0x60;
pub const FD_SHRED_TYPE_MERKLE_DATA_CHAINED_RESIGNED: u8 = 0xB0;
pub const FD_SHRED_TYPE_MERKLE_CODE_CHAINED_RESIGNED: u8 = 0x70;

pub const FD_SHRED_DATA_FLAG_SLOT_COMPLETE: u8 = 0x80;
pub const FD_SHRED_DATA_FLAG_DATA_COMPLETE: u8 = 0x40;
pub const FD_SHRED_DATA_REF_TICK_MASK: u8 = 0x3f;

pub const FD_SHRED_TYPEMASK_DATA: u8 = FD_SHRED_TYPE_MERKLE_DATA;
pub const FD_SHRED_TYPEMASK_CODE: u8 = FD_SHRED_TYPE_MERKLE_CODE;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shred_type_functions() {
        unsafe {
            // Test type extraction
            let legacy_data_variant = fd_shred_variant(FD_SHRED_TYPE_LEGACY_DATA, 0);
            assert_eq!(
                fd_shred_type(legacy_data_variant),
                FD_SHRED_TYPE_LEGACY_DATA
            );

            let legacy_code_variant = fd_shred_variant(FD_SHRED_TYPE_LEGACY_CODE, 0);
            assert_eq!(
                fd_shred_type(legacy_code_variant),
                FD_SHRED_TYPE_LEGACY_CODE
            );

            // Test type checking functions
            assert_ne!(fd_shred_is_data(FD_SHRED_TYPE_LEGACY_DATA as u64), 0);
            assert_eq!(fd_shred_is_code(FD_SHRED_TYPE_LEGACY_DATA as u64), 0);

            assert_ne!(fd_shred_is_code(FD_SHRED_TYPE_LEGACY_CODE as u64), 0);
            assert_eq!(fd_shred_is_data(FD_SHRED_TYPE_LEGACY_CODE as u64), 0);
        }
    }

    #[test]
    fn test_shred_header_sz() {
        unsafe {
            let data_variant = fd_shred_variant(FD_SHRED_TYPE_LEGACY_DATA, 0);
            let code_variant = fd_shred_variant(FD_SHRED_TYPE_LEGACY_CODE, 0);

            assert_eq!(
                fd_shred_header_sz(data_variant),
                FD_SHRED_DATA_HEADER_SZ as u64
            );
            assert_eq!(
                fd_shred_header_sz(code_variant),
                FD_SHRED_CODE_HEADER_SZ as u64
            );
        }
    }

    #[test]
    fn test_merkle_functions() {
        unsafe {
            // Test merkle count extraction
            let merkle_variant = fd_shred_variant(FD_SHRED_TYPE_MERKLE_DATA, 5);
            assert_eq!(fd_shred_merkle_cnt(merkle_variant), 5);

            let legacy_variant = fd_shred_variant(FD_SHRED_TYPE_LEGACY_DATA, 0);
            assert_eq!(fd_shred_merkle_cnt(legacy_variant), 0);

            // Test merkle size calculation
            assert_eq!(
                fd_shred_merkle_sz(merkle_variant),
                (5 * FD_SHRED_MERKLE_NODE_SZ) as u64
            );
            assert_eq!(fd_shred_merkle_sz(legacy_variant), 0);
        }
    }

    #[test]
    fn test_shred_type_checking() {
        unsafe {
            // Test chained detection
            assert_ne!(
                fd_shred_is_chained(FD_SHRED_TYPE_MERKLE_DATA_CHAINED as u64),
                0
            );
            assert_ne!(
                fd_shred_is_chained(FD_SHRED_TYPE_MERKLE_CODE_CHAINED as u64),
                0
            );
            assert_eq!(fd_shred_is_chained(FD_SHRED_TYPE_MERKLE_DATA as u64), 0);

            // Test resigned detection
            assert_ne!(
                fd_shred_is_resigned(FD_SHRED_TYPE_MERKLE_DATA_CHAINED_RESIGNED as u64),
                0
            );
            assert_ne!(
                fd_shred_is_resigned(FD_SHRED_TYPE_MERKLE_CODE_CHAINED_RESIGNED as u64),
                0
            );
            assert_eq!(
                fd_shred_is_resigned(FD_SHRED_TYPE_MERKLE_DATA_CHAINED as u64),
                0
            );
        }
    }

    #[test]
    fn test_type_swapping() {
        unsafe {
            // Test type swapping (data <-> code)
            assert_eq!(
                fd_shred_swap_type(FD_SHRED_TYPE_LEGACY_DATA as u64),
                FD_SHRED_TYPE_LEGACY_CODE
            );
            assert_eq!(
                fd_shred_swap_type(FD_SHRED_TYPE_LEGACY_CODE as u64),
                FD_SHRED_TYPE_LEGACY_DATA
            );
            assert_eq!(
                fd_shred_swap_type(FD_SHRED_TYPE_MERKLE_DATA as u64),
                FD_SHRED_TYPE_MERKLE_CODE
            );
            assert_eq!(
                fd_shred_swap_type(FD_SHRED_TYPE_MERKLE_CODE as u64),
                FD_SHRED_TYPE_MERKLE_DATA
            );
        }
    }
}
