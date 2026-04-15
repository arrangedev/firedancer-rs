//! Safe Rust API for Firedancer Transaction utilities
//!
//! This crate provides safe abstractions over the raw FFI bindings in `fd_txn_sys`.
//! The transaction implementation supports both legacy and versioned (V0) transaction formats
//! as used by Solana, with high-performance parsing and validation.
//!
//! ## Features
//!
//! - **High performance**: Optimized C implementation with Rust safety
//! - **Transaction parsing**: Parse transactions from wire format
//! - **Transaction validation**: Validate transaction structure and constraints
//! - **Address lookup tables**: Support for V0 transactions with ALTs
//! - **Instruction access**: Safe access to transaction instructions and data
//! - **Account categorization**: Iterate over accounts by type (signer, writable, etc.)
//! - **Compact-u16 encoding**: Utilities for Solana's compact integer format
//!
//! ## Usage
//!
//! ### Transaction Parsing
//!
//! ```rust,no_run
//! use fd_txn::{Transaction, ParseError};
//!
//! let transaction_bytes = b"...";
//! match Transaction::parse(transaction_bytes) {
//!     Ok(txn) => {
//!         println!("Transaction has {} instructions", txn.instruction_count());
//!         println!("Fee payer: {:?}", txn.fee_payer());
//!     }
//!     Err(e) => println!("Parse error: {}", e),
//! }
//! ```
//!
//! ### Instructions
//!
//! ```rust,no_run
//! # use fd_txn::{Transaction, ParseError};
//! # let transaction_bytes = b"";
//! # let txn = Transaction::parse(transaction_bytes).unwrap();
//! for (i, instr) in txn.instructions().enumerate() {
//!     println!("Instruction {}: program_id={}, accounts={}, data_len={}",
//!              i, instr.program_id(), instr.account_count(), instr.data().len());
//! }
//! ```
//!
//! ### Accounts Iterator
//!
//! ```rust,no_run
//! # use fd_txn::{Transaction, AccountCategory};
//! # let transaction_bytes = b"";
//! # let txn = Transaction::parse(transaction_bytes).unwrap();
//! // iterate over all writable accounts
//! for (idx, addr) in txn.accounts_by_category(AccountCategory::WRITABLE) {
//!     println!("Writable account {}: {:?}", idx, addr);
//! }
//! ```

pub mod builder;

pub use builder::{
    AccountMeta, BuildError, InstructionBuf, OwnedInstruction, TransactionBuilder,
    UnsignedTransaction,
};

use core::fmt;
use core::slice;
use fd_ed25519::Pubkey;
use fd_txn_sys as sys;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The transaction data is too short to be valid
    TooShort,
    /// The transaction data is too long (exceeds MTU)
    TooLong,
    /// The transaction has an invalid structure
    InvalidStructure,
    /// The transaction has invalid signatures
    InvalidSignatures,
    /// The transaction has invalid account references
    InvalidAccounts,
    /// The transaction has invalid instructions
    InvalidInstructions,
    /// The transaction version is not supported
    UnsupportedVersion,
    /// The transaction has invalid compact-u16 encoding
    InvalidCompactU16,
    /// Buffer too small for the operation
    BufferTooSmall,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::TooShort => write!(f, "Transaction data too short"),
            ParseError::TooLong => write!(f, "Transaction data too long"),
            ParseError::InvalidStructure => write!(f, "Invalid transaction structure"),
            ParseError::InvalidSignatures => write!(f, "Invalid signatures"),
            ParseError::InvalidAccounts => write!(f, "Invalid account references"),
            ParseError::InvalidInstructions => write!(f, "Invalid instructions"),
            ParseError::UnsupportedVersion => write!(f, "Unsupported transaction version"),
            ParseError::InvalidCompactU16 => write!(f, "Invalid compact-u16 encoding"),
            ParseError::BufferTooSmall => write!(f, "Buffer too small"),
        }
    }
}

