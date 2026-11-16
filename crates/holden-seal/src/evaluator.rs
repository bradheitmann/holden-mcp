//! The evaluator's Ed25519 signing key (slice-4: signed verdict records).
//!
//! The sealed evaluator signs every verdict record it produces. The signing
//! key is 32 bytes of [`KeyMaterial`] held in a [`KeyStore`]; its bytes never
//! leave this crate. Anyone holding the evaluator's public key can verify a
//! record; nobody without the key can make one.

use std::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

use crate::key::{KeyMaterial, REDACTED};
use crate::keystore::{KeyName, KeyStore, KeyStoreError};
use crate::owner::SignatureRefused;

/// The evaluator's signing key.
pub struct EvaluatorKey(SigningKey);

impl fmt::Debug for EvaluatorKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EvaluatorKey({REDACTED})")
    }
}

/// The evaluator's public key (32 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EvaluatorPublicKey(pub [u8; 32]);

/// An Ed25519 signature by the evaluator (64 bytes).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EvaluatorSignature(pub [u8; 64]);

impl fmt::Debug for EvaluatorSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EvaluatorSignature({:02x?}..)", &self.0[..4])
    }
}

impl EvaluatorKey {
    /// Use `material` as the evaluator's Ed25519 secret seed.
    #[must_use]
    pub fn from_material(material: &KeyMaterial) -> Self {
        Self(SigningKey::from_bytes(material.expose()))
    }

    /// Load the evaluator key stored under `name`.
    ///
    /// # Errors
    /// [`KeyStoreError`] when the store refuses or I/O fails.
    pub fn load(store: &dyn KeyStore, name: &KeyName) -> Result<Self, KeyStoreError> {
        Ok(Self::from_material(&store.get(name)?))
    }

    /// The public key.
    #[must_use]
    pub fn public(&self) -> EvaluatorPublicKey {
        EvaluatorPublicKey(self.0.verifying_key().to_bytes())
    }

    /// Sign `message`.
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> EvaluatorSignature {
        EvaluatorSignature(self.0.sign(message).to_bytes())
    }
}

impl EvaluatorPublicKey {
    /// Verify `signature` over `message` under this key (strict verification).
    ///
    /// # Errors
    /// [`SignatureRefused`] for a malformed key or a signature that does not
    /// verify.
    pub fn verify(
        &self,
        message: &[u8],
        signature: &EvaluatorSignature,
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
    fn evaluator_signature_verifies_only_for_its_key_and_message() {
        let a = EvaluatorKey::from_material(&KeyMaterial::from_bytes([9u8; 32]));
        let b = EvaluatorKey::from_material(&KeyMaterial::from_bytes([10u8; 32]));
        let sig = a.sign(b"verdict");
        assert_eq!(a.public().verify(b"verdict", &sig), Ok(()));
        assert_eq!(
            a.public().verify(b"verdicT", &sig),
            Err(SignatureRefused::BadSignature)
        );
        assert_eq!(
            b.public().verify(b"verdict", &sig),
            Err(SignatureRefused::BadSignature)
        );
        assert!(!format!("{a:?}").contains("09"));
    }
}
