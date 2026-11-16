//! Secret key material. It is zeroed on drop, and its `Debug` and `Display`
//! forms are redacted, so a key never reaches a log or a test failure.

use std::fmt;

use chacha20poly1305::aead::OsRng;
use chacha20poly1305::aead::rand_core::RngCore;
use zeroize::Zeroizing;

/// What a redacted secret prints as.
pub const REDACTED: &str = "<redacted>";

/// The operating system's random source failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoRandomness;

impl fmt::Display for NoRandomness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("refused: the operating system random source failed")
    }
}

impl std::error::Error for NoRandomness {}

/// `N` fresh bytes from the operating system's random source (slice-4: world
/// ids and builder nonces). Not for key material: use
/// [`KeyMaterial::generate`].
///
/// # Errors
/// [`NoRandomness`] when the random source fails.
pub fn random_bytes<const N: usize>() -> Result<[u8; N], NoRandomness> {
    let mut out = [0u8; N];
    os_random(&mut out)?;
    Ok(out)
}

/// Fill `buf` from the operating system's random source.
pub(crate) fn os_random(buf: &mut [u8]) -> Result<(), NoRandomness> {
    OsRng.try_fill_bytes(buf).map_err(|_| NoRandomness)
}

/// 32 bytes of secret key material.
pub struct KeyMaterial(Zeroizing<[u8; 32]>);

impl KeyMaterial {
    /// Wrap existing bytes. The caller's copy is not zeroed.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// Take ownership of bytes already held in a zeroing buffer.
    pub(crate) fn from_zeroizing(bytes: Zeroizing<[u8; 32]>) -> Self {
        Self(bytes)
    }

    /// Fresh key material from the operating system's random source.
    ///
    /// # Errors
    /// [`NoRandomness`] when the random source fails.
    pub fn generate() -> Result<Self, NoRandomness> {
        let mut k = Zeroizing::new([0u8; 32]);
        os_random(k.as_mut())?;
        Ok(Self(k))
    }

    /// The secret bytes. Crate-private: nothing outside the crate reads them.
    pub(crate) fn expose(&self) -> &[u8; 32] {
        &self.0
    }

    /// Constant-time equality, for tests and audits that compare two keys.
    #[must_use]
    pub fn ct_eq(&self, other: &Self) -> bool {
        ct_eq(self.expose(), other.expose())
    }
}

impl fmt::Debug for KeyMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KeyMaterial({REDACTED})")
    }
}

impl fmt::Display for KeyMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

/// The 32-byte sealed key under which L2 plans are encrypted.
pub struct SealKey(KeyMaterial);

impl SealKey {
    /// Use `material` as a seal key.
    #[must_use]
    pub fn new(material: KeyMaterial) -> Self {
        Self(material)
    }

    pub(crate) fn expose(&self) -> &[u8; 32] {
        self.0.expose()
    }
}

impl fmt::Debug for SealKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SealKey({REDACTED})")
    }
}

impl fmt::Display for SealKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

/// Constant-time byte-slice equality (lengths are not secret).
pub(crate) fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    std::hint::black_box(diff) == 0
}
