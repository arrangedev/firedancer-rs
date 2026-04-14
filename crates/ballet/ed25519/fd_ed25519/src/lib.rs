//! Safe API for `fd_ed25519_sys`.

use core::ffi::c_void;
use core::fmt;
use core::mem::MaybeUninit;
use fd_ed25519_sys as sys;

pub const PUBKEY_BYTES: usize = 32;
pub const SIGNATURE_BYTES: usize = 64;

pub(crate) const ED25519_SIGNATURE_SIZE: usize = 64;
pub(crate) const ED25519_PUBLIC_KEY_SIZE: usize = 32;
pub(crate) const ED25519_PRIVATE_KEY_SIZE: usize = 32;
pub(crate) const X25519_PUBLIC_KEY_SIZE: usize = 32;
pub(crate) const X25519_PRIVATE_KEY_SIZE: usize = 32;
pub(crate) const X25519_SHARED_SECRET_SIZE: usize = 32;

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

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wincode", derive(wincode::SchemaRead, wincode::SchemaWrite))]
pub struct Pubkey([u8; ED25519_PUBLIC_KEY_SIZE]);

impl Pubkey {
    pub const fn from_bytes(bytes: &[u8; ED25519_PUBLIC_KEY_SIZE]) -> Self {
        Self(*bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_SIZE] {
        &self.0
    }

    pub fn is_on_curve(&self) -> bool {
        bytes_are_curve_point(self.0)
    }

    pub fn to_hex(&self) -> String {
        crate::hex::encode(&self.0)
    }

    pub fn from_hex(hex_str: &str) -> Result<Self, Ed25519Error> {
        let bytes = crate::hex::decode(hex_str)?;
        if bytes.len() != ED25519_PUBLIC_KEY_SIZE {
            return Err(Ed25519Error::InvalidInput(
                "Invalid hex length for public key",
            ));
        }
        let mut key_bytes = [0u8; ED25519_PUBLIC_KEY_SIZE];
        key_bytes.copy_from_slice(&bytes);
        Ok(Self(key_bytes))
    }

    #[cfg(feature = "base58")]
    pub fn to_base58(&self) -> String {
        crate::base58::encode_32(&self.0)
    }

    #[cfg(feature = "base58")]
    pub fn from_base58(encoded: &str) -> Result<Self, Ed25519Error> {
        let bytes = crate::base58::decode_32(encoded)?;
        Ok(Self(bytes))
    }

    #[cfg(feature = "base58")]
    pub const fn from_base58_const(encoded: &str) -> Self {
        Self(five8_const::decode_32_const(encoded))
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
        if seeds.len() > MAX_SEEDS {
            return Err(Ed25519Error::MaxSeedLengthExceeded);
        }
        for seed in seeds.iter() {
            if seed.len() > MAX_SEED_LEN {
                return Err(Ed25519Error::MaxSeedLengthExceeded);
            }
        }

        unsafe {
            let mut base_sha = MaybeUninit::<sys::fd_sha256_t>::uninit();
            sys::fd_sha256_init(base_sha.as_mut_ptr());
            let base_sha = base_sha.assume_init();

            let base_sha = seeds.iter().fold(base_sha, |mut sha, seed| {
                sys::fd_sha256_append(&mut sha, seed.as_ptr() as *const c_void, seed.len() as u64);
                sha
            });

            let mut bump = u8::MAX;
            for _ in 0..u8::MAX {
                let mut sha = core::ptr::read(&base_sha);

                sys::fd_sha256_append(&mut sha, &bump as *const u8 as *const c_void, 1);
                sys::fd_sha256_append(
                    &mut sha,
                    program_id.as_ref().as_ptr() as *const c_void,
                    ED25519_PUBLIC_KEY_SIZE as u64,
                );
                sys::fd_sha256_append(
                    &mut sha,
                    PDA_MARKER.as_ptr() as *const c_void,
                    PDA_MARKER.len() as u64,
                );

                let mut hash = [0u8; 32];
                sys::fd_sha256_fini(&mut sha, hash.as_mut_ptr() as *mut c_void);

                if sys::fd_ed25519_point_is_on_curve(hash.as_ptr()) == 0 {
                    return Ok((Pubkey::from(hash), bump));
                }

                bump -= 1;
            }

            Err(Ed25519Error::DerivationFailed)
        }
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
                sys::fd_sha256_append(&mut sha, seed.as_ptr() as *const c_void, seed.len() as u64);
            }

            sys::fd_sha256_append(
                &mut sha,
                program_id.as_ref().as_ptr() as *const c_void,
                program_id.as_ref().len() as u64,
            );

            sys::fd_sha256_append(
                &mut sha,
                PDA_MARKER.as_ptr() as *const c_void,
                PDA_MARKER.len() as u64,
            );

            let mut hash = [0u8; 32];
            sys::fd_sha256_fini(&mut sha, hash.as_mut_ptr() as *mut c_void);

            if bytes_are_curve_point(hash) {
                return Err(Ed25519Error::InvalidInput("Provided seeds are invalid"));
            }

            Ok(Pubkey::from(hash))
        }
    }

    /// `create_program_address` with no checks on seed length bounds. Exceeeding
    /// these bounds may result in an invalid address or undefined behavior
    pub unsafe fn create_program_address_unchecked(
        seeds: &[&[u8]],
        program_id: &Pubkey,
    ) -> Result<Pubkey, Ed25519Error> {
        let mut sha = MaybeUninit::<sys::fd_sha256_t>::uninit();
        sys::fd_sha256_init(sha.as_mut_ptr());
        let mut sha = sha.assume_init();

        for seed in seeds.iter() {
            sys::fd_sha256_append(&mut sha, seed.as_ptr() as *const c_void, seed.len() as u64);
        }

        sys::fd_sha256_append(
            &mut sha,
            program_id.as_ref().as_ptr() as *const c_void,
            program_id.as_ref().len() as u64,
        );

        sys::fd_sha256_append(
            &mut sha,
            PDA_MARKER.as_ptr() as *const c_void,
            PDA_MARKER.len() as u64,
        );

        let mut hash = [0u8; 32];
        sys::fd_sha256_fini(&mut sha, hash.as_mut_ptr() as *mut c_void);

        if bytes_are_curve_point(hash) {
            return Err(Ed25519Error::InvalidInput("Provided seeds are invalid"));
        }

        Ok(Pubkey::from(hash))
    }

    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool, Ed25519Error> {
        unsafe {
            let mut sha = MaybeUninit::<sys::fd_sha512_t>::uninit();
            sys::fd_sha512_init(sha.as_mut_ptr());
            let mut sha = sha.assume_init();

            let result = sys::fd_ed25519_verify(
                message.as_ptr(),
                message.len() as u64,
                signature.as_ref().as_ptr(),
                self.as_ref().as_ptr(),
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

impl AsRef<[u8; ED25519_PUBLIC_KEY_SIZE]> for Pubkey {
    fn as_ref(&self) -> &[u8; ED25519_PUBLIC_KEY_SIZE] {
        &self.0
    }
}

impl From<&[u8]> for Pubkey {
    fn from(bytes: &[u8]) -> Self {
        let mut buf = [0u8; ED25519_PUBLIC_KEY_SIZE];
        buf.copy_from_slice(bytes);
        Self(buf)
    }
}

impl From<[u8; ED25519_PUBLIC_KEY_SIZE]> for Pubkey {
    fn from(bytes: [u8; ED25519_PUBLIC_KEY_SIZE]) -> Self {
        Self(bytes)
    }
}

impl From<&[u8; ED25519_PUBLIC_KEY_SIZE]> for Pubkey {
    fn from(bytes: &[u8; ED25519_PUBLIC_KEY_SIZE]) -> Self {
        Self(*bytes)
    }
}

impl Default for Pubkey {
    fn default() -> Self {
        Self([0u8; ED25519_PUBLIC_KEY_SIZE])
    }
}

impl fmt::Debug for Pubkey {
    #[cfg(feature = "base58")]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base58())
    }

    #[cfg(not(feature = "base58"))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pubkey")
            .field("bytes", &hex::encode(&self.0))
            .finish()
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "wincode", derive(wincode::SchemaRead, wincode::SchemaWrite))]
pub struct Signature([u8; ED25519_SIGNATURE_SIZE]);

impl Signature {
    pub const fn from_bytes(bytes: &[u8; ED25519_SIGNATURE_SIZE]) -> Self {
        Self(*bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; ED25519_SIGNATURE_SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    pub fn from_hex(hex_str: &str) -> Result<Self, Ed25519Error> {
        let bytes = hex::decode(hex_str)?;
        if bytes.len() != ED25519_SIGNATURE_SIZE {
            return Err(Ed25519Error::InvalidInput(
                "Invalid hex length for signature",
            ));
        }
        let mut sig_bytes = [0u8; ED25519_SIGNATURE_SIZE];
        sig_bytes.copy_from_slice(&bytes);
        Ok(Self(sig_bytes))
    }

    #[cfg(feature = "base58")]
    pub fn to_base58(&self) -> String {
        base58::encode_64(&self.0)
    }

    #[cfg(feature = "base58")]
    pub fn from_base58(encoded: &str) -> Result<Self, Ed25519Error> {
        let bytes = base58::decode_64(encoded)?;
        Ok(Self(bytes))
    }
}

impl fmt::Debug for Signature {
    #[cfg(feature = "base58")]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base58())
    }

    #[cfg(not(feature = "base58"))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signature")
            .field("bytes", &hex::encode(&self.0[..8]))
            .field("truncated", &"...")
            .finish()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

impl AsRef<[u8; ED25519_SIGNATURE_SIZE]> for Signature {
    fn as_ref(&self) -> &[u8; ED25519_SIGNATURE_SIZE] {
        &self.0
    }
}

impl From<[u8; ED25519_SIGNATURE_SIZE]> for Signature {
    fn from(bytes: [u8; ED25519_SIGNATURE_SIZE]) -> Self {
        Self(bytes)
    }
}

impl From<&[u8; ED25519_SIGNATURE_SIZE]> for Signature {
    fn from(bytes: &[u8; ED25519_SIGNATURE_SIZE]) -> Self {
        Self(*bytes)
    }
}

impl From<&[u8]> for Signature {
    fn from(bytes: &[u8]) -> Self {
        let mut buf = [0u8; ED25519_SIGNATURE_SIZE];
        buf.copy_from_slice(bytes);
        Self(buf)
    }
}

#[repr(C)]
pub struct Keypair([u8; ED25519_PRIVATE_KEY_SIZE], Pubkey);

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

            Ok(Self(*secret_key, Pubkey(pubkey_bytes)))
        }
    }

    pub fn pubkey(&self) -> &Pubkey {
        &self.1
    }

    pub fn secret_key(&self) -> &[u8; ED25519_PRIVATE_KEY_SIZE] {
        &self.0
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
                self.pubkey().as_ref().as_ptr(),
                self.secret_key().as_ptr(),
                &mut sha,
            );

            if result.is_null() {
                return Err(Ed25519Error::CryptoError("Failed to sign message"));
            }

            Ok(Signature(signature_bytes))
        }
    }
}