impl core::error::Error for ParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionVersion {
    /// Legacy transaction
    Legacy,
    /// V0 transaction (compiled format + address lookup tables)
    V0,
}

impl TransactionVersion {
    pub fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            sys::FD_TXN_VLEGACY => Some(TransactionVersion::Legacy),
            sys::FD_TXN_V0 => Some(TransactionVersion::V0),
            _ => None,
        }
    }

    pub fn to_raw(self) -> u8 {
        match self {
            TransactionVersion::Legacy => sys::FD_TXN_VLEGACY,
            TransactionVersion::V0 => sys::FD_TXN_V0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AccountCategory(pub i32);

impl AccountCategory {
    pub const WRITABLE_SIGNER: Self = Self(sys::FD_TXN_ACCT_CAT_WRITABLE_SIGNER as i32);
    pub const READONLY_SIGNER: Self = Self(sys::FD_TXN_ACCT_CAT_READONLY_SIGNER as i32);
    pub const WRITABLE_NONSIGNER_IMM: Self =
        Self(sys::FD_TXN_ACCT_CAT_WRITABLE_NONSIGNER_IMM as i32);
    pub const READONLY_NONSIGNER_IMM: Self =
        Self(sys::FD_TXN_ACCT_CAT_READONLY_NONSIGNER_IMM as i32);
    pub const WRITABLE_ALT: Self = Self(sys::FD_TXN_ACCT_CAT_WRITABLE_ALT as i32);
    pub const READONLY_ALT: Self = Self(sys::FD_TXN_ACCT_CAT_READONLY_ALT as i32);

    pub const WRITABLE: Self = Self(sys::FD_TXN_ACCT_CAT_WRITABLE as i32);
    pub const READONLY: Self = Self(sys::FD_TXN_ACCT_CAT_READONLY as i32);
    pub const SIGNER: Self = Self(sys::FD_TXN_ACCT_CAT_SIGNER as i32);
    pub const NONSIGNER: Self = Self(sys::FD_TXN_ACCT_CAT_NONSIGNER as i32);
    pub const IMMEDIATE: Self = Self(sys::FD_TXN_ACCT_CAT_IMM as i32);
    pub const ADDRESS_LOOKUP: Self = Self(sys::FD_TXN_ACCT_CAT_ALT as i32);
    pub const ALL: Self = Self(sys::FD_TXN_ACCT_CAT_ALL as i32);
}

/// A parsed Solana transaction.
///
/// Transactions on Solana can be max 1232 bytes (MTU packet size) for both
/// Legacy and V0 transactions.
///
/// You can find a reference in the Solana docs at: https://solana.com/docs/core/transactions
/// but to repeat here:
///
/// |---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
/// | -----------Version----------- | -------------Number of Signatures------------ |
/// | ---------Signature Off--------- | ----------------Message Off---------------- |
/// | -----------Readonly Signed Count----------- | ----Readonly Unsigned Count---- |
/// | ---------Account Addresses--------- | ----------------Blockhash-------------- |
/// | ---------Instructions--------- | -----------Address Lookup Tables------------ |
/// |---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
#[derive(Debug)]
pub struct Transaction {
    /// raw txn data (need for lifetime)
    _data: Vec<u8>,
    /// parsed txn structure
    _txn_buf: Vec<u8>,
    /// pointer to the parsed txn (txn_buf)
    txn_ptr: *const sys::fd_txn_t,
}

unsafe impl Send for Transaction {}
unsafe impl Sync for Transaction {}

impl Transaction {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.is_empty() {
            return Err(ParseError::TooShort);
        }

        if data.len() > sys::FD_TXN_MTU as usize {
            return Err(ParseError::TooLong);
        }

        let mut txn_buf = vec![0u8; sys::FD_TXN_MAX_SZ as usize];
        let data_copy = data.to_vec();

        let result = unsafe {
            sys::fd_txn_parse_core(
                data_copy.as_ptr(),
                data_copy.len() as u64,
                txn_buf.as_mut_ptr() as *mut core::ffi::c_void,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            )
        };

        if result == 0 {
            return Err(ParseError::InvalidStructure);
        }

        let txn_ptr = txn_buf.as_ptr() as *const sys::fd_txn_t;

        Ok(Transaction {
            _data: data_copy,
            _txn_buf: txn_buf,
            txn_ptr,
        })
    }

    /// legacy or v0
    pub fn version(&self) -> TransactionVersion {
        let raw_version = unsafe { (*self.txn_ptr).transaction_version };
        TransactionVersion::from_raw(raw_version).unwrap_or(TransactionVersion::Legacy)
    }

    /// number of signatures in the txn
    pub fn signature_count(&self) -> usize {
        unsafe { (*self.txn_ptr).signature_cnt as usize }
    }

    /// number of instructions in the txn
    pub fn instruction_count(&self) -> usize {
        unsafe { (*self.txn_ptr).instr_cnt as usize }
    }

    /// number of account addresses in the txn (excluding ALT accounts)
    pub fn account_count(&self) -> usize {
        unsafe { (*self.txn_ptr).acct_addr_cnt as usize }
    }

    /// total number of account addresses (including ALT accounts)
    pub fn total_account_count(&self) -> usize {
        unsafe {
            (*self.txn_ptr).acct_addr_cnt as usize + (*self.txn_ptr).addr_table_adtl_cnt as usize
        }
    }

    /// account addresses (excluding ALT accounts)
    pub fn accounts(&self) -> &[Pubkey] {
        unsafe {
            let addrs = sys::fd_txn_get_acct_addrs(
                self.txn_ptr,
                self._data.as_ptr() as *const core::ffi::c_void,
            );
            let count = (*self.txn_ptr).acct_addr_cnt as usize;
            slice::from_raw_parts(addrs as *const Pubkey, count)
        }
    }

    /// fee payer account address (always the first account)
    pub fn fee_payer(&self) -> Pubkey {
        self.accounts()[0]
    }

    pub fn recent_blockhash(&self) -> &[u8; 32] {
        unsafe {
            let hash_ptr = sys::fd_txn_get_recent_blockhash(
                self.txn_ptr,
                self._data.as_ptr() as *const core::ffi::c_void,
            );
            &*(hash_ptr as *const [u8; 32])
        }
    }

    /// check if an account at the given index is writable
    pub fn is_account_writable(&self, index: u16) -> bool {
        unsafe { sys::fd_txn_is_writable(self.txn_ptr, index) != 0 }
    }

    /// check if an account at the given index is a signer
    pub fn is_account_signer(&self, index: u16) -> bool {
        unsafe { sys::fd_txn_is_signer(self.txn_ptr, index as i32) != 0 }
    }

    /// number of accounts matching the given category
    pub fn account_count_by_category(&self, category: AccountCategory) -> usize {
        unsafe { sys::fd_txn_account_cnt(self.txn_ptr, category.0) as usize }
    }

    /// iterate over account indices that match the given category
    pub fn accounts_by_category(&self, category: AccountCategory) -> AccountIter<'_> {
        let iter = unsafe { sys::fd_txn_acct_iter_init(self.txn_ptr, category.0) };
        AccountIter {
            current: iter,
            accounts: self.accounts(),
        }
    }

    /// all instructions in the txn
    pub fn instructions(&self) -> InstructionIter<'_> {
        InstructionIter {
            txn: self,
            index: 0,
        }
    }

    /// get a specific instruction by index
    pub fn instruction(&self, index: usize) -> Option<Instruction<'_>> {
        if index >= self.instruction_count() {
            return None;
        }

        unsafe {
            let instr_ptr = (*self.txn_ptr).instr.as_ptr().add(index);
            Some(Instruction {
                txn: self,
                instr_ptr,
            })
        }
    }

    /// check if this is a simple vote transaction
    pub fn is_simple_vote_transaction(&self) -> bool {
        unsafe {
            sys::fd_txn_is_simple_vote_transaction(
                self.txn_ptr,
                self._data.as_ptr() as *const core::ffi::c_void,
            ) != 0
        }
    }

    /// get raw pointer to the parsed transaction
    ///
    /// # Safety: Caller must ensure the returned pointer is not used after the
    /// Transaction is dropped
    pub unsafe fn as_raw_ptr(&self) -> *const sys::fd_txn_t {
        self.txn_ptr
    }

    /// get the raw transaction data
    pub fn raw_data(&self) -> &[u8] {
        &self._data
    }
}

