//! Raw FFI bindings to Firedancer Transaction utilities
//!
//! This crate provides low-level, unsafe bindings to the Firedancer transaction parsing and validation utilities:
//! - High-performance transaction parsing from wire format
//! - Transaction structure validation and access
//! - Support for both legacy and versioned (V0) transactions
//! - Compact-u16 encoding/decoding utilities
//! - Address lookup table support
//!
//! For a safe Rust API, consider using the higher-level `fd_txn` wrapper crate.
//!
//! # Example
//!
//! ```rust,no_run
//! use fd_txn_sys::*;
//! use core::mem::MaybeUninit;
//!
//! unsafe {
//!     let transaction_data = b"..."; // Some serialized transaction bytes
//!     let mut out_buf = vec![0u8; FD_TXN_MAX_SZ as usize];
//!     
//!     let result = fd_txn_parse(
//!         transaction_data.as_ptr(),
//!         transaction_data.len() as u64,
//!         out_buf.as_mut_ptr() as *mut core::ffi::c_void,
//!         core::ptr::null_mut(), // No counters
//!     );
//!     
//!     if result > 0 {
//!         let txn = out_buf.as_ptr() as *const fd_txn_t;
//!         println!("Parsed transaction with {} instructions", (*txn).instr_cnt);
//!     }
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub const FD_TXN_VLEGACY: u8 = 0xFF;
pub const FD_TXN_V0: u8 = 0x00;

/// Wrapper for `fd_txn_footprint`
pub const fn fd_txn_footprint_const(instr_cnt: u64, addr_table_lookup_cnt: u64) -> u64 {
    core::mem::size_of::<fd_txn_t>() as u64
        + instr_cnt * core::mem::size_of::<fd_txn_instr_t>() as u64
        + addr_table_lookup_cnt * core::mem::size_of::<fd_txn_acct_addr_lut_t>() as u64
}

/// Wrapper for `fd_cu16_dec`
pub unsafe fn fd_cu16_decode_safe(buf: &[u8]) -> Option<(u16, usize)> {
    if buf.is_empty() {
        return None;
    }

    let mut result: u16 = 0;
    let consumed = fd_cu16_dec(buf.as_ptr(), buf.len() as u64, &mut result as *mut u16);

    if consumed > 0 {
        Some((result, consumed as usize))
    } else {
        None
    }
}

/// Wrapper for `fd_cu16_enc`
pub fn fd_cu16_encode_safe(val: u16) -> Vec<u8> {
    let mut out = [0u8; 3]; // max compact-u16 size
    let len = unsafe { fd_cu16_enc(val, out.as_mut_ptr()) };
    out[..len as usize].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanity() {
        assert!(FD_TXN_SIGNATURE_SZ > 0);
        assert!(FD_TXN_PUBKEY_SZ > 0);
        assert!(FD_TXN_ACCT_ADDR_SZ > 0);
        assert!(FD_TXN_BLOCKHASH_SZ > 0);
        assert!(FD_TXN_SIG_MAX > 0);
        assert!(FD_TXN_ACCT_ADDR_MAX > 0);
        assert!(FD_TXN_INSTR_MAX > 0);
        assert!(FD_TXN_MAX_SZ > 0);
        assert!(FD_TXN_MTU > 0);
        assert!(FD_TXN_MIN_SERIALIZED_SZ > 0);
    }

    #[test]
    fn test_footprint_calc() {
        let footprint = fd_txn_footprint_const(5, 2);
        let expected = core::mem::size_of::<fd_txn_t>() as u64
            + 5 * core::mem::size_of::<fd_txn_instr_t>() as u64
            + 2 * core::mem::size_of::<fd_txn_acct_addr_lut_t>() as u64;
        assert_eq!(footprint, expected);
    }

    #[test]
    fn test_cu16() {
        let test_values = [0u16, 1, 127, 128, 255, 16383, 16384, 65535];

        for &val in &test_values {
            let encoded = fd_cu16_encode_safe(val);
            assert!(!encoded.is_empty());
            assert!(encoded.len() <= 3);

            let (decoded, consumed) = unsafe { fd_cu16_decode_safe(&encoded) }.unwrap();
            assert_eq!(decoded, val);
            assert_eq!(consumed, encoded.len());
        }
    }

    #[test]
    fn test_cu16_edge_cases() {
        let test_cases = [
            (0x00, 1),   // 0 -> 1 byte
            (0x7F, 1),   // 127 -> 1 byte
            (0x80, 2),   // 128 -> 2 bytes
            (0x3FFF, 2), // 16383 -> 2 bytes
            (0x4000, 3), // 16384 -> 3 bytes
            (0xFFFF, 3), // 65535 -> 3 bytes
        ];

        for &(val, expected_len) in &test_cases {
            let encoded = fd_cu16_encode_safe(val);
            assert_eq!(encoded.len(), expected_len);

            let (decoded, consumed) = unsafe { fd_cu16_decode_safe(&encoded) }.unwrap();
            assert_eq!(decoded, val);
            assert_eq!(consumed, expected_len);
        }
    }

    #[test]
    fn test_cu16_decode_invalid() {
        let invalid_cases = [
            &[0x80][..],             // incomplete 2-byte
            &[0xFF, 0x80][..],       // incomplete 3-byte
            &[0x80, 0x00][..],       // nonminimal
            &[0xFF, 0xFF, 0x00][..], // nonminimal
        ];

        for &invalid in &invalid_cases {
            let result = unsafe { fd_cu16_decode_safe(invalid) };
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_versions() {
        assert_eq!(FD_TXN_VLEGACY, 0xFF);
        assert_eq!(FD_TXN_V0, 0x00);
    }

    #[test]
    fn test_categories() {
        assert_eq!(FD_TXN_ACCT_CAT_WRITABLE_SIGNER, 0x01);
        assert_eq!(FD_TXN_ACCT_CAT_READONLY_SIGNER, 0x02);
        assert_eq!(FD_TXN_ACCT_CAT_WRITABLE_NONSIGNER_IMM, 0x04);
        assert_eq!(FD_TXN_ACCT_CAT_READONLY_NONSIGNER_IMM, 0x08);
        assert_eq!(FD_TXN_ACCT_CAT_WRITABLE_ALT, 0x10);
        assert_eq!(FD_TXN_ACCT_CAT_READONLY_ALT, 0x20);

        assert_eq!(FD_TXN_ACCT_CAT_WRITABLE, 0x15);
        assert_eq!(FD_TXN_ACCT_CAT_READONLY, 0x2A);
        assert_eq!(FD_TXN_ACCT_CAT_SIGNER, 0x03);
        assert_eq!(FD_TXN_ACCT_CAT_NONSIGNER, 0x3C);
        assert_eq!(FD_TXN_ACCT_CAT_IMM, 0x0F);
        assert_eq!(FD_TXN_ACCT_CAT_ALT, 0x30);
        assert_eq!(FD_TXN_ACCT_CAT_ALL, 0x3F);
    }

    #[test]
    fn test_sizes() {
        assert!(core::mem::size_of::<fd_txn_t>() > 0);
        assert!(core::mem::size_of::<fd_txn_instr_t>() > 0);
        assert!(core::mem::size_of::<fd_txn_acct_addr_lut_t>() > 0);
        assert!(core::mem::size_of::<fd_acct_addr_t>() == FD_TXN_ACCT_ADDR_SZ as usize);
        assert!(core::mem::size_of::<fd_txn_parse_counters_t>() > 0);
    }
}
