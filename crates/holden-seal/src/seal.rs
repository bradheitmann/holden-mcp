//! L2: the sealed plan (docs/ARCHITECTURE.md §3.2).
//!
//! A typed [`Plan`] is encoded as JSON and encrypted with XChaCha20-Poly1305
//! under a 32-byte [`SealKey`]. The associated data binds the plan id and the
//! format version:
//!
//! ```text
//! aad = AAD_DOMAIN (24 bytes) ‖ format_version (u16, big endian) ‖ plan_id (32 bytes)
//! plan_commit = SHA-256(ciphertext ‖ nonce ‖ aad)
//! ```
//!
//! `plan_commit` is computed from the sealed bytes only; no key is involved.
//! A well-formed aad is exactly [`AAD_LEN`] bytes and the nonce exactly
//! [`NONCE_LEN`], so the concatenation splits one way only for every sealed
//! plan that [`unseal`] can accept.

use std::fmt;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::PlanCommit;
use crate::key::{NoRandomness, SealKey, ct_eq, os_random};

/// Domain prefix of the associated data.
pub const AAD_DOMAIN: &[u8; 24] = b"holden-mcp/l2-plan/aad/1";

/// The plan format this crate writes and reads.
pub const PLAN_FORMAT_V1: u16 = 1;

/// Length of a well-formed aad.
pub const AAD_LEN: usize = AAD_DOMAIN.len() + 2 + 32;

/// Length of an XChaCha20-Poly1305 nonce.
pub const NONCE_LEN: usize = 24;

/// Length of the Poly1305 tag at the end of the ciphertext.
pub const TAG_LEN: usize = 16;

/// The plan's 32-byte id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanId(pub [u8; 32]);

/// One step of a sealed plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanStep {
    /// What the evaluator does at the product boundary.
    pub action: String,
    /// What it must observe.
    pub expect: String,
}

/// The typed plan sealed at L2 (the *how*).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    /// The plan id; the aad binds it.
    pub id: PlanId,
    /// Content id of the L1 situated cognition the plan sits in.
    pub cognition: String,
    /// The steps, in their unrandomised order.
    pub steps: Vec<PlanStep>,
}

/// A sealed plan in its wire form. Fields are public so that a holder can
/// store and transmit it; any change to them is refused by [`unseal`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedPlan {
    /// The associated data, as transmitted.
    pub aad: Vec<u8>,
    /// The 24-byte nonce.
    pub nonce: [u8; NONCE_LEN],
    /// Ciphertext followed by the 16-byte tag.
    pub ciphertext: Vec<u8>,
}

/// Build the aad for `id` under `format_version`.
#[must_use]
pub fn aad_for(id: &PlanId, format_version: u16) -> Vec<u8> {
    let mut aad = Vec::with_capacity(AAD_LEN);
    aad.extend_from_slice(AAD_DOMAIN);
    aad.extend_from_slice(&format_version.to_be_bytes());
    aad.extend_from_slice(&id.0);
    aad
}

/// `plan_commit = SHA-256(ciphertext ‖ nonce ‖ aad)`, from the sealed bytes only.
#[must_use]
pub fn plan_commit(sealed: &SealedPlan) -> PlanCommit {
    let mut h = Sha256::new();
    h.update(&sealed.ciphertext);
    h.update(sealed.nonce);
    h.update(&sealed.aad);
    PlanCommit(h.finalize().into())
}

/// Why sealing failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealPlanError {
    /// The random source failed while drawing the nonce.
    NoRandomness,
    /// The plan could not be encoded.
    Encode,
    /// The AEAD refused to encrypt (message too long).
    Encrypt,
}

impl fmt::Display for SealPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRandomness => NoRandomness.fmt(f),
            Self::Encode => f.write_str("refused: the plan could not be encoded"),
            Self::Encrypt => f.write_str("refused: the AEAD refused to encrypt"),
        }
    }
}

impl std::error::Error for SealPlanError {}

/// Which unseal check refused the sealed plan. No variant carries plaintext.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsealError {
    /// Check `aad`: wrong length or domain.
    AadMalformed(&'static str),
    /// Check `format_version`: a version this crate does not read.
    UnsupportedFormat(u16),
    /// Check `plan_commit`: the sealed bytes do not hash to the recorded commit.
    CommitMismatch,
    /// Check `aead_tag`: the tag does not verify (ciphertext, nonce, aad or key).
    TagRefused,
    /// Check `plan_decode`: the authenticated plaintext is not a plan.
    PlanMalformed,
    /// Check `plan_id`: the plan's id differs from the id the aad binds.
    PlanIdMismatch,
}

impl UnsealError {
    /// The name of the check that failed.
    #[must_use]
    pub const fn check(&self) -> &'static str {
        match self {
            Self::AadMalformed(_) => "aad",
            Self::UnsupportedFormat(_) => "format_version",
            Self::CommitMismatch => "plan_commit",
            Self::TagRefused => "aead_tag",
            Self::PlanMalformed => "plan_decode",
            Self::PlanIdMismatch => "plan_id",
        }
    }
}

