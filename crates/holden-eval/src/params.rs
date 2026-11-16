//! Moment-of-test parameters, obtained only from the server (slice-4 P2).
//!
//! The evaluator never computes parameters. It takes the body of the MCP
//! `reveal_params` answer and accepts it only as a [`ServerReveal`], which
//! exists only after these checks pass:
//!
//! - the server nonce opens the published commitment;
//! - the VRF proof verifies under the epoch public key published by `brief`,
//!   for exactly this plan commit, evaluated commit and both nonces;
//! - the plan commit and the evaluated commit are the ones being evaluated.
//!
//! A reveal for another commit, another plan, or under another key, or a
//! reveal whose parameters were changed, is refused.

use holden_seal::{
    BuilderNonce, DrawInputs, EpochPublicKey, EvaluatedCommit, MomentParams, NonceCommitment,
    PlanCommit, Reveal, ServerNonce, VrfOutput, VrfProof,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Why a reveal was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamRefusal {
    /// The body is not a reveal.
    Malformed,
    /// The reveal is for another plan.
    OtherPlan,
    /// The reveal is for another evaluated commit.
    OtherCommit,
    /// The reveal names another epoch key than the server published.
    OtherEpochKey,
    /// The server nonce does not open the commitment.
    NonceNotCommitted,
    /// The VRF proof, or the parameters it derives, do not verify.
    ProofRefused,
}

impl std::fmt::Display for ParamRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Malformed => "the reveal is malformed",
            Self::OtherPlan => "the reveal is for another plan",
            Self::OtherCommit => "the reveal is for another evaluated commit",
            Self::OtherEpochKey => "the reveal names another epoch key",
            Self::NonceNotCommitted => "the server nonce does not open its commitment",
            Self::ProofRefused => "the VRF proof does not verify",
        })
    }
}

/// The parameters as revealed, hex and numbers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamsWire {
    /// Seed.
    pub seed: u64,
    /// Lens slot.
    pub lens_slot: u16,
    /// Step-order salt.
    pub step_order_salt: u32,
    /// The 32 derived bytes, hex.
    pub bytes: String,
}

/// The reveal exactly as the server sent it: every draw input, the VRF
/// output and proof, and the parameters. This is the verdict record's
/// parameter proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevealWire {
    /// The run id.
    pub run_id: String,
    /// The plan commit.
    pub plan_commit: String,
    /// The evaluated commit.
    pub evaluated_commit: String,
    /// The server-nonce commitment.
    pub commitment: String,
    /// The server nonce.
    pub server_nonce: String,
    /// The builder nonce.
    pub builder_nonce: String,
    /// The epoch public key.
    pub epoch_public_key: String,
    /// The VRF output.
    pub output: String,
    /// The VRF proof.
    pub proof: String,
    /// The parameters.
    pub params: ParamsWire,
}

fn unhex<const N: usize>(s: &str) -> Result<[u8; N], ParamRefusal> {
    hex::decode(s)
        .ok()
        .and_then(|v| v.try_into().ok())
        .ok_or(ParamRefusal::Malformed)
}

impl RevealWire {
    /// Verify the reveal on its own: the nonce opens the commitment and the
    /// VRF proof verifies under the named epoch key for the named inputs.
    ///
    /// # Errors
    /// A [`ParamRefusal`].
    pub fn verify(&self) -> Result<MomentParams, ParamRefusal> {
        let public = EpochPublicKey::from_bytes(&unhex::<32>(&self.epoch_public_key)?)
            .map_err(|_| ParamRefusal::Malformed)?;
        let server_nonce = ServerNonce(unhex(&self.server_nonce)?);
        let commitment = NonceCommitment(unhex(&self.commitment)?);
        if NonceCommitment::commit(&server_nonce) != commitment {
            return Err(ParamRefusal::NonceNotCommitted);
        }
        let inputs = DrawInputs {
            plan_commit: PlanCommit(unhex(&self.plan_commit)?),
            evaluated_commit: EvaluatedCommit::from_hex(&self.evaluated_commit)
                .map_err(|_| ParamRefusal::Malformed)?,
            server_nonce,
            builder_nonce: BuilderNonce(unhex(&self.builder_nonce)?),
        };
        let reveal = Reveal {
            output: VrfOutput(unhex(&self.output)?),
            proof: VrfProof(unhex::<64>(&self.proof)?),
            params: MomentParams {
                seed: self.params.seed,
                lens_slot: self.params.lens_slot,
                step_order_salt: self.params.step_order_salt,
                bytes: unhex(&self.params.bytes)?,
            },
        };
        holden_seal::verify_committed(&public, &inputs, &commitment, &reveal)
            .map_err(|_| ParamRefusal::ProofRefused)
    }
}

/// A reveal the server gave for this plan and evaluated commit, verified.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerReveal {
    wire: RevealWire,
    params: MomentParams,
}

impl ServerReveal {
    /// Accept the body of a `reveal_params` answer for `plan_commit` and
    /// `evaluated_commit`, under the epoch key `published` (from `brief`).
    ///
    /// # Errors
    /// A [`ParamRefusal`] naming the first check that failed.
    pub fn from_reveal_body(
        body: &Value,
        published: &str,
        plan_commit: &PlanCommit,
        evaluated_commit: &str,
    ) -> Result<Self, ParamRefusal> {
        let wire: RevealWire =
            serde_json::from_value(body.clone()).map_err(|_| ParamRefusal::Malformed)?;
        if wire.plan_commit != hex::encode(plan_commit.0) {
            return Err(ParamRefusal::OtherPlan);
        }
        if wire.evaluated_commit != evaluated_commit {
            return Err(ParamRefusal::OtherCommit);
        }
        if wire.epoch_public_key != published {
            return Err(ParamRefusal::OtherEpochKey);
        }
        let params = wire.verify()?;
        Ok(Self { wire, params })
    }

    /// The parameters.
    #[must_use]
    pub fn params(&self) -> &MomentParams {
        &self.params
    }

    /// The reveal as sent.
    #[must_use]
    pub fn wire(&self) -> &RevealWire {
        &self.wire
    }
}
