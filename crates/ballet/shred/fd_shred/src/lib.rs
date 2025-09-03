//! Safe Rust wrappers for Firedancer's shred module.
//!
//! This crate provides safe, idiomatic Rust wrappers around the low-level
//! FFI bindings in `fd_shred_sys`. It handles shred parsing, validation,
//! and provides type-safe access to shred data.
//!
//! # Overview
//!
//! Shreds are the fundamental unit of data transmission in Solana. They
//! represent fragments of block data optimized for transmission over
//! unreliable networks. This crate provides tools to parse, validate,
//! and extract data from shreds.
//!
//! # Shred Types
//!
//! There are several types of shreds:
//! - **Data shreds**: Contain actual block data
//! - **Coding shreds**: Contain Reed-Solomon error correction data
//! - **Legacy shreds**: Original shred format
//! - **Merkle shreds**: Include Merkle inclusion proofs
//! - **Chained shreds**: Include chained Merkle roots
//! - **Resigned shreds**: Include additional retransmitter signatures
//!
//! # Example
//!
//! ```rust
//! use fd_shred::{Shred, ShredType};
//!
//! let shred_data = vec![0u8; 1228];
//!
//! match Shred::parse(&shred_data) {
//!     Ok(shred) => {
//!         let slot = shred.slot();
//!         let idx = shred.index();
//!         let shred_ty = shred.shred_type();
//!         
//!         if let Some(payload) = shred.data_payload() {
//!             println!("payload size: {}", payload.len());
//!         }
//!     }
//!     Err(e) => eprintln!("Failed to parse shred: {e}"),
//! }
//! ```

use core::fmt;
use core::slice;
use fd_shred_sys as sys;

/// max size of a shred (1228 bytes)
pub const MAX_SHRED_SIZE: usize = sys::FD_SHRED_MAX_SZ as usize;

/// min size of a shred (1203 bytes)
pub const MIN_SHRED_SIZE: usize = sys::FD_SHRED_MIN_SZ as usize;

/// size of data shred headers (88 bytes)
pub const DATA_HEADER_SIZE: usize = sys::FD_SHRED_DATA_HEADER_SZ as usize;

/// size of coding shred headers (89 bytes)
pub const CODE_HEADER_SIZE: usize = sys::FD_SHRED_CODE_HEADER_SZ as usize;

/// size of a merkle tree root (32 bytes)
pub const MERKLE_ROOT_SIZE: usize = sys::FD_SHRED_MERKLE_ROOT_SZ as usize;

/// size of a merkle inclusion proof node (20 bytes)
pub const MERKLE_NODE_SIZE: usize = sys::FD_SHRED_MERKLE_NODE_SZ as usize;

/// size of a signature (64 bytes)
pub const SIGNATURE_SIZE: usize = sys::FD_SHRED_SIGNATURE_SZ as usize;

/// Shred type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShredType {
    LegacyData,
    LegacyCode,
    /// Merkle data shred
    MerkleData,
    /// Merkle coding shred
    MerkleCode,
    /// Chained merkle data shred
    MerkleDataChained,
    /// Chained merkle coding shred
    MerkleCodeChained,
    /// Resigned chained merkle data shred
    MerkleDataChainedResigned,
    /// Resigned chained merkle coding shred
    MerkleCodeChainedResigned,
}

impl ShredType {
    /// check if this is a data shred
    pub fn is_data(&self) -> bool {
        matches!(
            self,
            ShredType::LegacyData
                | ShredType::MerkleData
                | ShredType::MerkleDataChained
                | ShredType::MerkleDataChainedResigned
        )
    }

    /// check if this is a coding shred
    pub fn is_code(&self) -> bool {
        matches!(
            self,
            ShredType::LegacyCode
                | ShredType::MerkleCode
                | ShredType::MerkleCodeChained
                | ShredType::MerkleCodeChainedResigned
        )
    }

    /// check if this is a legacy shred
    pub fn is_legacy(&self) -> bool {
        matches!(self, ShredType::LegacyData | ShredType::LegacyCode)
    }

