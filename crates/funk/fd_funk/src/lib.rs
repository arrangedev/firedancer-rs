//! Safe API for `fd_funk_sys`
//!
//! Funk is a combination database and version control system.
//!
//! # Core Objects
//!
//! Records are the fundamental data storage unit in funk. Each record consists of:
//! - A 40-byte key for indexing
//! - A variable-size value (up to 4GB, or `FD_FUNK_REC_VAL_MAX`)
//! - Indexed for O(1) lookups
//!
//! Transactions represent different states of the database:
//! - Root: The initial state (ID of all zeros)
//! - In-Preparation: Mutable transactions that can be modified
//! - Published: Immutable transactions forming the permanent history
//! - Frozen: Transactions with children that cannot be modified
//!
//! # Core Concepts
//!
//! - Records are indexed by key
//! - Transactions form a tree structure representing different database states
//! - Each transaction has one parent (except root)
//! - Transactions can have multiple children (branches)
//! - Publishing a transaction also publishes all its ancestors
//! - Cancelling a transaction cancels all its descendants
//!
//! Limited concurrent access is also supported:
//! - Record-level operations are thread-safe
//! - Transaction-level operations require exclusive access
//! - Use appropriate synchronization when needed
//!
//! [MAINTAINER NOTES] These can be called concurrently:
//! - `fd_funk_rec_query_try`
//! - `fd_funk_rec_query_test`
//! - `fd_funk_rec_query_try_global`
//! - `fd_funk_rec_prepare`
//! - `fd_funk_rec_publish`
//! - `fd_funk_rec_cancel`
//! - `fd_funk_rec_remove`
//!
//! [MAINTAINER NOTES] These require exclusive access:
//! - `fd_funk_txn_prepare`
//! - `fd_funk_txn_publish`
//! - `fd_funk_txn_cancel`

use fd_funk_sys as sys;
use firedancer_rs_common::define_errors;
use std::fmt;
use std::marker::PhantomData;
use std::ptr::{self, NonNull};

define_errors! {
    FunkErr,
    { InvalidInput => "Invalid input" },
    { TransactionNotFound => "Transaction not found" },
    { RecordNotFound => "Record not found" },
    { TransactionFrozen => "Transaction is frozen" },
    { TransactionLimitReached => "Maximum transactions reached" },
    { RecordLimitReached => "Maximum records reached" },
    { OutOfMemory => "Out of memory" },
    { System => "System error" },
    { Workspace => "Workspace error" },
}

#[derive(Debug, Clone)]
pub struct FunkMetrics {
    pub workspace_backed: bool,
    pub transaction_full: bool,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId([u8; 16]);

impl TransactionId {
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// root transaction id (all zeros)
    pub fn root() -> Self {
        Self([0; 16])
    }

    /// new pseudo-random transaction id
    pub fn generate() -> Self {
        unsafe {
            let xid = sys::fd_funk_generate_xid();
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&xid.uc[..16]);
            Self(bytes)
        }
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// is the root transaction id
    pub fn is_root(&self) -> bool {
        self.0.iter().all(|&b| b == 0)
    }

    fn to_sys(&self) -> sys::fd_funk_txn_xid_t {
        let mut xid = sys::fd_funk_txn_xid_t { uc: [0; 16] };
        unsafe {
            xid.uc[..16].copy_from_slice(&self.0);
        }
        xid
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RecordKey([u8; 40]);

impl RecordKey {
    /// if the input is shorter than 40 bytes, it will be zero-padded
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 40 {
            return Err(FunkErr::InvalidInput);
        }

        let mut key = [0u8; 40];
        key[..bytes.len()].copy_from_slice(bytes);
        Ok(Self(key))
    }

    pub fn from_str(s: &str) -> Result<Self> {
        Self::from_bytes(s.as_bytes())
    }

    pub fn as_bytes(&self) -> &[u8; 40] {
        &self.0
    }

    fn to_sys(&self) -> sys::fd_funk_rec_key_t {
        let mut key = sys::fd_funk_rec_key_t { uc: [0; 40] };
        unsafe {
            key.uc.copy_from_slice(&self.0);
        }
        key
    }
}

#[repr(C)]
pub struct Record<'a> {
    _phantom: PhantomData<&'a ()>,
    key: RecordKey,
    value: Vec<u8>,
    flags: u64,
}

impl<'a> Record<'a> {
    pub fn key(&self) -> &RecordKey {
        &self.key
    }

