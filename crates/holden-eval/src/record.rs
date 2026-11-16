//! The signed verdict record (slice-4 P3) and its replay.
//!
//! A record holds everything needed to reach its decision again: the plan
//! and evaluated commits, the parameter proof (the server's reveal, with
//! every VRF input), the planted values, the bound checks, the captured
//! boundary observations (bounded, each with its SHA-256 digest), the canary
//! and config reports, the rules and engine versions, and the decision. The
//! body is signed with the evaluator's Ed25519 key.
//!
//! [`replay`] recomputes the decision from the stored inputs alone: it
//! checks the signature, re-verifies the VRF proof, re-derives the planted
//! values from the proven parameters, re-hashes every observation, re-runs
//! the rules, and compares. Any changed byte is refused.

use holden_seal::{EvaluatorKey, EvaluatorPublicKey, EvaluatorSignature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::binding::BoundCheck;
use crate::canary::CanaryReport;
use crate::config::ConfigChange;
use crate::confine::ConfinementReport;
use crate::drive::Observation;
use crate::params::{ParamRefusal, RevealWire};
use crate::plant::Planted;
use crate::rules::{self, Decided, Decision, Diagnostic, FailReason, Integrity, RULES_VERSION};
use crate::spawn::Limits;

/// The record format.
pub const RECORD_FORMAT: &str = "holden.verdict/1";

/// The engine version recorded in every verdict.
pub const ENGINE_VERSION: &str = concat!("holden-eval/", env!("CARGO_PKG_VERSION"));

/// The signed body of a verdict record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerdictBody {
    /// [`RECORD_FORMAT`].
    pub format: String,
    /// [`ENGINE_VERSION`] of the evaluator that produced it.
    pub engine_version: String,
    /// [`RULES_VERSION`] of the rules that decided it.
    pub rules_version: String,
    /// The product binding.
    pub product: String,
    /// The world this evaluation ran in.
    pub world_id: String,
    /// The plan id, hex.
    pub plan_id: String,
    /// The plan commit, hex.
    pub plan_commit: String,
    /// The evaluated commit.
    pub evaluated_commit: String,
    /// The baseline commit the public config is diffed against.
    pub baseline_commit: String,
    /// The parameter proof: the server's reveal, verbatim.
    pub reveal: RevealWire,
    /// The planted values (re-derived and compared on replay).
    pub planted: Planted,
    /// The plan's checks, bound to probes.
    pub checks: Vec<BoundCheck>,
    /// The limits every product run had.
    pub limits: Limits,
    /// The confinement summary.
    pub confinement: ConfinementReport,
    /// The canary scan of the evaluated tree.
    pub canary: CanaryReport,
    /// Public config changes since the baseline.
    pub config_changes: Vec<ConfigChange>,
    /// The boundary observations.
    pub observations: Vec<Observation>,
    /// SHA-256 of each observation, in order.
    pub observation_digests: Vec<String>,
    /// The decision and its reasons.
    pub decided: Decided,
}

impl VerdictBody {
    /// The bytes that are signed.
    #[must_use]
    pub fn canonical(&self) -> Vec<u8> {
        let mut out = format!("{RECORD_FORMAT}\n").into_bytes();
        out.extend(serde_json::to_vec(self).unwrap_or_default());
        out
    }
}

/// A signed verdict record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerdictRecord {
    /// The signed body.
    pub body: VerdictBody,
    /// The evaluator's Ed25519 public key, hex.
    pub evaluator_public_key: String,
    /// The Ed25519 signature over [`VerdictBody::canonical`], hex.
    pub signature: String,
}

/// What the builder sees.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicVerdict {
    /// The decision.
    pub decision: Decision,
    /// Typed fail reasons (empty for a pass).
    pub reasons: Vec<FailReason>,
    /// Promise and consequence only.
    pub diagnostics: Vec<Diagnostic>,
}

impl VerdictRecord {
    /// Sign `body` with `key`.
    #[must_use]
    pub fn sign(body: VerdictBody, key: &EvaluatorKey) -> Self {
        let signature = key.sign(&body.canonical());
        Self {
            body,
            evaluator_public_key: hex::encode(key.public().0),
            signature: hex::encode(signature.0),
        }
    }

    /// The recorded decision.
    #[must_use]
    pub fn decision(&self) -> Decision {
        self.body.decided.decision
    }

    /// The builder-visible verdict.
    #[must_use]
    pub fn public(&self) -> PublicVerdict {
        PublicVerdict {
            decision: self.decision(),
            reasons: self.body.decided.reasons.clone(),
            diagnostics: self.body.decided.diagnostics.clone(),
        }
    }

