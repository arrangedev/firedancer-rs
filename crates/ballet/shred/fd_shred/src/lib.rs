//! Safe API for `fd_shred_sys`

use core::fmt;
use core::slice;
use fd_shred_sys as sys;
use firedancer_rs_common::define_errors;

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

define_errors! {
    ShredErr,
    { TooSmall => "Shred data too small" },
    { Invalid => "Invalid shred data" },
    { BufferTooSmall => "Buffer too small" },
    { InvalidType => "Invalid shred type" },
    { UnsupportedOperation => "Unsupported operation for shred type" }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShredType {
    LegacyData,
    LegacyCode,
    MerkleData,
    MerkleCode,
    MerkleDataChained,
    MerkleCodeChained,
    MerkleDataChainedResigned,
    MerkleCodeChainedResigned,
}

impl ShredType {
    #[inline]
    pub fn is_data(&self) -> bool {
        matches!(
            self,
            ShredType::LegacyData
                | ShredType::MerkleData
                | ShredType::MerkleDataChained
                | ShredType::MerkleDataChainedResigned
        )
    }

    #[inline]
    pub fn is_code(&self) -> bool {
        matches!(
            self,
            ShredType::LegacyCode
                | ShredType::MerkleCode
                | ShredType::MerkleCodeChained
                | ShredType::MerkleCodeChainedResigned
        )
    }

    #[inline]
    pub fn is_legacy(&self) -> bool {
        matches!(self, ShredType::LegacyData | ShredType::LegacyCode)
    }

    #[inline]
    pub fn is_merkle(&self) -> bool {
        !self.is_legacy()
    }

    #[inline]
    pub fn is_chained(&self) -> bool {
        matches!(
            self,
            ShredType::MerkleDataChained
                | ShredType::MerkleCodeChained
                | ShredType::MerkleDataChainedResigned
                | ShredType::MerkleCodeChainedResigned
        )
    }

    #[inline]
    pub fn is_resigned(&self) -> bool {
        matches!(
            self,
            ShredType::MerkleDataChainedResigned | ShredType::MerkleCodeChainedResigned
        )
    }

    #[inline]
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

    #[inline]
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

#[repr(C)]
pub struct Shred<'a> {
    raw: &'a sys::fd_shred_t,
    _data: &'a [u8],
}

impl<'a> Shred<'a> {
    /// Flags are expressed as bitflags in agave
    ///
    /// const SHRED_TICK_REFERENCE_MASK = 0b0011_1111;
    /// const DATA_COMPLETE_SHRED       = 0b0100_0000;
    /// const LAST_SHRED_IN_SLOT        = 0b1100_0000;
    pub fn new_from_data(
        slot: u64,
        index: u32,
        parent_offset: u16,
        data: &'a [u8],
        flags: u8,
        reference_tick: u8,
        version: u16,
        fec_set_index: u32,
        signature: &[u8; 64],
        variant: u8,
    ) -> Result<Self> {
        let raw = Self::create_raw(
            slot,
            index,
            parent_offset,
            data,
            flags,
            version,
            fec_set_index,
            signature,
            variant,
        );

        Ok(Self {
            raw: unsafe { &*raw },
            _data: data,
        })
    }

    fn create_raw(
        slot: u64,
        index: u32,
        parent_offset: u16,
        data: &'a [u8],
        flags: u8,
        version: u16,
        fec_set_index: u32,
        signature: &[u8; 64],
        variant: u8,
    ) -> *const sys::fd_shred_t {
        let raw_shred: sys::fd_shred_t = sys::fd_shred_t {
            signature: *signature,
            variant,
            slot,
            idx: index,
            version,
            fec_set_idx: fec_set_index,
            __bindgen_anon_1: sys::fd_shred__bindgen_ty_1 {
                data: sys::fd_shred__bindgen_ty_1__bindgen_ty_1 {
                    parent_off: parent_offset,
                    flags,
                    size: data.len() as u16,
                },
            },
        };

        &raw_shred as *const sys::fd_shred_t
    }
    /// Parse a shred from raw bytes
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < MIN_SHRED_SIZE {
            return Err(ShredErr::TooSmall);
        }

        let raw_shred = unsafe { sys::fd_shred_parse(data.as_ptr(), data.len() as u64) };

        if raw_shred.is_null() {
            return Err(ShredErr::Invalid);
        }

        Ok(Shred {
            raw: unsafe { &*raw_shred },
            _data: data,
        })
    }

    #[inline]
    pub fn slot(&self) -> u64 {
        self.raw.slot
    }

    #[inline]
    pub fn index(&self) -> u32 {
        self.raw.idx
    }

    /// hash of genesis and hard forks
    #[inline]
    pub fn version(&self) -> u16 {
        self.raw.version
    }

    #[inline]
    pub fn fec_set_index(&self) -> u32 {
        self.raw.fec_set_idx
    }

    #[inline]
    pub fn shred_type(&self) -> ShredType {
        let raw_type = unsafe { sys::fd_shred_type(self.raw.variant) };
        ShredType::from_raw(raw_type).unwrap_or(ShredType::LegacyData)
    }

    #[inline]
    pub fn variant(&self) -> u8 {
        self.raw.variant
    }

    #[inline]
    pub fn signature(&self) -> &[u8; 64] {
        &self.raw.signature
    }

    #[inline]
    pub fn size(&self) -> usize {
        unsafe { sys::fd_shred_sz(self.raw) as usize }
    }

    #[inline]
    pub fn payload_size(&self) -> usize {
        unsafe { sys::fd_shred_payload_sz(self.raw) as usize }
    }

    #[inline]
    pub fn header_size(&self) -> usize {
        unsafe { sys::fd_shred_header_sz(self.raw.variant) as usize }
    }

    #[inline]
    pub fn merkle_node_count(&self) -> u32 {
        unsafe { sys::fd_shred_merkle_cnt(self.raw.variant) }
    }

    #[inline]
    pub fn merkle_proof_size(&self) -> usize {
        unsafe { sys::fd_shred_merkle_sz(self.raw.variant) as usize }
    }

    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
    pub fn is_slot_complete(&self) -> bool {
        if let Some(header) = self.data_header() {
            (header.flags & sys::FD_SHRED_DATA_FLAG_SLOT_COMPLETE) != 0
        } else {
            false
        }
    }

    #[inline]
    pub fn is_data_complete(&self) -> bool {
        if let Some(header) = self.data_header() {
            (header.flags & sys::FD_SHRED_DATA_FLAG_DATA_COMPLETE) != 0
        } else {
            false
        }
    }

    #[inline]
    pub fn reference_tick(&self) -> Option<u8> {
        if let Some(header) = self.data_header() {
            Some(header.flags & sys::FD_SHRED_DATA_REF_TICK_MASK)
        } else {
            None
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DataHeader {
    ///slot difference from parent
    pub parent_offset: u16,
    /// completion and tick info
    pub flags: u8,
    pub size: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CodeHeader {
    /// data shreds in FEC set
    pub data_count: u16,
    /// coding shreds in FEC set
    pub code_count: u16,
    /// idx within coding shreds vec
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
    fn test_parse_shred() {
        let shred_data = vec![0u8; MAX_SHRED_SIZE];
        let shred = Shred::parse(&shred_data).unwrap();
        assert_eq!(shred.slot(), 141939602);
        assert_eq!(shred.index(), 28685);
        assert_eq!(shred.version(), 45189);
    }

    #[test]
    fn test_parse_invalid_shred() {
        let too_small = vec![0u8; 100];
        assert!(matches!(Shred::parse(&too_small), Err(ShredErr::TooSmall)));

        let invalid = vec![0u8; MIN_SHRED_SIZE];
        assert!(matches!(Shred::parse(&invalid), Err(ShredErr::Invalid)));
    }
}
