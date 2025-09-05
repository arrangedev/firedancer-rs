//! This crate provides safe abstractions over the raw bindings in `fd_ed25519_sys`.
//!
//! ## Keygen + Signing
//!
//! ```rust
//! use fd_ed25519::{Keypair, Signature};
//!
//! let secret_key = [1u8; 32]; // not secure, don't actually do this
//! let keypair = Keypair::from_secret_key(&secret_key).unwrap();
//!
//! let message = b"Hello, world!";
//! let signature = keypair.sign(message).unwrap();
//!
//! assert!(keypair.pubkey().verify(message, &signature).unwrap());
//! ```
//!
//! ## Sigverify
//!
//! ```rust
//! use fd_ed25519::{Keypair, Pubkey, Signature};
//!
//! let secret_key = [1u8; 32]; // not secure, don't actually do this
//! let keypair = Keypair::from_secret_key(&secret_key).unwrap();
//! let message = b"Hello, world!";
//! let signature = keypair.sign(message).unwrap();
//!
//! let pubkey = keypair.pubkey();
//! let is_valid = pubkey.verify(message, &signature).unwrap();
//! assert!(is_valid);
//! ```
//!
//! ## Batch Verify
//!
//! ```rust
//! use fd_ed25519::{Pubkey, Signature, batch_verify_single_message};
//!
//! let message = b"shared message";
//! let pubkeys = vec![/* */];
//! let signatures = vec![/* */];
//!
//! let all_valid = batch_verify_single_message(message, &pubkeys, &signatures).unwrap();
//! ```
//!
//! ## X25519
//!
//! ```rust
//! use fd_ed25519::{X25519PrivateKey, X25519PublicKey};
//!
//! let alice_private = X25519PrivateKey::from_bytes(&[1u8; 32]).unwrap();
//! let alice_public = alice_private.pubkey();
//!
//! let bob_private = X25519PrivateKey::from_bytes(&[2u8; 32]).unwrap();
//! let bob_public = bob_private.pubkey();
//!
//! // both parties compute the shared secret
//! let alice_shared = alice_private.exchange(&bob_public).unwrap();
//! let bob_shared = bob_private.exchange(&alice_public).unwrap();
//!
//! assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
//! ```

use fd_ed25519_sys as sys;
use std::fmt;
use std::mem::MaybeUninit;

pub const ED25519_SIGNATURE_SIZE: usize = 64;
pub const ED25519_PUBLIC_KEY_SIZE: usize = 32;
pub const ED25519_PRIVATE_KEY_SIZE: usize = 32;
pub const X25519_PUBLIC_KEY_SIZE: usize = 32;
pub const X25519_PRIVATE_KEY_SIZE: usize = 32;
pub const X25519_SHARED_SECRET_SIZE: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub enum Ed25519Error {
    /// Invalid signature format or content
    InvalidSignature(String),
    /// Invalid public key format or content
    InvalidPublicKey(String),
    /// Signature verification failed - message doesn't match
    VerificationFailed,
    /// Invalid input parameters
    InvalidInput(String),
    /// Internal cryptographic operation failed
    CryptoError(String),
    /// X25519 key exchange failed (e.g., low-order point)
    KeyExchangeFailed,
}

impl fmt::Display for Ed25519Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ed25519Error::InvalidSignature(msg) => write!(f, "Invalid signature: {}", msg),
            Ed25519Error::InvalidPublicKey(msg) => write!(f, "Invalid public key: {}", msg),
            Ed25519Error::VerificationFailed => write!(f, "Signature verification failed"),
            Ed25519Error::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Ed25519Error::CryptoError(msg) => write!(f, "Cryptographic error: {}", msg),
            Ed25519Error::KeyExchangeFailed => write!(f, "X25519 key exchange failed"),
        }
    }
}

impl std::error::Error for Ed25519Error {}