    pub fn value(&self) -> &[u8] {
        &self.value
    }

    pub fn flags(&self) -> u64 {
        self.flags
    }

    pub fn is_erased(&self) -> bool {
        self.flags & (sys::FD_FUNK_REC_FLAG_ERASE as u64) != 0
    }
}

#[repr(C)]
pub struct Transaction<'a> {
    _phantom: PhantomData<&'a ()>,
    id: TransactionId,
    funk_ptr: NonNull<sys::fd_funk_t>,
    txn_ptr: Option<NonNull<sys::fd_funk_txn_t>>,
}

impl<'a> Transaction<'a> {
    pub fn id(&self) -> &TransactionId {
        &self.id
    }

    /// is the root transaction
    pub fn is_root(&self) -> bool {
        self.id.is_root()
    }

    /// transaction is frozen (has children)
    pub fn is_frozen(&self) -> bool {
        match self.txn_ptr {
            Some(ptr) => unsafe { sys::fd_funk_txn_is_frozen(ptr.as_ptr()) != 0 },
            None => false, // root tx
        }
    }
}

pub struct FunkBuilder {
    max_transactions: u64,
    max_records: u64,
    workspace_tag: u64,
    seed: u64,
}

impl FunkBuilder {
    pub fn new() -> Self {
        Self {
            max_transactions: 1000,
            max_records: 10000,
            workspace_tag: 1,
            seed: 42,
        }
    }

    /// max number of in-preparation transactions
    pub fn with_max_transactions(mut self, max: u64) -> Self {
        self.max_transactions = max;
        self
    }

    /// max number of records
    pub fn with_max_records(mut self, max: u64) -> Self {
        self.max_records = max;
        self
    }

    /// workspace tag for allocations
    pub fn with_workspace_tag(mut self, tag: u64) -> Self {
        self.workspace_tag = tag;
        self
    }

    /// hash seed
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// build a funk instance backed by a single heap allocation
    pub fn build_with_alloc(self) -> Result<Funk> {
        Funk::new_from_alloc(
            self.workspace_tag,
            self.seed,
            self.max_transactions,
            self.max_records,
        )
    }

    /// build the funk instance backed by memory from an existing wksp
    pub fn build(self, workspace: *mut core::ffi::c_void) -> Result<Funk> {
        Funk::new(
            workspace,
            self.workspace_tag,
            self.seed,
            self.max_transactions,
            self.max_records,
        )
    }
}

impl Default for FunkBuilder {
    fn default() -> Self {
        Self::new()
    }
}

enum Backing {
    /// wksp-managed memory
    Workspace,
    /// heap-allocation from the global allocator
    Heap { layout: std::alloc::Layout },
}

pub struct Funk {
    funk_ptr: NonNull<sys::fd_funk_t>,
    _shmem_ptr: NonNull<u8>, // keep reference to prevent deallocation
    _memory: Backing,
}

impl Funk {
    /// Create a new funk instance using a single heap allocation.
    ///
    /// This uses the configured global allocator.
    fn new_from_alloc(
        workspace_tag: u64,
        seed: u64,
        max_transactions: u64,
        max_records: u64,
    ) -> Result<Self> {
        unsafe {
            let footprint = sys::fd_funk_footprint(max_transactions, max_records);
            if footprint == 0 {
                return Err(FunkErr::InvalidInput);
            }

            let layout = std::alloc::Layout::from_size_align(
                footprint as usize,
                sys::FD_FUNK_ALIGN as usize,
            )
            .map_err(|_| FunkErr::OutOfMemory)?;

            let shmem_ptr = std::alloc::alloc_zeroed(layout);
            if shmem_ptr.is_null() {
                return Err(FunkErr::OutOfMemory);
            }

            Self::create_instance(
                shmem_ptr as *mut core::ffi::c_void,
                workspace_tag,
                seed,
                max_transactions,
                max_records,
                Backing::Heap { layout },
            )
        }
    }

    /// Create a new funk instance using memory from an existing wksp.
    ///
    ///
    fn new(
        workspace: *mut core::ffi::c_void,
        workspace_tag: u64,
        seed: u64,
        max_transactions: u64,
        max_records: u64,
    ) -> Result<Self> {
        if workspace.is_null() {
            return Err(FunkErr::InvalidInput);
        }

        Self::create_instance(
            workspace,
            workspace_tag,
            seed,
            max_transactions,
            max_records,
            Backing::Workspace,
        )
    }

