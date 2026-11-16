//! The server-nonce commitment (docs/ARCHITECTURE.md §3.2).
//!
//! `NonceCommitment = SHA-256(NONCE_COMMIT_DOMAIN ‖ server_nonce)` is
//! published before the builder's commit exists, so the server cannot pick
//! its nonce after seeing the evaluated commit. A draw whose server nonce does
//! not open the recorded commitment is refused before any parameters exist.

use sha2::{Digest, Sha256};

use crate::key::{NoRandomness, ct_eq, os_random};
use crate::{SealError, ServerNonce};

/// Domain prefix of the commitment hash.
pub const NONCE_COMMIT_DOMAIN: &[u8] = b"holden-mcp/server-nonce/commit/1";

/// A published commitment to a server nonce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonceCommitment(pub [u8; 32]);

impl NonceCommitment {
    /// Commit to `nonce`.
    #[must_use]
    pub fn commit(nonce: &ServerNonce) -> Self {
        let mut h = Sha256::new();
        h.update(NONCE_COMMIT_DOMAIN);
        h.update(nonce.0);
        Self(h.finalize().into())
    }
}

/// Accept `nonce` only if it is the nonce `commitment` was made to.
///
/// # Errors
/// [`SealError::NonceNotCommitted`] for any other nonce.
pub fn open(commitment: &NonceCommitment, nonce: &ServerNonce) -> Result<(), SealError> {
    if ct_eq(&NonceCommitment::commit(nonce).0, &commitment.0) {
        Ok(())
    } else {
        Err(SealError::NonceNotCommitted)
    }
}

impl ServerNonce {
    /// A fresh server nonce from the operating system's random source.
    ///
    /// # Errors
    /// [`NoRandomness`] when the random source fails.
    pub fn generate() -> Result<Self, NoRandomness> {
        let mut n = [0u8; 32];
        os_random(&mut n)?;
        Ok(Self(n))
    }
}