impl fmt::Debug for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Keypair")
            .field("pubkey", &self.pubkey())
            .field("secret_key", &"<redacted>")
            .finish()
    }
}

impl AsRef<[u8; ED25519_PRIVATE_KEY_SIZE]> for Keypair {
    fn as_ref(&self) -> &[u8; ED25519_PRIVATE_KEY_SIZE] {
        &self.0
    }
}

impl From<&[u8]> for Keypair {
    fn from(bytes: &[u8]) -> Self {
        let mut buf = [0u8; ED25519_PRIVATE_KEY_SIZE];
        buf.copy_from_slice(bytes);
        Self::from_secret_key(&buf).expect("Failed to create keypair from bytes")
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
            signature_bytes.extend_from_slice(signature.as_ref());
        }

        for pubkey in pubkeys {
            pubkey_bytes.extend_from_slice(pubkey.as_ref());
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

#[repr(transparent)]
pub struct X25519PrivateKey([u8; X25519_PRIVATE_KEY_SIZE]);

impl X25519PrivateKey {
    pub fn from_bytes(bytes: &[u8; X25519_PRIVATE_KEY_SIZE]) -> Result<Self, Ed25519Error> {
        Ok(Self(*bytes))
    }

    pub fn pubkey(&self) -> X25519PublicKey {
        unsafe {
            let mut pubkey_bytes = [0u8; X25519_PUBLIC_KEY_SIZE];
            sys::fd_x25519_public(pubkey_bytes.as_mut_ptr(), self.as_ref().as_ptr());
            X25519PublicKey(pubkey_bytes)
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
                self.as_ref().as_ptr(),
                peer_pubkey.as_ref().as_ptr(),
            );

            if result.is_null() {
                return Err(Ed25519Error::KeyExchangeFailed);
            }

            Ok(X25519SharedSecret(shared_secret_bytes))
        }
    }

    pub fn as_bytes(&self) -> &[u8; X25519_PRIVATE_KEY_SIZE] {
        &self.0
    }
}

impl fmt::Debug for X25519PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("X25519PrivateKey")
            .field("bytes", &"<redacted>")
            .finish()
    }
}