    fn create_instance(
        shmem_ptr: *mut core::ffi::c_void,
        workspace_tag: u64,
        seed: u64,
        max_transactions: u64,
        max_records: u64,
        memory: Backing,
    ) -> Result<Self> {
        unsafe {
            let funk_shmem = sys::fd_funk_new(
                shmem_ptr,
                workspace_tag,
                seed,
                max_transactions,
                max_records,
            );

            if funk_shmem.is_null() {
                if let Backing::Heap { layout } = memory {
                    std::alloc::dealloc(shmem_ptr as *mut u8, layout);
                }
                return Err(FunkErr::System);
            }

            let mut join_mem = std::mem::MaybeUninit::<sys::fd_funk_t>::uninit();
            let funk_ptr = sys::fd_funk_join(join_mem.as_mut_ptr(), funk_shmem);

            if funk_ptr.is_null() {
                sys::fd_funk_delete(funk_shmem);
                if let Backing::Heap { layout } = memory {
                    std::alloc::dealloc(shmem_ptr as *mut u8, layout);
                }
                return Err(FunkErr::System);
            }

            Ok(Self {
                funk_ptr: NonNull::new_unchecked(funk_ptr),
                _shmem_ptr: NonNull::new_unchecked(shmem_ptr as *mut u8),
                _memory: memory,
            })
        }
    }

