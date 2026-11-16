//! The sealed draw key (slice-3: keyed lens draws).
//!
//! Lens concepts for a plan are chosen on the sealed side by HMAC-SHA-256
//! under this key. The key is 32 bytes of [`KeyMaterial`] held in a
//! [`KeyStore`]; its bytes never leave this crate, so a builder without the
//! key cannot compute which lens a plan was drawn with.

use std::fmt;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::key::{KeyMaterial, REDACTED};
use crate::keystore::{KeyName, KeyStore, KeyStoreError};

/// The draw key.
pub struct DrawKey(KeyMaterial);

impl fmt::Debug for DrawKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DrawKey({REDACTED})")
    }
}

impl DrawKey {
    /// Use `material` as the draw key.
    #[must_use]
    pub fn new(material: KeyMaterial) -> Self {
        Self(material)
    }

    /// Load the draw key stored under `name`.
    ///
    /// # Errors
    /// [`KeyStoreError`] when the store refuses or I/O fails.
    pub fn load(store: &dyn KeyStore, name: &KeyName) -> Result<Self, KeyStoreError> {
        Ok(Self(store.get(name)?))
    }

    /// `HMAC-SHA-256(key, message)`.
    #[must_use]
    pub fn mac(&self, message: &[u8]) -> [u8; 32] {
        // HMAC accepts a key of any length, so this branch is unreachable.
        let Ok(mut m) = <Hmac<Sha256> as Mac>::new_from_slice(self.0.expose()) else {
            return [0u8; 32];
        };
        m.update(message);
        m.finalize().into_bytes().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 4231 test case 2 is for a short key; this checks a 32-byte key
    /// against the HMAC definition computed with SHA-256 directly.
    #[test]
    fn mac_is_hmac_sha256() {
        use sha2::Digest;
        let key = [0x0bu8; 32];
        let msg = b"Hi There";
        let mut ipad = [0x36u8; 64];
        let mut opad = [0x5cu8; 64];
        for i in 0..32 {
            ipad[i] ^= key[i];
            opad[i] ^= key[i];
        }
        let inner = Sha256::new()
            .chain_update(ipad)
            .chain_update(msg)
            .finalize();
        let outer: [u8; 32] = Sha256::new()
            .chain_update(opad)
            .chain_update(inner)
            .finalize()
            .into();
        let k = DrawKey::new(KeyMaterial::from_bytes(key));
        assert_eq!(k.mac(msg), outer);
        assert_ne!(
            k.mac(msg),
            DrawKey::new(KeyMaterial::from_bytes([1; 32])).mac(msg)
        );
        assert_eq!(format!("{k:?}"), "DrawKey(<redacted>)");
    }
}
