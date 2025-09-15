//! Safe API for `fd_ed25519_sys`.

use core::fmt;
use core::mem::MaybeUninit;
use fd_ed25519_sys as sys;

pub const ED25519_SIGNATURE_SIZE: usize = 64;
pub const ED25519_PUBLIC_KEY_SIZE: usize = 32;
pub const ED25519_PRIVATE_KEY_SIZE: usize = 32;
pub const X25519_PUBLIC_KEY_SIZE: usize = 32;
pub const X25519_PRIVATE_KEY_SIZE: usize = 32;
pub const X25519_SHARED_SECRET_SIZE: usize = 32;

pub const MAX_SEED_LEN: usize = 32;
pub const MAX_SEEDS: usize = 16;
const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";

#[derive(Debug, Clone, PartialEq)]
pub enum Ed25519Error {
    InvalidSignature(&'static str),
    InvalidPublicKey(&'static str),
    VerificationFailed,
    InvalidInput(&'static str),
    CryptoError(&'static str),
    DerivationFailed,
    MaxSeedLengthExceeded,
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
            Ed25519Error::DerivationFailed => write!(f, "Program address derivation failed"),
            Ed25519Error::MaxSeedLengthExceeded => write!(f, "Maximum seed length exceeded"),
        }
    }
}

impl core::error::Error for Ed25519Error {}

fn convert_ed25519_error(code: i32) -> Ed25519Error {
    match code {
        sys::FD_ED25519_ERR_SIG => Ed25519Error::InvalidSignature("Invalid signature format"),
        sys::FD_ED25519_ERR_PUBKEY => Ed25519Error::InvalidPublicKey("Invalid public key format"),
        sys::FD_ED25519_ERR_MSG => Ed25519Error::VerificationFailed,
        _ => Ed25519Error::CryptoError("Unknown error occurred"),
    }
}

fn bytes_are_curve_point(bytes: [u8; 32]) -> bool {
    unsafe {
        let mut point = MaybeUninit::<sys::fd_ed25519_point_t>::uninit();
        let result = sys::fd_ed25519_point_frombytes(point.as_mut_ptr(), bytes.as_ptr());

        if result.is_null() {
            return false;
        }

        let point = point.assume_init();
        let mut encoded = [0u8; 32];
        sys::fd_ed25519_point_tobytes(encoded.as_mut_ptr(), &point);
        encoded == bytes
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

    /// # Safety
    /// It's up to the caller to ensure the bytes represent a valid public key, and
    /// are properly aligned.
    pub unsafe fn from_bytes_unchecked(bytes: &[u8; ED25519_PUBLIC_KEY_SIZE]) -> Self {
        Self { bytes: *bytes }
    }

    pub fn as_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_SIZE] {
        &self.bytes
    }

    pub fn is_on_curve(&self) -> bool {
        bytes_are_curve_point(self.bytes)
    }

    /// Iterates over a set of seeds to create a valid Program Derived Address.
    ///
    /// Program Derived Addresses are not valid Ed25519 public keys, so this function
    /// guarantees to find an address off-curve by appending a "bump" seed, and iterating downward
    /// from `u8::MAX` until an off-curve address is found.
    pub fn find_program_address(
        seeds: &[&[u8]],
        program_id: &Pubkey,
    ) -> Result<(Pubkey, u8), Ed25519Error> {
        let mut bump_seed = [u8::MAX];
        for _ in 0..u8::MAX {
            {
                let mut seeds_with_bump = seeds.to_vec();
                seeds_with_bump.push(&bump_seed);
                match Self::create_program_address(&seeds_with_bump, program_id) {
                    Ok(address) => return Ok((address, bump_seed[0])),
                    Err(Ed25519Error::InvalidInput(_)) => (),
                    Err(e) => return Err(e),
                }
            }
            bump_seed[0] -= 1;
        }
        Err(Ed25519Error::DerivationFailed)
    }

    /// Create a program address with the given seeds and program ID. This is faster
    /// than `find_program_address` but only guarantees a valid address as long as
    /// the seeds & bump are valid with a single iteration.
    ///
    /// If this hasn't been precomputed, use `find_program_address` instead.
    pub fn create_program_address(
        seeds: &[&[u8]],
        program_id: &Pubkey,
    ) -> Result<Pubkey, Ed25519Error> {
        if seeds.len() > MAX_SEEDS {
            return Err(Ed25519Error::MaxSeedLengthExceeded);
        }
        for seed in seeds.iter() {
            if seed.len() > MAX_SEED_LEN {
                return Err(Ed25519Error::MaxSeedLengthExceeded);
            }
        }

        unsafe {
            let mut sha = MaybeUninit::<sys::fd_sha256_t>::uninit();
            sys::fd_sha256_init(sha.as_mut_ptr());
            let mut sha = sha.assume_init();

            for seed in seeds.iter() {
                sys::fd_sha256_append(
                    &mut sha,
                    seed.as_ptr() as *const ::std::os::raw::c_void,
                    seed.len() as u64,
                );
            }

            sys::fd_sha256_append(
                &mut sha,
                program_id.as_ref().as_ptr() as *const ::std::os::raw::c_void,
                program_id.as_ref().len() as u64,
            );

            sys::fd_sha256_append(
                &mut sha,
                PDA_MARKER.as_ptr() as *const ::std::os::raw::c_void,
                PDA_MARKER.len() as u64,
            );

            let mut hash = [0u8; 32];
            sys::fd_sha256_fini(&mut sha, hash.as_mut_ptr() as *mut ::std::os::raw::c_void);

            if bytes_are_curve_point(hash) {
                return Err(Ed25519Error::InvalidInput("Provided seeds are invalid"));
            }

            Ok(Pubkey::from(hash))
        }
    }

    /// `create_program_address` with no checks on seed length bounds. Exceeeding
    /// these bounds may result in an invalid address or undefined behavior
    pub fn create_program_address_unchecked(
        seeds: &[&[u8]],
        program_id: &Pubkey,
    ) -> Result<Pubkey, Ed25519Error> {
        unsafe {
            let mut sha = MaybeUninit::<sys::fd_sha256_t>::uninit();
            sys::fd_sha256_init(sha.as_mut_ptr());
            let mut sha = sha.assume_init();

            for seed in seeds.iter() {
                sys::fd_sha256_append(
                    &mut sha,
                    seed.as_ptr() as *const ::std::os::raw::c_void,
                    seed.len() as u64,
                );
            }

            sys::fd_sha256_append(
                &mut sha,
                program_id.as_ref().as_ptr() as *const ::std::os::raw::c_void,
                program_id.as_ref().len() as u64,
            );

            sys::fd_sha256_append(
                &mut sha,
                PDA_MARKER.as_ptr() as *const ::std::os::raw::c_void,
                PDA_MARKER.len() as u64,
            );

            let mut hash = [0u8; 32];
            sys::fd_sha256_fini(&mut sha, hash.as_mut_ptr() as *mut ::std::os::raw::c_void);

            if bytes_are_curve_point(hash) {
                return Err(Ed25519Error::InvalidInput("Provided seeds are invalid"));
            }

            Ok(Pubkey::from(hash))
        }
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
                return Err(Ed25519Error::CryptoError("Failed to derive public key"));
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
                return Err(Ed25519Error::CryptoError("Failed to sign message"));
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
            "Number of public keys must match number of signatures",
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

    #[test]
    fn test_create_program_address() {
        let program_id = Pubkey::from([1u8; 32]);
        let seeds = [&b"hello"[..], &b"world"[..]];

        let result = Pubkey::create_program_address(&seeds, &program_id);
        assert!(result.is_ok());

        let address = result.unwrap();
        assert!(!address.is_on_curve());
    }

    #[test]
    fn test_find_program_address() {
        let program_id = Pubkey::from([42u8; 32]);
        let seeds = [&b"other_test"[..], &b"some_seed"[..]];

        let (address1, bump1) = Pubkey::find_program_address(&seeds, &program_id).unwrap();
        let (address2, bump2) = Pubkey::find_program_address(&seeds, &program_id).unwrap();

        assert_eq!(address1, address2);
        assert_eq!(bump1, bump2);

        println!("Found PDA: {:?} with bump {}", address1.as_bytes(), bump1);

        let mut seeds_with_bump = seeds.to_vec();
        let bump_seed = &[bump1];
        seeds_with_bump.push(bump_seed);
        let recreated = Pubkey::create_program_address(&seeds_with_bump, &program_id).unwrap();
        assert_eq!(recreated, address1);
    }
}