    /// SHA-256 of the record's JSON, hex.
    #[must_use]
    pub fn sha256(&self) -> String {
        hex::encode(Sha256::digest(serde_json::to_vec(self).unwrap_or_default()))
    }
}

/// Why a replay refused a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayRefusal {
    /// The record is not JSON of the record shape.
    Malformed,
    /// An unknown record format.
    UnknownFormat,
    /// Rules this engine does not have.
    UnknownRules,
    /// The record is signed by another key than the pinned one.
    OtherEvaluatorKey,
    /// The signature does not verify.
    BadSignature,
    /// The parameter proof does not verify.
    Reveal(ParamRefusal),
    /// The parameter proof is for another plan or commit than the record.
    RevealMismatch,
    /// The planted values are not the ones the proven parameters derive.
    PlantedMismatch,
    /// The canary report is for another commit.
    CanaryMismatch,
    /// Observation digests are missing or do not match.
    DigestMismatch,
    /// The recomputed decision differs from the recorded one.
    DecisionMismatch,
}

impl std::fmt::Display for ReplayRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed => f.write_str("refused: the record is malformed"),
            Self::UnknownFormat => f.write_str("refused: unknown record format"),
            Self::UnknownRules => f.write_str("refused: unknown rules version"),
            Self::OtherEvaluatorKey => f.write_str("refused: signed by another evaluator key"),
            Self::BadSignature => f.write_str("refused: the signature does not verify"),
            Self::Reveal(r) => write!(f, "refused: parameter proof: {r}"),
            Self::RevealMismatch => {
                f.write_str("refused: the parameter proof is for another plan or commit")
            }
            Self::PlantedMismatch => {
                f.write_str("refused: the planted values do not follow from the parameters")
            }
            Self::CanaryMismatch => f.write_str("refused: the canary scan is for another commit"),
            Self::DigestMismatch => {
                f.write_str("refused: an observation does not match its digest")
            }
            Self::DecisionMismatch => {
                f.write_str("refused: the recomputed decision differs from the record")
            }
        }
    }
}

impl std::error::Error for ReplayRefusal {}

fn unhex<const N: usize>(s: &str) -> Result<[u8; N], ReplayRefusal> {
    hex::decode(s)
        .ok()
        .and_then(|v| v.try_into().ok())
        .ok_or(ReplayRefusal::Malformed)
}

/// Parse a record from JSON text.
///
/// # Errors
/// [`ReplayRefusal::Malformed`].
pub fn parse_record(text: &str) -> Result<VerdictRecord, ReplayRefusal> {
    serde_json::from_str(text).map_err(|_| ReplayRefusal::Malformed)
}

/// Recompute the decision of `record` from its stored inputs alone, and
/// refuse it unless every check holds and the decision matches. When
/// `pinned` is given, the record must be signed by that key.
///
/// # Errors
/// The first [`ReplayRefusal`].
pub fn replay(
    record: &VerdictRecord,
    pinned: Option<&EvaluatorPublicKey>,
) -> Result<Decided, ReplayRefusal> {
    let b = &record.body;
    if b.format != RECORD_FORMAT {
        return Err(ReplayRefusal::UnknownFormat);
    }
    if b.rules_version != RULES_VERSION {
        return Err(ReplayRefusal::UnknownRules);
    }
    let key = EvaluatorPublicKey(unhex(&record.evaluator_public_key)?);
    if pinned.is_some_and(|p| *p != key) {
        return Err(ReplayRefusal::OtherEvaluatorKey);
    }
    let signature = EvaluatorSignature(unhex::<64>(&record.signature)?);
    key.verify(&b.canonical(), &signature)
        .map_err(|_| ReplayRefusal::BadSignature)?;
    let params = b.reveal.verify().map_err(ReplayRefusal::Reveal)?;
    if b.reveal.plan_commit != b.plan_commit || b.reveal.evaluated_commit != b.evaluated_commit {
        return Err(ReplayRefusal::RevealMismatch);
    }
    if Planted::derive(&params.bytes) != b.planted {
        return Err(ReplayRefusal::PlantedMismatch);
    }
    if b.canary.commit != b.evaluated_commit {
        return Err(ReplayRefusal::CanaryMismatch);
    }
    if b.observations.len() != b.observation_digests.len()
        || b.observations
            .iter()
            .zip(&b.observation_digests)
            .any(|(o, d)| o.digest() != *d)
    {
        return Err(ReplayRefusal::DigestMismatch);
    }
    let decided = rules::decide(
        &b.checks,
        &b.planted,
        &b.observations,
        Integrity {
            canary: &b.canary,
            confinement: &b.confinement,
        },
    );
    if decided != b.decided {
        return Err(ReplayRefusal::DecisionMismatch);
    }
    Ok(decided)
}