fn convert_ed25519_error(code: i32) -> Ed25519Error {
    match code {
        sys::FD_ED25519_ERR_SIG => {
            Ed25519Error::InvalidSignature("Invalid signature format".to_string())
        }
        sys::FD_ED25519_ERR_PUBKEY => {
            Ed25519Error::InvalidPublicKey("Invalid public key format".to_string())
        }
        sys::FD_ED25519_ERR_MSG => Ed25519Error::VerificationFailed,
        _ => Ed25519Error::CryptoError(format!("Unknown error code: {}", code)),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pubkey {
    bytes: [u8; ED25519_PUBLIC_KEY_SIZE],
}

impl Pubkey {
    pub fn from_bytes(bytes: &[u8; ED25519_PUBLIC_KEY_SIZE]) -> Result<Self, Ed25519Error> {
        Ok(unsafe { Self::from_bytes_unchecked(bytes) })
    }

    pub unsafe fn from_bytes_unchecked(bytes: &[u8; ED25519_PUBLIC_KEY_SIZE]) -> Self {
        Self { bytes: *bytes }
    }

    pub fn as_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_SIZE] {
        &self.bytes
    }

    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool, Ed25519Error> {
        unsafe {
            let mut sha = MaybeUninit::<sys::fd_sha512_t>::uninit();
            sys::fd_sha512_init(sha.as_mut_ptr());
            let mut sha = sha.assume_init();

            let result = sys::fd_ed25519_verify(
                message.as_ptr(),
                message.len() as u64,
                signature.bytes.as_ptr(),
                self.bytes.as_ptr(),
                &mut sha,
            );

            match result {
                r if r == sys::FD_ED25519_SUCCESS as i32 => Ok(true),
                sys::FD_ED25519_ERR_MSG => Ok(false),
                _ => Err(convert_ed25519_error(result)),
            }
        }
    }
}

impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl From<[u8; 32]> for Pubkey {
    fn from(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }
}

impl fmt::Debug for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pubkey")
            .field("bytes", &hex::encode(&self.bytes))
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Signature {
    bytes: [u8; ED25519_SIGNATURE_SIZE],
}

impl Signature {
    pub fn from_bytes(bytes: &[u8; ED25519_SIGNATURE_SIZE]) -> Result<Self, Ed25519Error> {
        Ok(Self { bytes: *bytes })
    }

    pub fn as_bytes(&self) -> &[u8; ED25519_SIGNATURE_SIZE] {
        &self.bytes
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signature")
            .field("bytes", &hex::encode(&self.bytes[..8]))
            .field("truncated", &"...")
            .finish()
    }
}

pub struct Keypair {
    secret_key: [u8; ED25519_PRIVATE_KEY_SIZE],
    pubkey: Pubkey,
}

impl Keypair {
    pub fn from_secret_key(
        secret_key: &[u8; ED25519_PRIVATE_KEY_SIZE],
    ) -> Result<Self, Ed25519Error> {
        unsafe {
            let mut sha = MaybeUninit::<sys::fd_sha512_t>::uninit();
            sys::fd_sha512_init(sha.as_mut_ptr());
            let mut sha = sha.assume_init();
            let mut pubkey_bytes = [0u8; ED25519_PUBLIC_KEY_SIZE];
            let result = sys::fd_ed25519_public_from_private(
                pubkey_bytes.as_mut_ptr(),
                secret_key.as_ptr(),
                &mut sha,
            );

            if result.is_null() {
                return Err(Ed25519Error::CryptoError(
                    "Failed to derive public key".to_string(),
                ));
            }

            Ok(Self {
                secret_key: *secret_key,
                pubkey: Pubkey {
                    bytes: pubkey_bytes,
                },
            })
        }
    }

    pub fn pubkey(&self) -> &Pubkey {
        &self.pubkey
    }

    pub fn secret_key(&self) -> &[u8; ED25519_PRIVATE_KEY_SIZE] {
        &self.secret_key
    }

    pub fn sign(&self, message: &[u8]) -> Result<Signature, Ed25519Error> {
        unsafe {
            // Initialize SHA512 calculator
            let mut sha = MaybeUninit::<sys::fd_sha512_t>::uninit();
            sys::fd_sha512_init(sha.as_mut_ptr());
            let mut sha = sha.assume_init();

            let mut signature_bytes = [0u8; ED25519_SIGNATURE_SIZE];
            let result = sys::fd_ed25519_sign(
                signature_bytes.as_mut_ptr(),
                message.as_ptr(),
                message.len() as u64,
                self.pubkey.bytes.as_ptr(),
                self.secret_key.as_ptr(),
                &mut sha,
            );

            if result.is_null() {
                return Err(Ed25519Error::CryptoError(
                    "Failed to sign message".to_string(),
                ));
            }

            Ok(Signature {
                bytes: signature_bytes,
            })
        }
    }
}

impl fmt::Debug for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Keypair")
            .field("pubkey", &self.pubkey)
            .field("secret_key", &"<redacted>")
            .finish()
    }
}