    /// check if this is a merkle shred
    pub fn is_merkle(&self) -> bool {
        !self.is_legacy()
    }

    /// check if this is a chained shred
    pub fn is_chained(&self) -> bool {
        matches!(
            self,
            ShredType::MerkleDataChained
                | ShredType::MerkleCodeChained
                | ShredType::MerkleDataChainedResigned
                | ShredType::MerkleCodeChainedResigned
        )
    }

    /// check if this is a resigned shred
    pub fn is_resigned(&self) -> bool {
        matches!(
            self,
            ShredType::MerkleDataChainedResigned | ShredType::MerkleCodeChainedResigned
        )
    }

    pub fn from_raw(raw_type: u8) -> Option<Self> {
        match raw_type {
            sys::FD_SHRED_TYPE_LEGACY_DATA => Some(ShredType::LegacyData),
            sys::FD_SHRED_TYPE_LEGACY_CODE => Some(ShredType::LegacyCode),
            sys::FD_SHRED_TYPE_MERKLE_DATA => Some(ShredType::MerkleData),
            sys::FD_SHRED_TYPE_MERKLE_CODE => Some(ShredType::MerkleCode),
            sys::FD_SHRED_TYPE_MERKLE_DATA_CHAINED => Some(ShredType::MerkleDataChained),
            sys::FD_SHRED_TYPE_MERKLE_CODE_CHAINED => Some(ShredType::MerkleCodeChained),
            sys::FD_SHRED_TYPE_MERKLE_DATA_CHAINED_RESIGNED => {
                Some(ShredType::MerkleDataChainedResigned)
            }
            sys::FD_SHRED_TYPE_MERKLE_CODE_CHAINED_RESIGNED => {
                Some(ShredType::MerkleCodeChainedResigned)
            }
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn to_raw(self) -> u8 {
        match self {
            ShredType::LegacyData => sys::FD_SHRED_TYPE_LEGACY_DATA,
            ShredType::LegacyCode => sys::FD_SHRED_TYPE_LEGACY_CODE,
            ShredType::MerkleData => sys::FD_SHRED_TYPE_MERKLE_DATA,
            ShredType::MerkleCode => sys::FD_SHRED_TYPE_MERKLE_CODE,
            ShredType::MerkleDataChained => sys::FD_SHRED_TYPE_MERKLE_DATA_CHAINED,
            ShredType::MerkleCodeChained => sys::FD_SHRED_TYPE_MERKLE_CODE_CHAINED,
            ShredType::MerkleDataChainedResigned => sys::FD_SHRED_TYPE_MERKLE_DATA_CHAINED_RESIGNED,
            ShredType::MerkleCodeChainedResigned => sys::FD_SHRED_TYPE_MERKLE_CODE_CHAINED_RESIGNED,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ShredError {
    /// Shred data is too small
    TooSmall,
    /// Shred data is malformed or invalid
    Invalid,
    /// Buffer is too small for the operation
    BufferTooSmall,
    /// Invalid shred type
    InvalidType,
    /// Operation not supported for this shred type
    UnsupportedOperation,
}

impl fmt::Display for ShredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShredError::TooSmall => write!(f, "Shred data too small"),
            ShredError::Invalid => write!(f, "Invalid shred data"),
            ShredError::BufferTooSmall => write!(f, "Buffer too small"),
            ShredError::InvalidType => write!(f, "Invalid shred type"),
            ShredError::UnsupportedOperation => write!(f, "Unsupported operation for shred type"),
        }
    }
}

impl core::error::Error for ShredError {}

pub struct Shred<'a> {
    raw: &'a sys::fd_shred_t,
    _data: &'a [u8],
}

impl<'a> Shred<'a> {
    /// Parse a shred from raw bytes
    ///
    /// # Arguments
    /// * `data` - Raw shred bytes (must be at least MIN_SHRED_SIZE bytes)
    ///
    /// # Returns
    /// * `Ok(Shred)` - Successfully parsed shred
    /// * `Err(ShredError)` - Parse error
    pub fn parse(data: &'a [u8]) -> Result<Self, ShredError> {
        if data.len() < MIN_SHRED_SIZE {
            return Err(ShredError::TooSmall);
        }

        let raw_shred = unsafe { sys::fd_shred_parse(data.as_ptr(), data.len() as u64) };

        if raw_shred.is_null() {
            return Err(ShredError::Invalid);
        }

        Ok(Shred {
            raw: unsafe { &*raw_shred },
            _data: data,
        })
    }

