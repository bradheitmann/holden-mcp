//! The owner's Ed25519 signing key (slice-3: approval records).
//!
//! The owner approves a situated cognition for a promise by signing it. The
//! signing key is 32 bytes of [`KeyMaterial`] held in a [`KeyStore`]; its
//! bytes never leave this crate. Anyone holding the owner's public key can
//! verify an approval; nobody without the key can make one.

use std::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

use crate::key::{KeyMaterial, REDACTED};
use crate::keystore::{KeyName, KeyStore, KeyStoreError};

/// The owner's signing key.
pub struct OwnerKey(SigningKey);

impl fmt::Debug for OwnerKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OwnerKey({REDACTED})")
    }
}

/// The owner's public key (32 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OwnerPublicKey(pub [u8; 32]);

/// An Ed25519 signature (64 bytes).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct OwnerSignature(pub [u8; 64]);

impl fmt::Debug for OwnerSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OwnerSignature({:02x?}..)", &self.0[..4])
    }
}

/// Why a signature was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureRefused {
    /// The public key bytes are not a valid Ed25519 point.
    BadPublicKey,
    /// The signature does not verify for this message and key.
    BadSignature,
}

impl fmt::Display for SignatureRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadPublicKey => f.write_str("refused: not a valid Ed25519 public key"),
            Self::BadSignature => f.write_str("refused: the Ed25519 signature does not verify"),
        }
    }
}

impl std::error::Error for SignatureRefused {}

impl OwnerKey {
    /// Use `material` as the owner's Ed25519 secret seed.
    #[must_use]
    pub fn from_material(material: &KeyMaterial) -> Self {
        Self(SigningKey::from_bytes(material.expose()))
    }

    /// Load the owner key stored under `name`.
    ///
    /// # Errors
    /// [`KeyStoreError`] when the store refuses or I/O fails.
    pub fn load(store: &dyn KeyStore, name: &KeyName) -> Result<Self, KeyStoreError> {
        Ok(Self::from_material(&store.get(name)?))
    }

    /// The public key.
    #[must_use]
    pub fn public(&self) -> OwnerPublicKey {
        OwnerPublicKey(self.0.verifying_key().to_bytes())
    }

    /// Sign `message`.
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> OwnerSignature {
        OwnerSignature(self.0.sign(message).to_bytes())
    }
}

impl OwnerPublicKey {
    /// Verify `signature` over `message` under this key (strict verification:
    /// non-canonical and small-order encodings are refused).
    ///
    /// # Errors
    /// [`SignatureRefused`] for a malformed key or a signature that does not
    /// verify.
    pub fn verify(
        &self,
        message: &[u8],
        signature: &OwnerSignature,
    ) -> Result<(), SignatureRefused> {
        let key = VerifyingKey::from_bytes(&self.0).map_err(|_| SignatureRefused::BadPublicKey)?;
        let sig = Signature::from_bytes(&signature.0);
        key.verify_strict(message, &sig)
            .map_err(|_| SignatureRefused::BadSignature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_signature_verifies_only_for_its_key_and_message() {
        let a = OwnerKey::from_material(&KeyMaterial::from_bytes([7u8; 32]));
        let b = OwnerKey::from_material(&KeyMaterial::from_bytes([8u8; 32]));
        let sig = a.sign(b"approve");
        assert_eq!(a.public().verify(b"approve", &sig), Ok(()));
        assert_eq!(
            a.public().verify(b"approvE", &sig),
            Err(SignatureRefused::BadSignature)
        );
        assert_eq!(
            b.public().verify(b"approve", &sig),
            Err(SignatureRefused::BadSignature)
        );
        assert!(!format!("{a:?}").contains("07"));
    }

    /// Slice-12b P3: an Ed25519 approval made from the former plain-file key
    /// remains valid under owner.pub after that key has been sealed. Signature
    /// verification needs only the published public key, not either backend.
    /// Fail-without mutant: reject the pre-migration signature in `verify`.
    #[test]
    fn pre_migration_approvals_still_verify() {
        let root =
            std::env::temp_dir().join(format!("holden-pre-migration-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let legacy = crate::FileKeyStore::open(&root).unwrap();
        let name = crate::KeyName::new("owner").unwrap();
        let material = KeyMaterial::generate().unwrap();
        legacy.put(&name, &material).unwrap();
        let owner = OwnerKey::load(&legacy, &name).unwrap();
        let message = b"pre-migration approval bytes";
        let signature = owner.sign(message);
        let public = owner.public();
        #[cfg(target_os = "linux")]
        crate::LinuxPassphraseKeyStore::open(&root)
            .unwrap()
            .seal_with_passphrase(&name, &material, "throwaway-test-passphrase")
            .unwrap();
        std::fs::remove_file(legacy.key_path(&name)).unwrap();
        public.verify(message, &signature).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
