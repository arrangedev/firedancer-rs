//! Low-level FFI bindings to Firedancer's fd_ed25519 module.
//!
//! This crate provides raw, unsafe bindings to the Firedancer Ed25519 API,
//! including Ed25519 signature generation and verification, X25519 key exchange,
//! and Ristretto255 operations.
//!
//! For safe, idiomatic Rust wrappers, see the `fd_ed25519` crate.
//!
//! # Safety
//!
//! All functions in this crate are unsafe and require careful handling of:
//! - Memory management and lifetime guarantees
//! - Proper initialization of SHA512 calculators
//! - Buffer size requirements (32 bytes for keys, 64 bytes for signatures)
//! - Thread safety considerations
//!
//! # Ed25519 Operations
//!
//! The main Ed25519 operations available:
//! - `fd_ed25519_public_from_private`: Derive public key from private key
//! - `fd_ed25519_sign`: Sign a message with Ed25519
//! - `fd_ed25519_verify`: Verify an Ed25519 signature
//! - `fd_ed25519_verify_batch_single_msg`: Batch verification for multiple signatures
//!
//! # X25519 Operations
//!
//! X25519 Diffie-Hellman key exchange:
//! - `fd_x25519_public`: Generate public key from private key
//! - `fd_x25519_exchange`: Compute shared secret
//!
//! # Example
//!
//! ```rust,no_run
//! use fd_ed25519_sys::*;
//! use std::mem::MaybeUninit;
//!
//! unsafe {
//!     // Initialize SHA512 calculator (required for Ed25519 operations)
//!     let mut sha = MaybeUninit::<fd_sha512_t>::uninit();
//!     fd_sha512_init(sha.as_mut_ptr());
//!     let mut sha = sha.assume_init();
//!
//!     // Generate key pair
//!     let private_key: [u8; 32] = [1; 32]; // In practice, use secure random bytes
//!     let mut public_key = [0u8; 32];
//!     fd_ed25519_public_from_private(
//!         public_key.as_mut_ptr(),
//!         private_key.as_ptr(),
//!         &mut sha
//!     );
//!
//!     // Sign a message
//!     let message = b"Hello, world!";
//!     let mut signature = [0u8; 64];
//!     fd_ed25519_sign(
//!         signature.as_mut_ptr(),
//!         message.as_ptr(),
//!         message.len() as u64,
//!         public_key.as_ptr(),
//!         private_key.as_ptr(),
//!         &mut sha
//!     );
//!
//!     // Verify the signature
//!     let result = fd_ed25519_verify(
//!         message.as_ptr(),
//!         message.len() as u64,
//!         signature.as_ptr(),
//!         public_key.as_ptr(),
//!         &mut sha
//!     );
//!
//!     if result == FD_ED25519_SUCCESS {
//!         println!("Signature verification successful");
//!     } else {
//!         println!("Signature verification failed: {}", result);
//!     }
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::MaybeUninit;

    #[test]
    fn test_strerror() {
        unsafe {
            let success_str = fd_ed25519_strerror(FD_ED25519_SUCCESS as i32);
            assert!(!success_str.is_null());

            let sig_err_str = fd_ed25519_strerror(FD_ED25519_ERR_SIG);
            assert!(!sig_err_str.is_null());

            let pubkey_err_str = fd_ed25519_strerror(FD_ED25519_ERR_PUBKEY);
            assert!(!pubkey_err_str.is_null());

            let msg_err_str = fd_ed25519_strerror(FD_ED25519_ERR_MSG);
            assert!(!msg_err_str.is_null());
        }
    }

    #[test]
    fn test_keygen() {
        unsafe {
            let mut sha = MaybeUninit::<fd_sha512_t>::uninit();
            fd_sha512_init(sha.as_mut_ptr());
            let mut sha = sha.assume_init();

            let private_key: [u8; 32] = [
                0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec,
                0x2c, 0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03,
                0x1c, 0xae, 0x7f, 0x60,
            ];

            let mut public_key = [0u8; 32];
            let result = fd_ed25519_public_from_private(
                public_key.as_mut_ptr(),
                private_key.as_ptr(),
                &mut sha,
            );
            assert!(!result.is_null());
            assert_ne!(public_key, [0u8; 32]);
        }
    }

    #[test]
    fn test_sigverify() {
        unsafe {
            let mut sha = MaybeUninit::<fd_sha512_t>::uninit();
            fd_sha512_init(sha.as_mut_ptr());
            let mut sha = sha.assume_init();

            let private_key: [u8; 32] = [
                0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec,
                0x2c, 0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03,
                0x1c, 0xae, 0x7f, 0x60,
            ];

            let mut public_key = [0u8; 32];
            fd_ed25519_public_from_private(public_key.as_mut_ptr(), private_key.as_ptr(), &mut sha);

            let message = b"test message";
            let mut signature = [0u8; 64];
            let result = fd_ed25519_sign(
                signature.as_mut_ptr(),
                message.as_ptr(),
                message.len() as u64,
                public_key.as_ptr(),
                private_key.as_ptr(),
                &mut sha,
            );
            assert!(!result.is_null());

            let verify_result = fd_ed25519_verify(
                message.as_ptr(),
                message.len() as u64,
                signature.as_ptr(),
                public_key.as_ptr(),
                &mut sha,
            );
            assert_eq!(verify_result, FD_ED25519_SUCCESS as i32);

            let wrong_message = b"wrong message";
            let wrong_verify_result = fd_ed25519_verify(
                wrong_message.as_ptr(),
                wrong_message.len() as u64,
                signature.as_ptr(),
                public_key.as_ptr(),
                &mut sha,
            );
            assert_ne!(wrong_verify_result, FD_ED25519_SUCCESS as i32);
        }
    }

    #[test]
    fn test_empty_msg() {
        unsafe {
            let mut sha = MaybeUninit::<fd_sha512_t>::uninit();
            fd_sha512_init(sha.as_mut_ptr());
            let mut sha = sha.assume_init();

            let private_key: [u8; 32] = [1; 32];
            let mut public_key = [0u8; 32];
            fd_ed25519_public_from_private(public_key.as_mut_ptr(), private_key.as_ptr(), &mut sha);

            let mut signature = [0u8; 64];
            let result = fd_ed25519_sign(
                signature.as_mut_ptr(),
                std::ptr::null(),
                0,
                public_key.as_ptr(),
                private_key.as_ptr(),
                &mut sha,
            );
            assert!(!result.is_null());

            let verify_result = fd_ed25519_verify(
                std::ptr::null(),
                0,
                signature.as_ptr(),
                public_key.as_ptr(),
                &mut sha,
            );
            assert_eq!(verify_result, FD_ED25519_SUCCESS as i32);
        }
    }

    #[cfg(feature = "x25519")]
    #[test]
    fn test_x25519_exchange() {
        unsafe {
            let alice_private: [u8; 32] = [
                0x77, 0x07, 0x6d, 0x0a, 0x73, 0x18, 0xa5, 0x7d, 0x3c, 0x16, 0xc1, 0x72, 0x51, 0xb2,
                0x66, 0x45, 0xdf, 0x4c, 0x2f, 0x87, 0xeb, 0xc0, 0x99, 0x2a, 0xb1, 0x77, 0xfb, 0xa5,
                0x1d, 0xb9, 0x2c, 0x2a,
            ];

            let bob_private: [u8; 32] = [
                0x5d, 0xab, 0x08, 0x7e, 0x62, 0x4a, 0x8a, 0x4b, 0x79, 0xe1, 0x7f, 0x8b, 0x83, 0x80,
                0x0e, 0xe6, 0x6f, 0x3b, 0xb1, 0x29, 0x26, 0x18, 0xb6, 0xfd, 0x1c, 0x2f, 0x8b, 0x27,
                0xff, 0x88, 0xe0, 0xeb,
            ];

            let mut alice_public = [0u8; 32];
            let mut bob_public = [0u8; 32];

            let alice_pub_result =
                fd_x25519_public(alice_public.as_mut_ptr(), alice_private.as_ptr());
            assert!(!alice_pub_result.is_null());

            let bob_pub_result = fd_x25519_public(bob_public.as_mut_ptr(), bob_private.as_ptr());
            assert!(!bob_pub_result.is_null());

            let mut alice_shared = [0u8; 32];
            let mut bob_shared = [0u8; 32];

            let alice_exchange_result = fd_x25519_exchange(
                alice_shared.as_mut_ptr(),
                alice_private.as_ptr(),
                bob_public.as_ptr(),
            );
            assert!(!alice_exchange_result.is_null());

            let bob_exchange_result = fd_x25519_exchange(
                bob_shared.as_mut_ptr(),
                bob_private.as_ptr(),
                alice_public.as_ptr(),
            );
            assert!(!bob_exchange_result.is_null());

            // shared secrets should be identical
            assert_eq!(alice_shared, bob_shared);
            assert_ne!(alice_shared, [0u8; 32]);
        }
    }
}