    /// the slot number this shred belongs to
    pub fn slot(&self) -> u64 {
        self.raw.slot
    }

    /// the index of this shred within the slot
    pub fn index(&self) -> u32 {
        self.raw.idx
    }

    /// the version field (hash of genesis and hard forks)
    pub fn version(&self) -> u16 {
        self.raw.version
    }

    /// the FEC set index
    pub fn fec_set_index(&self) -> u32 {
        self.raw.fec_set_idx
    }

    pub fn shred_type(&self) -> ShredType {
        let raw_type = unsafe { sys::fd_shred_type(self.raw.variant) };
        ShredType::from_raw(raw_type).unwrap_or(ShredType::LegacyData)
    }

    pub fn variant(&self) -> u8 {
        self.raw.variant
    }

    /// the signature of this shred
    pub fn signature(&self) -> &[u8; 64] {
        &self.raw.signature
    }

    /// the total size of this shred
    pub fn size(&self) -> usize {
        unsafe { sys::fd_shred_sz(self.raw) as usize }
    }

    pub fn payload_size(&self) -> usize {
        unsafe { sys::fd_shred_payload_sz(self.raw) as usize }
    }

    pub fn header_size(&self) -> usize {
        unsafe { sys::fd_shred_header_sz(self.raw.variant) as usize }
    }

    /// merkle proof node count (excluding root)
    pub fn merkle_node_count(&self) -> u32 {
        unsafe { sys::fd_shred_merkle_cnt(self.raw.variant) }
    }

    /// merkle proof size
    pub fn merkle_proof_size(&self) -> usize {
        unsafe { sys::fd_shred_merkle_sz(self.raw.variant) as usize }
    }

    /// data payload (if this is a data shred)
    pub fn data_payload(&self) -> Option<&[u8]> {
        if !self.shred_type().is_data() {
            return None;
        }

        let payload_ptr = unsafe { sys::fd_shred_data_payload(self.raw) };
        let payload_size = self.payload_size();

        if payload_ptr.is_null() || payload_size == 0 {
            return None;
        }

        Some(unsafe { slice::from_raw_parts(payload_ptr, payload_size) })
    }

    /// coding payload (if this is a coding shred)
    pub fn code_payload(&self) -> Option<&[u8]> {
        if !self.shred_type().is_code() {
            return None;
        }

        let payload_ptr = unsafe { sys::fd_shred_code_payload(self.raw) };
        let payload_size = self.payload_size();

        if payload_ptr.is_null() || payload_size == 0 {
            return None;
        }

        Some(unsafe { slice::from_raw_parts(payload_ptr, payload_size) })
    }

    /// data shred specific fields (only for data shreds)
    pub fn data_header(&self) -> Option<DataHeader> {
        if !self.shred_type().is_data() {
            return None;
        }

        Some(DataHeader {
            parent_offset: unsafe { self.raw.__bindgen_anon_1.data.parent_off },
            flags: unsafe { self.raw.__bindgen_anon_1.data.flags },
            size: unsafe { self.raw.__bindgen_anon_1.data.size },
        })
    }

    /// coding shred specific fields (only for coding shreds)
    pub fn code_header(&self) -> Option<CodeHeader> {
        if !self.shred_type().is_code() {
            return None;
        }

        Some(CodeHeader {
            data_count: unsafe { self.raw.__bindgen_anon_1.code.data_cnt },
            code_count: unsafe { self.raw.__bindgen_anon_1.code.code_cnt },
            index: unsafe { self.raw.__bindgen_anon_1.code.idx },
        })
    }