pub fn batch_verify_single_message(
    message: &[u8],
    pubkeys: &[Pubkey],
    signatures: &[Signature],
) -> Result<bool, Ed25519Error> {
    if pubkeys.len() != signatures.len() {
        return Err(Ed25519Error::InvalidInput(
            "Number of public keys must match number of signatures".to_string(),
        ));
    }

    if pubkeys.is_empty() {
        return Ok(true);
    }

    unsafe {
        let batch_size = pubkeys.len() as u8;
        let mut signature_bytes = Vec::with_capacity(signatures.len() * ED25519_SIGNATURE_SIZE);
        let mut pubkey_bytes = Vec::with_capacity(pubkeys.len() * ED25519_PUBLIC_KEY_SIZE);
        let mut shas = Vec::with_capacity(pubkeys.len());

        for signature in signatures {
            signature_bytes.extend_from_slice(&signature.bytes);
        }

        for pubkey in pubkeys {
            pubkey_bytes.extend_from_slice(&pubkey.bytes);
        }

        for _ in 0..pubkeys.len() {
            let mut sha = MaybeUninit::<sys::fd_sha512_t>::uninit();
            sys::fd_sha512_init(sha.as_mut_ptr());
            shas.push(Box::into_raw(Box::new(sha.assume_init())));
        }

        let result = sys::fd_ed25519_verify_batch_single_msg(
            message.as_ptr(),
            message.len() as u64,
            signature_bytes.as_ptr(),
            pubkey_bytes.as_ptr(),
            shas.as_mut_ptr(),
            batch_size,
        );

        for sha_ptr in shas {
            let _ = Box::from_raw(sha_ptr);
        }

        match result {
            r if r == sys::FD_ED25519_SUCCESS as i32 => Ok(true),
            sys::FD_ED25519_ERR_MSG => Ok(false),
            _ => Err(convert_ed25519_error(result)),
        }
    }
}

/// X25519 secret key for a Diffie-Hellman key exchange
pub struct X25519PrivateKey {
    bytes: [u8; X25519_PRIVATE_KEY_SIZE],
}

impl X25519PrivateKey {
    pub fn from_bytes(bytes: &[u8; X25519_PRIVATE_KEY_SIZE]) -> Result<Self, Ed25519Error> {
        Ok(Self { bytes: *bytes })
    }

    pub fn pubkey(&self) -> X25519PublicKey {
        unsafe {
            let mut pubkey_bytes = [0u8; X25519_PUBLIC_KEY_SIZE];
            sys::fd_x25519_public(pubkey_bytes.as_mut_ptr(), self.bytes.as_ptr());
            X25519PublicKey {
                bytes: pubkey_bytes,
            }
        }
    }

    pub fn exchange(
        &self,
        peer_pubkey: &X25519PublicKey,
    ) -> Result<X25519SharedSecret, Ed25519Error> {
        unsafe {
            let mut shared_secret_bytes = [0u8; X25519_SHARED_SECRET_SIZE];
            let result = sys::fd_x25519_exchange(
                shared_secret_bytes.as_mut_ptr(),
                self.bytes.as_ptr(),
                peer_pubkey.bytes.as_ptr(),
            );

            if result.is_null() {
                return Err(Ed25519Error::KeyExchangeFailed);
            }

            Ok(X25519SharedSecret {
                bytes: shared_secret_bytes,
            })
        }
    }

    pub fn as_bytes(&self) -> &[u8; X25519_PRIVATE_KEY_SIZE] {
        &self.bytes
    }
}

impl fmt::Debug for X25519PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("X25519PrivateKey")
            .field("bytes", &"<redacted>")
            .finish()
    }
}

/// X25519 pubkey for a Diffie-Hellman key exchange
#[derive(Clone, PartialEq, Eq)]
pub struct X25519PublicKey {
    bytes: [u8; X25519_PUBLIC_KEY_SIZE],
}

impl X25519PublicKey {
    pub fn from_bytes(bytes: &[u8; X25519_PUBLIC_KEY_SIZE]) -> Result<Self, Ed25519Error> {
        Ok(Self { bytes: *bytes })
    }

    pub fn as_bytes(&self) -> &[u8; X25519_PUBLIC_KEY_SIZE] {
        &self.bytes
    }
}

impl fmt::Debug for X25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("X25519PublicKey")
            .field("bytes", &hex::encode(&self.bytes))
            .finish()
    }
}

/// X25519 shared secret from a Diffie-Hellman key exchange
pub struct X25519SharedSecret {
    bytes: [u8; X25519_SHARED_SECRET_SIZE],
}

impl X25519SharedSecret {
    pub fn as_bytes(&self) -> &[u8; X25519_SHARED_SECRET_SIZE] {
        &self.bytes
    }
}