impl AsRef<[u8; X25519_PRIVATE_KEY_SIZE]> for X25519PrivateKey {
    fn as_ref(&self) -> &[u8; X25519_PRIVATE_KEY_SIZE] {
        &self.0
    }
}

impl From<[u8; X25519_PRIVATE_KEY_SIZE]> for X25519PrivateKey {
    fn from(bytes: [u8; X25519_PRIVATE_KEY_SIZE]) -> Self {
        Self(bytes)
    }
}

#[repr(transparent)]
#[derive(Clone, PartialEq, Eq)]
pub struct X25519PublicKey([u8; X25519_PUBLIC_KEY_SIZE]);

impl X25519PublicKey {
    pub fn from_bytes(bytes: &[u8; X25519_PUBLIC_KEY_SIZE]) -> Result<Self, Ed25519Error> {
        Ok(Self(*bytes))
    }

    pub fn as_bytes(&self) -> &[u8; X25519_PUBLIC_KEY_SIZE] {
        &self.0
    }
}

impl fmt::Debug for X25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("X25519PublicKey")
            .field("bytes", &hex::encode(&self.0))
            .finish()
    }
}

impl AsRef<[u8]> for X25519PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; X25519_PUBLIC_KEY_SIZE]> for X25519PublicKey {
    fn from(bytes: [u8; X25519_PUBLIC_KEY_SIZE]) -> Self {
        Self(bytes)
    }
}