    /// merkle proof nodes (if this is a merkle shred)
    pub fn merkle_nodes(&self) -> Option<&[u8]> {
        if self.shred_type().is_legacy() {
            return None;
        }

        let node_count = self.merkle_node_count();
        if node_count == 0 {
            return None;
        }

        let nodes_ptr = unsafe { sys::fd_shred_merkle_nodes(self.raw) };
        if nodes_ptr.is_null() {
            return None;
        }

        let total_size = (node_count as usize) * MERKLE_NODE_SIZE;
        Some(unsafe { slice::from_raw_parts(nodes_ptr as *const u8, total_size) })
    }

    /// check if the shred represents the last in a slot
    pub fn is_slot_complete(&self) -> bool {
        if let Some(header) = self.data_header() {
            (header.flags & sys::FD_SHRED_DATA_FLAG_SLOT_COMPLETE) != 0
        } else {
            false
        }
    }

    /// check if the shred is the last in a data batch
    pub fn is_data_complete(&self) -> bool {
        if let Some(header) = self.data_header() {
            (header.flags & sys::FD_SHRED_DATA_FLAG_DATA_COMPLETE) != 0
        } else {
            false
        }
    }

    /// reference tick number from data shred flags
    pub fn reference_tick(&self) -> Option<u8> {
        if let Some(header) = self.data_header() {
            Some(header.flags & sys::FD_SHRED_DATA_REF_TICK_MASK)
        } else {
            None
        }
    }
}

/// Data shred specific header fields
#[derive(Debug, Clone, Copy)]
pub struct DataHeader {
    /// Parent offset (slot difference from parent)
    pub parent_offset: u16,
    /// Flags field containing completion and tick info
    pub flags: u8,
    /// Size of the data shred
    pub size: u16,
}

/// Coding shred specific header fields
#[derive(Debug, Clone, Copy)]
pub struct CodeHeader {
    /// Total number of data shreds in FEC set
    pub data_count: u16,
    /// Total number of coding shreds in FEC set
    pub code_count: u16,
    /// Index within coding shreds vector
    pub index: u16,
}

impl<'a> fmt::Debug for Shred<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shred")
            .field("slot", &self.slot())
            .field("index", &self.index())
            .field("version", &self.version())
            .field("fec_set_index", &self.fec_set_index())
            .field("shred_type", &self.shred_type())
            .field("size", &self.size())
            .field("payload_size", &self.payload_size())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(MAX_SHRED_SIZE, 1228);
        assert_eq!(MIN_SHRED_SIZE, 1203);
        assert_eq!(DATA_HEADER_SIZE, 0x58);
        assert_eq!(CODE_HEADER_SIZE, 0x59);
    }

    #[test]
    fn test_shred_type() {
        assert!(ShredType::LegacyData.is_data());
        assert!(!ShredType::LegacyData.is_code());
        assert!(ShredType::LegacyData.is_legacy());
        assert!(!ShredType::LegacyData.is_merkle());

        assert!(ShredType::MerkleDataChained.is_data());
        assert!(ShredType::MerkleDataChained.is_merkle());
        assert!(ShredType::MerkleDataChained.is_chained());
        assert!(!ShredType::MerkleDataChained.is_resigned());

        assert!(ShredType::MerkleCodeChainedResigned.is_code());
        assert!(ShredType::MerkleCodeChainedResigned.is_chained());
        assert!(ShredType::MerkleCodeChainedResigned.is_resigned());
    }

    #[test]
    fn test_parse_invalid_shred() {
        let too_small = vec![0u8; 100];
        assert!(matches!(
            Shred::parse(&too_small),
            Err(ShredError::TooSmall)
        ));

        let invalid = vec![0u8; MIN_SHRED_SIZE];
        assert!(matches!(Shred::parse(&invalid), Err(ShredError::Invalid)));
    }
}