pub struct AccountIter<'a> {
    current: sys::fd_txn_acct_iter_t,
    accounts: &'a [Pubkey],
}

impl<'a> Iterator for AccountIter<'a> {
    type Item = (usize, Pubkey);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == unsafe { sys::fd_txn_acct_iter_end() } {
            return None;
        }

        let idx = unsafe { sys::fd_txn_acct_iter_idx(self.current) } as usize;
        let addr = self.accounts.get(idx).copied()?;

        self.current = unsafe { sys::fd_txn_acct_iter_next(self.current) };

        Some((idx, addr))
    }
}

pub struct Instruction<'a> {
    txn: &'a Transaction,
    instr_ptr: *const sys::fd_txn_instr_t,
}

impl<'a> Instruction<'a> {
    /// the program_id_index for this ixn
    pub fn program_id(&self) -> u8 {
        unsafe { (*self.instr_ptr).program_id }
    }

    /// the program_id account address
    pub fn program_id_address(&self) -> Pubkey {
        let idx = self.program_id() as usize;
        self.txn.accounts()[idx]
    }

    /// number of accounts this instruction references
    pub fn account_count(&self) -> usize {
        unsafe { (*self.instr_ptr).acct_cnt as usize }
    }

    /// account indices for this instruction
    pub fn account_indices(&self) -> &[u8] {
        unsafe {
            let indices_ptr = sys::fd_txn_get_instr_accts(
                self.instr_ptr,
                self.txn._data.as_ptr() as *const core::ffi::c_void,
            );
            slice::from_raw_parts(indices_ptr, self.account_count())
        }
    }