#[repr(transparent)]
pub struct X25519SharedSecret([u8; X25519_SHARED_SECRET_SIZE]);

impl X25519SharedSecret {
    pub fn as_bytes(&self) -> &[u8; X25519_SHARED_SECRET_SIZE] {
        &self.0
    }
}

impl Drop for X25519SharedSecret {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

impl AsRef<[u8; X25519_SHARED_SECRET_SIZE]> for X25519SharedSecret {
    fn as_ref(&self) -> &[u8; X25519_SHARED_SECRET_SIZE] {
        &self.0
    }
}

impl From<[u8; X25519_SHARED_SECRET_SIZE]> for X25519SharedSecret {
    fn from(bytes: [u8; X25519_SHARED_SECRET_SIZE]) -> Self {
        Self(bytes)
    }
}

#[cfg(feature = "base58")]
pub mod base58 {
    use crate::Ed25519Error;
    use fd_ed25519_sys as sys;

    pub const ENCODED_32_LEN: usize = sys::FD_BASE58_ENCODED_32_LEN as usize;
    pub const ENCODED_64_LEN: usize = sys::FD_BASE58_ENCODED_64_LEN as usize;

    #[macro_export]
    macro_rules! pubkey {
        ($str:expr) => {
            $crate::Pubkey::from_base58_const($str)
        };
    }