    pub fn prepare_transaction(
        &self,
        parent: Option<&Transaction<'_>>,
        id: &TransactionId,
    ) -> Result<Transaction<'_>> {
        unsafe {
            let parent_ptr = match parent {
                Some(txn) => txn.txn_ptr.map(|p| p.as_ptr()).unwrap_or(ptr::null_mut()),
                None => ptr::null_mut(),
            };

            let xid = id.to_sys();
            let txn_ptr = sys::fd_funk_txn_prepare(
                self.funk_ptr.as_ptr(),
                parent_ptr,
                &xid,
                1, // verbose
            );

            if txn_ptr.is_null() {
                return Err(FunkErr::TransactionNotFound);
            }

            Ok(Transaction {
                _phantom: PhantomData,
                id: *id,
                funk_ptr: self.funk_ptr,
                txn_ptr: NonNull::new(txn_ptr),
            })
        }
    }

    pub fn publish_transaction(&self, txn: &Transaction) -> Result<usize> {
        unsafe {
            let txn_ptr = txn.txn_ptr.ok_or_else(|| FunkErr::InvalidInput)?;

            let count = sys::fd_funk_txn_publish(
                self.funk_ptr.as_ptr(),
                txn_ptr.as_ptr(),
                1, // verbose
            );

            if count == 0 {
                return Err(FunkErr::System);
            }

            Ok(count as usize)
        }
    }

    pub fn cancel_transaction(&self, txn: &Transaction) -> Result<usize> {
        unsafe {
            let txn_ptr = txn.txn_ptr.ok_or_else(|| FunkErr::InvalidInput)?;

            let count = sys::fd_funk_txn_cancel(
                self.funk_ptr.as_ptr(),
                txn_ptr.as_ptr(),
                1, // verbose
            );

            Ok(count as usize)
        }
    }

    pub fn insert_record(&self, txn: &Transaction, key: &RecordKey, value: &[u8]) -> Result<()> {
        unsafe {
            let txn_ptr = txn.txn_ptr.map(|p| p.as_ptr()).unwrap_or(ptr::null_mut());
            let sys_key = key.to_sys();

            let mut prepare = std::mem::MaybeUninit::<sys::fd_funk_rec_prepare_t>::uninit();
            let mut err = 0i32;

            let rec_ptr = sys::fd_funk_rec_prepare(
                self.funk_ptr.as_ptr(),
                txn_ptr,
                &sys_key,
                prepare.as_mut_ptr(),
                &mut err,
            );

            if rec_ptr.is_null() {
                return Err(match err {
                    _ if err == sys::FD_FUNK_ERR_REC => FunkErr::RecordLimitReached,
                    _ if err == sys::FD_FUNK_ERR_MEM => FunkErr::OutOfMemory,
                    _ => FunkErr::System,
                });
            }

            if !value.is_empty() {
                let wksp = sys::fd_funk_wksp(self.funk_ptr.as_ptr());
                let alloc = sys::fd_funk_alloc(self.funk_ptr.as_ptr());

                let val_ptr = sys::fd_funk_val_truncate(
                    rec_ptr,
                    alloc,
                    wksp,
                    0, // align
                    value.len() as u64,
                    &mut err,
                );

                if val_ptr.is_null() {
                    sys::fd_funk_rec_cancel(self.funk_ptr.as_ptr(), prepare.as_mut_ptr());
                    return Err(match err {
                        _ if err == sys::FD_FUNK_ERR_MEM => FunkErr::OutOfMemory,
                        _ => FunkErr::System,
                    });
                }

                ptr::copy_nonoverlapping(value.as_ptr(), val_ptr as *mut u8, value.len());
            }

            sys::fd_funk_rec_publish(self.funk_ptr.as_ptr(), prepare.as_mut_ptr());

            Ok(())
        }
    }

    pub fn query_record(&self, txn: &Transaction<'_>, key: &RecordKey) -> Result<Record<'_>> {
        unsafe {
            let txn_ptr = txn.txn_ptr.map(|p| p.as_ptr()).unwrap_or(ptr::null_mut());
            let sys_key = key.to_sys();
            let mut query = std::mem::MaybeUninit::<sys::fd_funk_rec_query_t>::uninit();

            let rec_ptr = sys::fd_funk_rec_query_try(
                self.funk_ptr.as_ptr(),
                txn_ptr,
                &sys_key,
                query.as_mut_ptr(),
            );

            if rec_ptr.is_null() {
                return Err(FunkErr::RecordNotFound);
            }

            let val_sz = sys::fd_funk_val_sz(rec_ptr) as usize;
            let mut value = vec![0u8; val_sz];

            if val_sz > 0 {
                let wksp = sys::fd_funk_wksp(self.funk_ptr.as_ptr());
                let val_ptr = sys::fd_funk_val_const(rec_ptr, wksp);

                if !val_ptr.is_null() {
                    ptr::copy_nonoverlapping(val_ptr as *const u8, value.as_mut_ptr(), val_sz);
                }
            }

            Ok(Record {
                _phantom: PhantomData,
                key: *key,
                value,
                flags: (*rec_ptr).flags,
            })
        }
    }

    pub fn remove_record(&self, txn: &Transaction, key: &RecordKey) -> Result<()> {
        unsafe {
            let txn_ptr = txn.txn_ptr.map(|p| p.as_ptr()).unwrap_or(ptr::null_mut());
            let sys_key = key.to_sys();
            let mut rec_out = ptr::null_mut();

            let result =
                sys::fd_funk_rec_remove(self.funk_ptr.as_ptr(), txn_ptr, &sys_key, &mut rec_out);

            match result {
                _ if result == sys::FD_FUNK_SUCCESS as i32 => Ok(()),
                _ if result == sys::FD_FUNK_ERR_KEY => Err(FunkErr::RecordNotFound),
                _ if result == sys::FD_FUNK_ERR_INVAL => Err(FunkErr::InvalidInput),
                _ => Err(FunkErr::System),
            }
        }
    }

    pub fn root_transaction(&self) -> Transaction<'_> {
        Transaction {
            _phantom: PhantomData,
            id: TransactionId::root(),
            funk_ptr: self.funk_ptr,
            txn_ptr: None,
        }
    }

    pub fn is_transaction_full(&self) -> bool {
        unsafe { sys::fd_funk_txn_is_full(self.funk_ptr.as_ptr()) != 0 }
    }

    pub fn metrics(&self) -> FunkMetrics {
        FunkMetrics {
            workspace_backed: self.is_workspace_backed(),
            transaction_full: self.is_transaction_full(),
        }
    }

    pub fn is_workspace_backed(&self) -> bool {
        matches!(self._memory, Backing::Workspace)
    }
}

unsafe impl Send for Funk {}
unsafe impl Sync for Funk {}