    /// account addresses for this instruction
    pub fn account_addresses(&self) -> Vec<Pubkey> {
        self.account_indices()
            .iter()
            .map(|&idx| self.txn.accounts()[idx as usize])
            .collect()
    }

    pub fn data(&self) -> &[u8] {
        unsafe {
            let data_ptr = sys::fd_txn_get_instr_data(
                self.instr_ptr,
                self.txn._data.as_ptr() as *const core::ffi::c_void,
            );
            let data_size = (*self.instr_ptr).data_sz as usize;
            slice::from_raw_parts(data_ptr, data_size)
        }
    }
}

pub struct InstructionIter<'a> {
    txn: &'a Transaction,
    index: usize,
}

impl<'a> Iterator for InstructionIter<'a> {
    type Item = Instruction<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.txn.instruction_count() {
            return None;
        }

        let instr = self.txn.instruction(self.index)?;
        self.index += 1;
        Some(instr)
    }
}

/// Helpers for the `compact-u16` encoding format used in versioned transactions
///
/// The format is documented at
/// https://docs.solana.com/developing/programming-model/transactions#compact-u16-format
/// but briefly:
///     If the 16 bit number has (big endian bits) ponmlkji hgfedcba
///     [  0x00,    0x80)   (implies [h..p] = 0) ->  0gfedcba                      (1 byte )
///     [  0x80,  0x4000)   (implies [o..p] = 0) ->  1gfedcba 0nmlkjih             (2 bytes)
///     [0x4000, 0x10000)                        ->  1gfedcba 1nmlkjih 000000po    (3 bytes)
/// Numbers must be encoded with the minimal number of bytes possible.
pub mod compact_u16 {
    use super::*;