    pub fn encode_32(bytes: &[u8; 32]) -> String {
        unsafe {
            let mut out = [0u8; sys::FD_BASE58_ENCODED_32_SZ as usize];
            let mut len = 0usize;
            let result = sys::fd_base58_encode_32(
                bytes.as_ptr(),
                &mut len as *mut usize as *mut u64,
                out.as_mut_ptr() as *mut i8,
            );
            if result.is_null() {
                panic!("Base58 encoding failed");
            }
            std::str::from_utf8_unchecked(&out[..len]).to_string()
        }
    }

    pub fn encode_64(bytes: &[u8; 64]) -> String {
        unsafe {
            let mut out = [0u8; sys::FD_BASE58_ENCODED_64_SZ as usize];
            let mut len = 0usize;
            let result = sys::fd_base58_encode_64(
                bytes.as_ptr(),
                &mut len as *mut usize as *mut u64,
                out.as_mut_ptr() as *mut i8,
            );
            if result.is_null() {
                panic!("Base58 encoding failed");
            }
            core::str::from_utf8_unchecked(&out[..len]).to_string()
        }
    }

    pub fn decode_32(encoded: &str) -> Result<[u8; 32], Ed25519Error> {
        unsafe {
            let mut out = [0u8; 32];
            let c_str = std::ffi::CString::new(encoded)
                .map_err(|_| Ed25519Error::InvalidInput("Invalid base58 string"))?;
            let result = sys::fd_base58_decode_32(c_str.as_ptr(), out.as_mut_ptr());
            if result.is_null() {
                return Err(Ed25519Error::InvalidInput("Failed to decode base58"));
            }
            Ok(out)
        }
    }

    pub fn decode_64(encoded: &str) -> Result<[u8; 64], Ed25519Error> {
        unsafe {
            let mut out = [0u8; 64];
            let c_str = std::ffi::CString::new(encoded)
                .map_err(|_| Ed25519Error::InvalidInput("Invalid base58 string"))?;
            let result = sys::fd_base58_decode_64(c_str.as_ptr(), out.as_mut_ptr());
            if result.is_null() {
                return Err(Ed25519Error::InvalidInput("Failed to decode base58"));
            }
            Ok(out)
        }
    }
}

pub mod hex {
    use crate::Ed25519Error;
    use fd_ed25519_sys as sys;

    pub fn encode(bytes: &[u8]) -> String {
        unsafe {
            let mut out = vec![0u8; bytes.len() * 2 + 1]; // null terminator, +1
            let result = sys::fd_hex_encode(
                out.as_mut_ptr() as *mut i8,
                bytes.as_ptr() as *const core::ffi::c_void,
                bytes.len() as u64,
            );
            if result.is_null() {
                panic!("Hex encoding failed");
            }

            let len = out.iter().position(|&b| b == 0).unwrap_or(out.len());
            core::str::from_utf8_unchecked(&out[..len]).to_string()
        }
    }

    pub fn decode(hex_str: &str) -> Result<Vec<u8>, Ed25519Error> {
        if hex_str.len() % 2 != 0 {
            return Err(Ed25519Error::InvalidInput(
                "Hex string must have even length",
            ));
        }

        let expected_len = hex_str.len() / 2;
        let mut out = vec![0u8; expected_len];

        unsafe {
            let c_str = std::ffi::CString::new(hex_str)
                .map_err(|_| Ed25519Error::InvalidInput("Invalid hex string"))?;
            let decoded_len = sys::fd_hex_decode(
                out.as_mut_ptr() as *mut std::ffi::c_void,
                c_str.as_ptr(),
                expected_len as u64,
            );

            if decoded_len != expected_len as u64 {
                return Err(Ed25519Error::InvalidInput("Failed to decode hex string"));
            }
        }

        Ok(out)
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
        let pubkeys = vec![Pubkey::from([1u8; 32])];
        let signatures = vec![Signature::from([0u8; 64]), Signature::from([1u8; 64])];

        let result = batch_verify_single_message(message, &pubkeys, &signatures);
        assert!(matches!(result, Err(Ed25519Error::InvalidInput(_))));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let secret_key = [123u8; 32];
        let keypair = Keypair::from_secret_key(&secret_key).unwrap();
        let pubkey_bytes = keypair.pubkey().as_bytes();
        let recovered_pubkey = Pubkey::from(*pubkey_bytes);
        assert_eq!(keypair.pubkey(), &recovered_pubkey);

        let message = b"test message";
        let signature = keypair.sign(message).unwrap();
        let signature_bytes = signature.as_bytes();
        let recovered_signature = Signature::from(*signature_bytes);
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

        let mut seeds_with_bump = seeds.to_vec();
        let bump_seed = &[bump1];
        seeds_with_bump.push(bump_seed);
        let recreated = Pubkey::create_program_address(&seeds_with_bump, &program_id).unwrap();

        assert_eq!(recreated, address1);
    }

