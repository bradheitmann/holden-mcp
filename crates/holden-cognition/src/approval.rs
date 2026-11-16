//! Owner approval records (docs/ARCHITECTURE.md §3.1 step 4).
//!
//! A cognition can host tests only through a [`HostableCognition`], and the
//! only way to make one is [`HostableCognition::admit`]. It requires an
//! [`Approval`]: the owner's Ed25519 signature over
//! `APPROVAL_DOMAIN ‖ cognition id ‖ promise id`, made with the owner key
//! from the key store, checked against the owner public key the caller
//! trusts. An absent approval, an approval for another cognition or
//! promise, an approval by a different key, and a signature that does not
//! verify are all refused.

use std::fmt;

use holden_canon::Sha256Digest;
use holden_seal::{OwnerKey, OwnerPublicKey, OwnerSignature};
use serde::{Deserialize, Serialize};

use crate::cognition::SituatedCognition;
use crate::promise::{ContentId, PlanningDocument, Promise, PromiseError};

/// Domain prefix of the signed message.
pub const APPROVAL_DOMAIN: &[u8] = b"holden-cognition/approval/1";

/// The bytes the owner signs.
#[must_use]
pub fn approval_message(cognition: &ContentId, promise: &ContentId) -> Vec<u8> {
    let mut m = APPROVAL_DOMAIN.to_vec();
    m.extend_from_slice(cognition.as_bytes());
    m.extend_from_slice(promise.as_bytes());
    m
}

/// An owner approval record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    /// The approved cognition.
    pub cognition: ContentId,
    /// The promise it hosts tests for.
    pub promise: ContentId,
    /// The signer's Ed25519 public key, 64 lowercase hex characters.
    pub owner: String,
    /// The Ed25519 signature, 128 lowercase hex characters.
    pub signature: String,
}

impl Approval {
    /// Sign `cognition` for its promise with the owner key.
    #[must_use]
    pub fn sign(owner: &OwnerKey, cognition: &SituatedCognition) -> Self {
        let sig = owner.sign(&approval_message(&cognition.id, &cognition.promise));
        Self {
            cognition: cognition.id,
            promise: cognition.promise,
            owner: hex::encode(owner.public().0),
            signature: hex::encode(sig.0),
        }
    }
}

/// Why a cognition may not host tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApprovalError {
    /// The promise does not match its source document.
    Promise(PromiseError),
    /// The cognition's id is not its content hash.
    CognitionIdNotContentHash,
    /// The cognition relies on another promise.
    CognitionForOtherPromise,
    /// There is no approval record.
    Unapproved,
    /// The approval is for another cognition or promise.
    ApprovalForOther,
    /// The approval was made with a key other than the trusted owner key.
    OwnerKeyMismatch,
    /// The record's key or signature is not well-formed hex.
    Malformed,
    /// The signature does not verify under the trusted owner key.
    BadSignature,
}

impl fmt::Display for ApprovalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Promise(e) => e.fmt(f),
            Self::CognitionIdNotContentHash => {
                f.write_str("refused: the cognition id is not its content hash")
            }
            Self::CognitionForOtherPromise => {
                f.write_str("refused: the cognition relies on another promise")
            }
            Self::Unapproved => f.write_str("refused: the cognition has no owner approval"),
            Self::ApprovalForOther => {
                f.write_str("refused: the approval is for another cognition or promise")
            }
            Self::OwnerKeyMismatch => {
                f.write_str("refused: the approval was made with a key other than the owner's")
            }
            Self::Malformed => f.write_str("refused: the approval record is malformed"),
            Self::BadSignature => {
                f.write_str("refused: the approval signature does not verify under the owner key")
            }
        }
    }
}

impl std::error::Error for ApprovalError {}

fn decode<const N: usize>(s: &str) -> Result<[u8; N], ApprovalError> {
    let mut out = [0u8; N];
    if s.len() != 2 * N || s.bytes().any(|b| b.is_ascii_uppercase()) {
        return Err(ApprovalError::Malformed);
    }
    hex::decode_to_slice(s, &mut out).map_err(|_| ApprovalError::Malformed)?;
    Ok(out)
}

/// An approved promise and cognition: the only input a generator accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostableCognition {
    promise: Promise,
    cognition: SituatedCognition,
    approval: Approval,
    owner: OwnerPublicKey,
}

impl HostableCognition {
    /// Admit `cognition` to host tests for `promise`, given the approval
    /// record and the owner public key the caller trusts.
    ///
    /// Checks, in order: the promise is verbatim in `doc`; the cognition id
    /// is its content hash and it relies on this promise; an approval
    /// exists, names this cognition and promise, names the trusted owner
    /// key, and its signature verifies under that key.
    ///
    /// # Errors
    /// [`ApprovalError`] naming the first failed check.
    pub fn admit(
        doc: &PlanningDocument,
        promise: &Promise,
        cognition: &SituatedCognition,
        approval: Option<&Approval>,
        owner: &OwnerPublicKey,
    ) -> Result<Self, ApprovalError> {
        promise
            .verify_against(doc)
            .map_err(ApprovalError::Promise)?;
        if cognition.computed_id() != cognition.id {
            return Err(ApprovalError::CognitionIdNotContentHash);
        }
        if cognition.promise != promise.id {
            return Err(ApprovalError::CognitionForOtherPromise);
        }
        let approval = approval.ok_or(ApprovalError::Unapproved)?;
        if approval.cognition != cognition.id || approval.promise != promise.id {
            return Err(ApprovalError::ApprovalForOther);
        }
        let signer = OwnerPublicKey(decode::<32>(&approval.owner)?);
        if signer != *owner {
            return Err(ApprovalError::OwnerKeyMismatch);
        }
        let signature = OwnerSignature(decode::<64>(&approval.signature)?);
        verify_signature(
            owner,
            &approval_message(&cognition.id, &promise.id),
            &signature,
        )?;
        Ok(Self {
            promise: promise.clone(),
            cognition: cognition.clone(),
            approval: approval.clone(),
            owner: *owner,
        })
    }