    /// encode a u16 value as a compact-u16
    pub fn encode(value: u16) -> Vec<u8> {
        sys::fd_cu16_encode_safe(value)
    }

    /// decode a compact-u16 value from bytes
    ///
    /// Returns: (decoded_value, bytes_consumed)
    pub fn decode(data: &[u8]) -> Result<(u16, usize), ParseError> {
        unsafe { sys::fd_cu16_decode_safe(data).ok_or(ParseError::InvalidCompactU16) }
    }

    /// get the size of a compact-u16 encoding without decoding
    pub fn encoded_size(data: &[u8]) -> Option<usize> {
        decode(data).ok().map(|(_, size)| size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_txn_version() {
        assert_eq!(TransactionVersion::Legacy.to_raw(), sys::FD_TXN_VLEGACY);
        assert_eq!(TransactionVersion::V0.to_raw(), sys::FD_TXN_V0);

        assert_eq!(
            TransactionVersion::from_raw(sys::FD_TXN_VLEGACY),
            Some(TransactionVersion::Legacy)
        );
        assert_eq!(
            TransactionVersion::from_raw(sys::FD_TXN_V0),
            Some(TransactionVersion::V0)
        );
        assert_eq!(TransactionVersion::from_raw(0x42), None);
    }

    #[test]
    fn test_categories() {
        assert_eq!(
            AccountCategory::WRITABLE_SIGNER.0,
            sys::FD_TXN_ACCT_CAT_WRITABLE_SIGNER as i32
        );
        assert_eq!(
            AccountCategory::READONLY_SIGNER.0,
            sys::FD_TXN_ACCT_CAT_READONLY_SIGNER as i32
        );
        assert_eq!(
            AccountCategory::WRITABLE.0,
            sys::FD_TXN_ACCT_CAT_WRITABLE as i32
        );
        assert_eq!(
            AccountCategory::READONLY.0,
            sys::FD_TXN_ACCT_CAT_READONLY as i32
        );
        assert_eq!(AccountCategory::ALL.0, sys::FD_TXN_ACCT_CAT_ALL as i32);
    }

    #[test]
    fn test_compactu16_encoding() {
        let test_cases = [0u16, 1, 127, 128, 255, 16383, 16384, 65535];
        for &val in &test_cases {
            let encoded = compact_u16::encode(val);
            let (decoded, consumed) = compact_u16::decode(&encoded).unwrap();
            assert_eq!(decoded, val);
            assert_eq!(consumed, encoded.len());

            let size = compact_u16::encoded_size(&encoded).unwrap();
            assert_eq!(size, encoded.len());
        }
    }

    #[test]
    fn test_compactu16_invalid() {
        let invalid_cases = [
            &[0x80][..],       // incomplete
            &[0xFF, 0x80][..], // incomplete
            &[0x80, 0x00][..], // non-minimal
        ];

        for &invalid in &invalid_cases {
            assert!(compact_u16::decode(invalid).is_err());
            assert!(compact_u16::encoded_size(invalid).is_none());
        }
    }

    #[test]
    fn test_parse_error() {
        let errors = [
            ParseError::TooShort,
            ParseError::TooLong,
            ParseError::InvalidStructure,
            ParseError::InvalidSignatures,
            ParseError::InvalidAccounts,
            ParseError::InvalidInstructions,
            ParseError::UnsupportedVersion,
            ParseError::InvalidCompactU16,
            ParseError::BufferTooSmall,
        ];

        for error in &errors {
            let display = format!("{}", error);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_parse_empty() {
        let result = Transaction::parse(&[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ParseError::TooShort);
    }

    #[test]
    fn test_parse_too_long() {
        let data = vec![0u8; (sys::FD_TXN_MTU + 1) as usize];
        let result = Transaction::parse(&data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ParseError::TooLong);
    }
}