impl Drop for Funk {
    fn drop(&mut self) {
        unsafe {
            let mut shmem_ptr = ptr::null_mut();
            sys::fd_funk_leave(self.funk_ptr.as_ptr(), &mut shmem_ptr);

            if !shmem_ptr.is_null() {
                sys::fd_funk_delete(shmem_ptr);

                match &self._memory {
                    Backing::Heap { layout } => {
                        std::alloc::dealloc(self._shmem_ptr.as_ptr(), *layout);
                    }
                    Backing::Workspace => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transactionid_gen() {
        let id1 = TransactionId::generate();
        let id2 = TransactionId::generate();
        assert_ne!(id1, id2);

        let root = TransactionId::root();
        assert!(root.is_root());
        assert!(!id1.is_root());

        let bytes = id1.as_bytes();
        let id3 = TransactionId::from_bytes(*bytes);
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_recordkey_ser() {
        let key1 = RecordKey::from_bytes(b"test_key").unwrap();
        let key2 = RecordKey::from_str("test_key").unwrap();
        assert_eq!(key1, key2);

        let long_bytes = vec![0u8; 50];
        assert!(RecordKey::from_bytes(&long_bytes).is_err());

        let short_key = RecordKey::from_bytes(b"short").unwrap();
        let bytes = short_key.as_bytes();
        assert_eq!(&bytes[..5], b"short");
        assert_eq!(&bytes[5..], &[0u8; 35]);
    }

    #[test]
    fn test_core_ops() {
        let funk = FunkBuilder::new()
            .with_max_transactions(10)
            .with_max_records(100)
            .build_with_alloc()
            .unwrap();

        let metrics = funk.metrics();
        assert!(!metrics.workspace_backed);
        assert!(!metrics.transaction_full);

        let root = funk.root_transaction();
        assert!(root.is_root());
        assert!(!root.is_frozen());

        let txn_id = TransactionId::generate();
        let txn = funk.prepare_transaction(None, &txn_id).unwrap();
        assert_eq!(txn.id(), &txn_id);
        assert!(!txn.is_root());
        assert!(!txn.is_frozen());

        let key = RecordKey::from_str("test_record").unwrap();
        let value = b"test_value";
        funk.insert_record(&txn, &key, value).unwrap();

        let record = funk.query_record(&txn, &key).unwrap();
        assert_eq!(record.key(), &key);
        assert_eq!(record.value(), value);
        assert!(!record.is_erased());

        let count = funk.publish_transaction(&txn).unwrap();
        assert!(count > 0);

        let root_record = funk.query_record(&root, &key).unwrap();
        assert_eq!(root_record.value(), value);
    }

    #[test]
    fn test_tx_branching() {
        let funk = FunkBuilder::new()
            .with_max_transactions(20)
            .with_max_records(100)
            .build_with_alloc()
            .unwrap();

        let root = funk.root_transaction();
        let key = RecordKey::from_str("shared_key").unwrap();

        // create first transaction and insert a record
        let txn1_id = TransactionId::generate();
        let txn1 = funk.prepare_transaction(None, &txn1_id).unwrap();
        funk.insert_record(&txn1, &key, b"value1").unwrap();

        // create two branches from txn1
        let txn2_id = TransactionId::generate();
        let txn2 = funk.prepare_transaction(Some(&txn1), &txn2_id).unwrap();
        funk.insert_record(&txn2, &key, b"value2").unwrap();

        let txn3_id = TransactionId::generate();
        let txn3 = funk.prepare_transaction(Some(&txn1), &txn3_id).unwrap();
        funk.insert_record(&txn3, &key, b"value3").unwrap();

        // query shows different values in each branch
        assert_eq!(funk.query_record(&txn1, &key).unwrap().value(), b"value1");
        assert_eq!(funk.query_record(&txn2, &key).unwrap().value(), b"value2");
        assert_eq!(funk.query_record(&txn3, &key).unwrap().value(), b"value3");

        // root should not see any changes
        assert!(funk.query_record(&root, &key).is_err());

        // publish txn2 (which includes txn1)
        funk.publish_transaction(&txn2).unwrap();

        // root should now see value2
        assert_eq!(funk.query_record(&root, &key).unwrap().value(), b"value2");

        // txn3 should still see its own value, since it's still in prep
        assert_eq!(funk.query_record(&txn3, &key).unwrap().value(), b"value3");
    }

    #[test]
    fn test_record_ops() {
        let funk = FunkBuilder::new()
            .with_max_transactions(10)
            .with_max_records(50)
            .build_with_alloc()
            .unwrap();

        let txn_id = TransactionId::generate();
        let txn = funk.prepare_transaction(None, &txn_id).unwrap();

        let key1 = RecordKey::from_str("key1").unwrap();
        let key2 = RecordKey::from_str("key2").unwrap();

        funk.insert_record(&txn, &key1, b"data1").unwrap();
        funk.insert_record(&txn, &key2, b"longer_data_string")
            .unwrap();

        let record1 = funk.query_record(&txn, &key1).unwrap();
        let record2 = funk.query_record(&txn, &key2).unwrap();
        assert_eq!(record1.value(), b"data1");
        assert_eq!(record2.value(), b"longer_data_string");

        funk.insert_record(&txn, &key1, b"updated_data1").unwrap();
        let updated_record = funk.query_record(&txn, &key1).unwrap();
        assert_eq!(updated_record.value(), b"updated_data1");

        let key3 = RecordKey::from_str("empty_key").unwrap();
        funk.insert_record(&txn, &key3, b"").unwrap();
        let empty_record = funk.query_record(&txn, &key3).unwrap();
        assert_eq!(empty_record.value(), b"");

        funk.remove_record(&txn, &key2).unwrap();
        assert!(funk.query_record(&txn, &key2).is_err());

        // key1 and key3 should still exist
        assert!(funk.query_record(&txn, &key1).is_ok());
        assert!(funk.query_record(&txn, &key3).is_ok());
    }

    #[test]
    fn test_tx_cancel() {
        let funk = FunkBuilder::new()
            .with_max_transactions(10)
            .with_max_records(50)
            .build_with_alloc()
            .unwrap();

        let root = funk.root_transaction();
        let key = RecordKey::from_str("test_key").unwrap();

        let parent_id = TransactionId::generate();
        let parent = funk.prepare_transaction(None, &parent_id).unwrap();
        funk.insert_record(&parent, &key, b"parent_value").unwrap();

        let child_id = TransactionId::generate();
        let child = funk.prepare_transaction(Some(&parent), &child_id).unwrap();
        funk.insert_record(&child, &key, b"child_value").unwrap();

        // child sees its value
        assert_eq!(
            funk.query_record(&child, &key).unwrap().value(),
            b"child_value"
        );

        // cancel child transaction
        let cancelled_count = funk.cancel_transaction(&child).unwrap();
        assert_eq!(cancelled_count, 1);

        // parent should still see its value
        assert_eq!(
            funk.query_record(&parent, &key).unwrap().value(),
            b"parent_value"
        );

        // root should not see anything yet
        assert!(funk.query_record(&root, &key).is_err());

        // publish parent
        funk.publish_transaction(&parent).unwrap();

        // root should now see parent's value
        assert_eq!(
            funk.query_record(&root, &key).unwrap().value(),
            b"parent_value"
        );
    }

    #[test]
    fn test_error_conditions() {
        let funk = FunkBuilder::new()
            .with_max_transactions(2)
            .with_max_records(10)
            .build_with_alloc()
            .unwrap();

        // invalid record key
        let long_key_result = RecordKey::from_bytes(&vec![0u8; 50]);
        assert!(matches!(long_key_result, Err(FunkErr::InvalidInput)));

        // querying non-existent record
        let root = funk.root_transaction();
        let key = RecordKey::from_str("nonexistent").unwrap();
        let result = funk.query_record(&root, &key);
        assert!(matches!(result, Err(FunkErr::RecordNotFound)));

        // removing non-existent record
        let txn_id = TransactionId::generate();
        let txn = funk.prepare_transaction(None, &txn_id).unwrap();
        let result = funk.remove_record(&txn, &key);
        assert!(matches!(result, Err(FunkErr::RecordNotFound)));
    }

    #[test]
    fn test_large_values() {
        let funk = FunkBuilder::new()
            .with_max_transactions(5)
            .with_max_records(10)
            .build_with_alloc()
            .unwrap();

        let txn_id = TransactionId::generate();
        let txn = funk.prepare_transaction(None, &txn_id).unwrap();

        let sizes = [0, 1, 100, 1024, 4096, 65536];

        for (i, &size) in sizes.iter().enumerate() {
            let key = RecordKey::from_str(&format!("key_{}", i)).unwrap();
            let value = vec![i as u8; size];

            funk.insert_record(&txn, &key, &value).unwrap();
            let record = funk.query_record(&txn, &key).unwrap();

            assert_eq!(record.value().len(), size);
            if size > 0 {
                assert!(record.value().iter().all(|&b| b == i as u8));
            }
        }
    }

    #[test]
    fn test_tx_capacity() {
        let funk = FunkBuilder::new()
            .with_max_transactions(2)
            .with_max_records(10)
            .build_with_alloc()
            .unwrap();

        // should not be full initially
        assert!(!funk.is_transaction_full());

        // create transactions up to limit
        let txn1_id = TransactionId::generate();
        let _txn1 = funk.prepare_transaction(None, &txn1_id).unwrap();

        let txn2_id = TransactionId::generate();
        let _txn2 = funk.prepare_transaction(None, &txn2_id).unwrap();

        // should now be full
        assert!(funk.is_transaction_full());
    }
}
