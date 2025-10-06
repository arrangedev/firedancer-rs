//! Raw FFI bindings to Firedancer's fd_funk module.
//!
//! # Platform Support
//!
//! x86_64 Linux targets have the full feature set. Users building for aarch64 targets
//! like macOS will be missing:
//! - NUMA memory policy (`fd_numa_linux.c`)
//! - Shared memory (`fd_shmem_admin.c`, `fd_shmem_ctl.c`)
//! - Memory allocation control (`fd_alloc_ctl.c`)
//!
//! [MAINTAINER NOTES] Top level
//! - `fd_funk_new`: Create a new funk instance
//! - `fd_funk_join`: Join to an existing funk instance
//! - `fd_funk_leave`: Leave a funk instance
//! - `fd_funk_delete`: Delete a funk instance
//!
//! [MAINTAINER NOTES] Transaction
//! - `fd_funk_txn_prepare`: Start a new transaction
//! - `fd_funk_txn_publish`: Publish a transaction (make it immutable)
//! - `fd_funk_txn_cancel`: Cancel a transaction and its descendants
//! - `fd_funk_txn_query`: Find a transaction by ID
//!
//! [MAINTAINER NOTES] Record
//! - `fd_funk_rec_query_try`: Query for a record in a transaction
//! - `fd_funk_rec_query_try_global`: Query with ancestor traversal
//! - `fd_funk_rec_prepare`: Prepare to insert a new record
//! - `fd_funk_rec_publish`: Publish a prepared record
//! - `fd_funk_rec_remove`: Remove a record

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!(FD_FUNK_REC_KEY_FOOTPRINT > 0);
        assert!(FD_FUNK_TXN_XID_FOOTPRINT > 0);
        assert!(FD_FUNK_ALIGN > 0);
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(FD_FUNK_SUCCESS, 0);
        assert!(FD_FUNK_ERR_INVAL < 0);
        assert!(FD_FUNK_ERR_XID < 0);
        assert!(FD_FUNK_ERR_KEY < 0);
        assert!(FD_FUNK_ERR_FROZEN < 0);
    }

    #[test]
    fn test_footprint_calculation() {
        unsafe {
            let footprint = fd_funk_footprint(100, 1000);
            assert!(footprint > 0);
            let larger_footprint = fd_funk_footprint(200, 2000);
            assert!(larger_footprint > footprint);
        }
    }
}