impl fmt::Debug for X25519SharedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("X25519SharedSecret")
            .field("bytes", &"<redacted>")
            .finish()
    }
}

impl Drop for X25519SharedSecret {
    fn drop(&mut self) {
        self.bytes.fill(0);
    }
}

mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keygen() {
        let secret_key = [1u8; 32];
        let keypair = Keypair::from_secret_key(&secret_key).unwrap();
        let keypair2 = Keypair::from_secret_key(&secret_key).unwrap();
        assert_eq!(keypair.pubkey(), keypair2.pubkey());
    }

    #[test]
    fn test_sigverify() {
        let secret_key = [42u8; 32];
        let keypair = Keypair::from_secret_key(&secret_key).unwrap();

        let message = b"Hello, Ed25519!";
        let signature = keypair.sign(message).unwrap();

        assert!(keypair.pubkey().verify(message, &signature).unwrap());

        let wrong_message = b"Hello, Ed25518!";
        assert!(!keypair.pubkey().verify(wrong_message, &signature).unwrap());
    }

    #[test]
    fn test_empty_msg() {
        let secret_key = [99u8; 32];
        let keypair = Keypair::from_secret_key(&secret_key).unwrap();

        let empty_message = b"";
        let signature = keypair.sign(empty_message).unwrap();

        assert!(keypair.pubkey().verify(empty_message, &signature).unwrap());
    }

    #[test]
    fn test_batch_verify() {
        let message = b"shared message for batch verification";
        let keypairs: Vec<_> = (0..3)
            .map(|i| {
                let mut secret_key = [0u8; 32];
                secret_key[0] = i as u8 + 1;
                Keypair::from_secret_key(&secret_key).unwrap()
            })
            .collect();

        let signatures: Result<Vec<_>, _> = keypairs.iter().map(|kp| kp.sign(message)).collect();
        let signatures = signatures.unwrap();

        let pubkeys: Vec<_> = keypairs.iter().map(|kp| kp.pubkey().clone()).collect();

        assert!(batch_verify_single_message(message, &pubkeys, &signatures).unwrap());

        let wrong_message = b"wrong message";
        assert!(!batch_verify_single_message(wrong_message, &pubkeys, &signatures).unwrap());
    }

    #[test]
    fn test_x25519_exchange() {
        let alice_private = X25519PrivateKey::from_bytes(&[1u8; 32]).unwrap();
        let alice_public = alice_private.pubkey();

        let bob_private = X25519PrivateKey::from_bytes(&[2u8; 32]).unwrap();
        let bob_public = bob_private.pubkey();

        // both parties compute the shared secret
        let alice_shared = alice_private.exchange(&bob_public).unwrap();
        let bob_shared = bob_private.exchange(&alice_public).unwrap();

        // shared secrets should be identical
        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
        assert_ne!(alice_shared.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_same_key_exch() {
        let secret_key = [42u8; 32];
        let alice_private = X25519PrivateKey::from_bytes(&secret_key).unwrap();
        let alice_public = alice_private.pubkey();
        let bob_private = X25519PrivateKey::from_bytes(&secret_key).unwrap();
        let bob_public = bob_private.pubkey();
        let alice_shared = alice_private.exchange(&bob_public).unwrap();
        let bob_shared = bob_private.exchange(&alice_public).unwrap();

        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_error_handling() {
        let message = b"test message";
        let pubkeys = vec![Pubkey::from_bytes(&[1u8; 32]).unwrap()];
        let signatures = vec![
            Signature::from_bytes(&[0u8; 64]).unwrap(),
            Signature::from_bytes(&[1u8; 64]).unwrap(),
        ];

        let result = batch_verify_single_message(message, &pubkeys, &signatures);
        assert!(matches!(result, Err(Ed25519Error::InvalidInput(_))));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let secret_key = [123u8; 32];
        let keypair = Keypair::from_secret_key(&secret_key).unwrap();
        let pubkey_bytes = keypair.pubkey().as_bytes();
        let recovered_pubkey = Pubkey::from_bytes(pubkey_bytes).unwrap();
        assert_eq!(keypair.pubkey(), &recovered_pubkey);

        let message = b"test message";
        let signature = keypair.sign(message).unwrap();
        let signature_bytes = signature.as_bytes();
        let recovered_signature = Signature::from_bytes(signature_bytes).unwrap();
        assert_eq!(signature, recovered_signature);

        assert!(recovered_pubkey
            .verify(message, &recovered_signature)
            .unwrap());
    }
}