impl fmt::Display for UnsealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "refused: unseal check `{}` failed", self.check())?;
        match self {
            Self::AadMalformed(why) => write!(f, ": {why}"),
            Self::UnsupportedFormat(v) => write!(f, ": format version {v}"),
            _ => Ok(()),
        }
    }
}

impl std::error::Error for UnsealError {}

/// A typed plan document that can be sealed at L2: it serializes as JSON
/// and carries the id the aad binds. [`Plan`] is one; generated plans
/// (slice-3, `holden-gen`) are another. Every sealable document uses the
/// same aad, nonce and `plan_commit` format.
pub trait SealableDocument: Serialize + for<'de> Deserialize<'de> {
    /// The id the aad binds.
    fn plan_id(&self) -> PlanId;
}

impl SealableDocument for Plan {
    fn plan_id(&self) -> PlanId {
        self.id
    }
}

/// Seal `plan` under `key` with a fresh random nonce.
///
/// # Errors
/// [`SealPlanError`] when the random source, the encoder or the AEAD fails.
pub fn seal(key: &SealKey, plan: &Plan) -> Result<SealedPlan, SealPlanError> {
    seal_document(key, plan)
}

/// Seal any [`SealableDocument`] under `key` with a fresh random nonce.
///
/// # Errors
/// [`SealPlanError`] when the random source, the encoder or the AEAD fails.
pub fn seal_document<T: SealableDocument>(
    key: &SealKey,
    doc: &T,
) -> Result<SealedPlan, SealPlanError> {
    let mut nonce = [0u8; NONCE_LEN];
    os_random(&mut nonce).map_err(|_| SealPlanError::NoRandomness)?;
    seal_document_with_nonce(key, doc, nonce)
}

/// Seal with a caller-chosen nonce. Crate-private: used for fixed vectors.
/// A nonce must never repeat under one key.
#[cfg(test)]
pub(crate) fn seal_with_nonce(
    key: &SealKey,
    plan: &Plan,
    nonce: [u8; NONCE_LEN],
) -> Result<SealedPlan, SealPlanError> {
    seal_document_with_nonce(key, plan, nonce)
}

fn seal_document_with_nonce<T: SealableDocument>(
    key: &SealKey,
    plan: &T,
    nonce: [u8; NONCE_LEN],
) -> Result<SealedPlan, SealPlanError> {
    let msg = Zeroizing::new(serde_json::to_vec(plan).map_err(|_| SealPlanError::Encode)?);
    let aad = aad_for(&plan.plan_id(), PLAN_FORMAT_V1);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key.expose()));
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &msg,
                aad: &aad,
            },
        )
        .map_err(|_| SealPlanError::Encrypt)?;
    Ok(SealedPlan {
        aad,
        nonce,
        ciphertext,
    })
}

fn parse_aad(aad: &[u8]) -> Result<PlanId, UnsealError> {
    if aad.len() != AAD_LEN {
        return Err(UnsealError::AadMalformed("wrong length"));
    }
    let (domain, rest) = aad.split_at(AAD_DOMAIN.len());
    if domain != AAD_DOMAIN {
        return Err(UnsealError::AadMalformed("wrong domain"));
    }
    let (version, id) = rest.split_at(2);
    let version = u16::from_be_bytes([version[0], version[1]]);
    if version != PLAN_FORMAT_V1 {
        return Err(UnsealError::UnsupportedFormat(version));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(id);
    Ok(PlanId(out))
}

/// Unseal `sealed` under `key`, after checking it against the recorded
/// `expected` commit. Returns the whole typed plan or a typed refusal; never
/// partial plaintext.
///
/// Checks run in this order: `aad`, `format_version`, `plan_commit`,
/// `aead_tag`, `plan_decode`, `plan_id`.
///
/// # Errors
/// [`UnsealError`] naming the first check that failed.
pub fn unseal(
    key: &SealKey,
    sealed: &SealedPlan,
    expected: &PlanCommit,
) -> Result<Plan, UnsealError> {
    unseal_document(key, sealed, expected)
}

/// Unseal any [`SealableDocument`], with the same checks in the same order
/// as [`unseal`].
///
/// # Errors
/// [`UnsealError`] naming the first check that failed.
pub fn unseal_document<T: SealableDocument>(
    key: &SealKey,
    sealed: &SealedPlan,
    expected: &PlanCommit,
) -> Result<T, UnsealError> {
    let aad_id = parse_aad(&sealed.aad)?;
    if !ct_eq(&plan_commit(sealed).0, &expected.0) {
        return Err(UnsealError::CommitMismatch);
    }
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key.expose()));
    let plaintext = Zeroizing::new(
        cipher
            .decrypt(
                XNonce::from_slice(&sealed.nonce),
                Payload {
                    msg: &sealed.ciphertext,
                    aad: &sealed.aad,
                },
            )
            .map_err(|_| UnsealError::TagRefused)?,
    );
    let plan: T = serde_json::from_slice(&plaintext).map_err(|_| UnsealError::PlanMalformed)?;
    if plan.plan_id() != aad_id {
        return Err(UnsealError::PlanIdMismatch);
    }
    Ok(plan)
}