    #[test]
    fn test_pubkey_hex_roundtrip() {
        let secret_key = [123u8; 32];
        let keypair = Keypair::from_secret_key(&secret_key).unwrap();
        let pubkey = keypair.pubkey();

        let hex_str = pubkey.to_hex();
        let decoded_pubkey = Pubkey::from_hex(&hex_str).unwrap();

        assert_eq!(pubkey, &decoded_pubkey);
    }

    #[test]
    fn test_sig_hex_roundtrip() {
        let secret_key = [200u8; 32];
        let keypair = Keypair::from_secret_key(&secret_key).unwrap();
        let message = b"hex test message";
        let signature = keypair.sign(message).unwrap();

        let hex_str = signature.to_hex();
        let decoded_signature = Signature::from_hex(&hex_str).unwrap();

        assert_eq!(signature, decoded_signature);
    }

    #[cfg(feature = "base58")]
    #[test]
    fn test_pubkey_base58_roundtrip() {
        let secret_key = [42u8; 32];
        let keypair = Keypair::from_secret_key(&secret_key).unwrap();
        let pubkey = keypair.pubkey();

        let base58_str = pubkey.to_base58();
        let decoded_pubkey = Pubkey::from_base58(&base58_str).unwrap();

        assert_eq!(pubkey, &decoded_pubkey);
    }

    #[cfg(feature = "base58")]
    #[test]
    fn test_sig_base58_roundtrip() {
        let secret_key = [99u8; 32];
        let keypair = Keypair::from_secret_key(&secret_key).unwrap();
        let message = b"test message";
        let signature = keypair.sign(message).unwrap();

        let base58_str = signature.to_base58();
        let decoded_signature = Signature::from_base58(&base58_str).unwrap();

        assert_eq!(signature, decoded_signature);
    }

    #[cfg(feature = "base58")]
    #[test]
    fn test_encoding() {
        let pubkey_bytes = [1u8; 32];
        let pubkey = Pubkey::from(pubkey_bytes);

        let base58_str = pubkey.to_base58();
        let hex_str = pubkey.to_hex();

        let from_base58 = Pubkey::from_base58(&base58_str).unwrap();
        let from_hex = Pubkey::from_hex(&hex_str).unwrap();

        assert_eq!(pubkey, from_base58);
        assert_eq!(pubkey, from_hex);
        assert_eq!(from_base58, from_hex);
    }

    #[cfg(feature = "base58")]
    #[test]
    fn test_pubkey_macro() {
        const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
        const __PUBKEY: Pubkey = pubkey!(SYSTEM_PROGRAM);

        let _pubkey = pubkey!(SYSTEM_PROGRAM);
        let pubkey_literal = pubkey!("11111111111111111111111111111111");

        assert_eq!(__PUBKEY, _pubkey);
        let key = Pubkey::from_base58(SYSTEM_PROGRAM).unwrap();
        assert_eq!(pubkey_literal, key);
        assert_eq!(pubkey_literal, _pubkey);
        assert_eq!(_pubkey, key);
    }
}