    /// The approved promise.
    #[must_use]
    pub fn promise(&self) -> &Promise {
        &self.promise
    }

    /// The approved cognition.
    #[must_use]
    pub fn cognition(&self) -> &SituatedCognition {
        &self.cognition
    }

    /// The approval record.
    #[must_use]
    pub fn approval(&self) -> &Approval {
        &self.approval
    }

    /// The owner key the approval verified under.
    #[must_use]
    pub fn owner(&self) -> &OwnerPublicKey {
        &self.owner
    }

    /// The source file hash of the approved promise.
    #[must_use]
    pub fn source(&self) -> Sha256Digest {
        self.promise.source
    }
}

fn verify_signature(
    owner: &OwnerPublicKey,
    message: &[u8],
    signature: &OwnerSignature,
) -> Result<(), ApprovalError> {
    owner
        .verify(message, signature)
        .map_err(|_| ApprovalError::BadSignature)
}

/// The owner public key a host's approvals are checked against (slice-7
/// P4).
///
/// It comes only from the owner's key store: [`OwnerTrust::of_owner`] is
/// used by the owner's own `holden approve`, which also publishes the public
/// half next to the key ([`OwnerTrust::publish`]); everyone else reads that
/// published half ([`OwnerTrust::read_published`]) and never touches the
/// secret. The trusted key is never taken from an approval record or from
/// the host repository, so an approval an agent signed with its own key is
/// refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnerTrust {
    public: OwnerPublicKey,
}

/// The file the owner's public key is published in, inside the owner's key
/// store directory.
pub const OWNER_PUBLIC_FILE: &str = "owner.pub";

impl OwnerTrust {
    /// The trust of the owner's own key (the secret half is not kept).
    #[must_use]
    pub fn of_owner(owner: &OwnerKey) -> Self {
        Self {
            public: owner.public(),
        }
    }

    /// The trusted public key.
    #[must_use]
    pub fn public(&self) -> &OwnerPublicKey {
        &self.public
    }

    /// Publish the public half as `owner.pub` (64 lowercase hex and a line
    /// feed) in the owner's key store directory `dir`. An existing file for
    /// another key is never overwritten.
    ///
    /// # Errors
    /// The I/O error, or `AlreadyExists` when `dir` publishes another key.
    pub fn publish(&self, dir: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        let path = dir.join(OWNER_PUBLIC_FILE);
        let text = format!("{}\n", hex::encode(self.public.0));
        match std::fs::read_to_string(&path) {
            Ok(existing) if existing == text => return Ok(path),
            Ok(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "the owner store already publishes another owner key",
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        std::fs::write(&path, text)?;
        Ok(path)
    }

    /// Read the public half the owner published in the owner's key store
    /// directory `dir`.
    ///
    /// # Errors
    /// [`ApprovalError::Malformed`] when the file is absent or is not 64
    /// lowercase hex characters.
    pub fn read_published(dir: &std::path::Path) -> Result<Self, ApprovalError> {
        let text = std::fs::read_to_string(dir.join(OWNER_PUBLIC_FILE))
            .map_err(|_| ApprovalError::Malformed)?;
        Ok(Self {
            public: OwnerPublicKey(decode::<32>(text.trim_end_matches('\n'))?),
        })
    }
}

/// Admit every owner-approved cognition of a host (slice-7 P4): each
/// cognition must rest on one of `promises` (each verbatim in `doc`), and
/// its approval must be signed by the key `trust` holds. Returns the
/// admitted cognitions and, for every other one, why it was refused; an
/// approval signed by any other key, whatever key it names, is refused.
#[must_use]
pub fn admit_approved(
    doc: &PlanningDocument,
    promises: &[Promise],
    cognitions: &[SituatedCognition],
    approvals: &[Approval],
    trust: &OwnerTrust,
) -> (Vec<HostableCognition>, Vec<(ContentId, ApprovalError)>) {
    let mut admitted = Vec::new();
    let mut refused = Vec::new();
    for c in cognitions {
        let Some(promise) = promises.iter().find(|p| p.id == c.promise) else {
            refused.push((c.id, ApprovalError::CognitionForOtherPromise));
            continue;
        };
        let approval = approvals.iter().find(|a| a.cognition == c.id);
        match HostableCognition::admit(doc, promise, c, approval, trust.public()) {
            Ok(h) => admitted.push(h),
            Err(e) => refused.push((c.id, e)),
        }
    }
    (admitted, refused)
}
