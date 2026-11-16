//! holden-seal: the L2 seal and the L3 draw behind G-UNPREDICTABLE
//! (docs/ARCHITECTURE.md §3.2).
//!
//! - [`seal`]: a typed [`Plan`] sealed with XChaCha20-Poly1305 under a
//!   32-byte [`SealKey`]; the aad binds the plan id and format version, and
//!   `plan_commit = SHA-256(ciphertext ‖ nonce ‖ aad)`.
//! - [`nonce`]: the server-nonce commitment, published before the builder
//!   commits.
//! - [`keystore`]: the [`KeyStore`] trait, its 0600/0700 file backend and,
//!   with the `keychain` feature on macOS, its Security-framework backend.
//! - The draw: moment-of-test parameters from a verifiable random function
//!   (sr25519 VRF, `schnorrkel`).
//!
//! `params = VRF_sk(plan_commit ‖ evaluated_commit ‖ server_nonce ‖ builder_nonce)`
//!
//! The four inputs are typed newtypes. They enter a Merlin transcript under
//! the domain label [`VRF_DOMAIN`]; each `append_message` call frames the
//! value with its own label and a length prefix, so no two input tuples share
//! a transcript. The VRF admits exactly one output per (key, input), and the
//! proof lets anyone holding the epoch public key check it afterwards.
//!
//! The draw takes the server's secret key as [`EpochKeypair`]. No L2
//! artifact converts into it, so a holder of the sealed plan and its commit
//! cannot compute the parameters (`params_require_server_secret`).
//!
//! Slice-3 additions (additive; nothing above changed behaviour):
//! - [`owner`]: the owner's Ed25519 signing key, loaded from a [`KeyStore`],
//!   for approval records;
//! - [`drawkey`]: the sealed HMAC-SHA-256 draw key for keyed lens draws;
//! - [`seal::SealableDocument`]: any typed plan document sealed with the
//!   same aad, nonce and commit format as [`Plan`].
//!
//! Slice-4 additions (additive; nothing above changed behaviour):
//! - [`evaluator`]: the evaluator's Ed25519 signing key, for signed verdict
//!   records;
//! - [`key::random_bytes`]: the operating system's random source, for
//!   per-evaluation world ids and builder nonces.

pub mod drawkey;
pub mod evaluator;
pub mod key;
#[cfg(all(target_os = "macos", feature = "keychain"))]
pub mod keychain;
pub mod keystore;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod nonce;
pub mod owner;
pub mod seal;

use std::fmt;

use merlin::Transcript;
use schnorrkel::vrf::{VRFPreOut, VRFProof};
use schnorrkel::{ExpansionMode, Keypair, MiniSecretKey, PublicKey};

pub use drawkey::DrawKey;
pub use evaluator::{EvaluatorKey, EvaluatorPublicKey, EvaluatorSignature};
pub use key::{KeyMaterial, NoRandomness, SealKey, random_bytes};
#[cfg(all(target_os = "macos", feature = "keychain"))]
pub use keychain::KeychainKeyStore;
pub use keystore::{FileKeyStore, KeyName, KeyStore, KeyStoreError};
#[cfg(target_os = "linux")]
pub use linux::LinuxPassphraseKeyStore;
pub use nonce::NonceCommitment;
pub use owner::{OwnerKey, OwnerPublicKey, OwnerSignature, SignatureRefused};
pub use seal::{
    Plan, PlanId, PlanStep, SealPlanError, SealableDocument, SealedPlan, UnsealError, plan_commit,
};

/// Domain label of the VRF transcript.
pub const VRF_DOMAIN: &[u8] = b"holden-mcp/moment-params/vrf/1";

/// Context label used to expand the VRF output into parameter bytes.
pub const PARAMS_CONTEXT: &[u8] = b"holden-mcp/moment-params/derive/1";

/// Commitment to the sealed plan: `SHA-256(ciphertext ‖ nonce ‖ aad)`, see
/// [`seal::plan_commit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanCommit(pub [u8; 32]);

/// The git commit id of the evaluated tree: 20 bytes (SHA-1) or 32 bytes
/// (SHA-256 object format).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluatedCommit(Vec<u8>);

/// The nonce the server commits to before the builder commits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerNonce(pub [u8; 32]);

/// The nonce the builder contributes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuilderNonce(pub [u8; 32]);

impl EvaluatedCommit {
    /// Parse a 40- or 64-character lowercase hex commit id.
    ///
    /// # Errors
    /// Refuses any other length or a non-lowercase-hex character.
    pub fn from_hex(s: &str) -> Result<Self, SealError> {
        if s.len() != 40 && s.len() != 64 {
            return Err(SealError::BadInput("evaluated commit must be 40 or 64 hex"));
        }
        if !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(SealError::BadInput(
                "evaluated commit must be lowercase hex",
            ));
        }
        let mut out = Vec::with_capacity(s.len() / 2);
        for pair in s.as_bytes().chunks_exact(2) {
            let hi = nibble(pair[0]);
            let lo = nibble(pair[1]);
            out.push((hi << 4) | lo);
        }
        Ok(Self(out))
    }

    /// The raw commit bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

const fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        _ => c - b'a' + 10,
    }
}

/// The four inputs of one draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawInputs {
    /// Commitment to the sealed plan.
    pub plan_commit: PlanCommit,
    /// The evaluated commit.
    pub evaluated_commit: EvaluatedCommit,
    /// The server's pre-committed nonce.
    pub server_nonce: ServerNonce,
    /// The builder's nonce.
    pub builder_nonce: BuilderNonce,
}

impl DrawInputs {
    /// The inputs of a draw for `sealed`, its `plan_commit` computed from the
    /// sealed bytes.
    #[must_use]
    pub fn for_sealed(
        sealed: &SealedPlan,
        evaluated_commit: EvaluatedCommit,
        server_nonce: ServerNonce,
        builder_nonce: BuilderNonce,
    ) -> Self {
        Self {
            plan_commit: plan_commit(sealed),
            evaluated_commit,
            server_nonce,
            builder_nonce,
        }
    }

    fn transcript(&self) -> Transcript {
        let mut t = Transcript::new(VRF_DOMAIN);
        t.append_message(b"plan_commit", &self.plan_commit.0);
        t.append_message(b"evaluated_commit", self.evaluated_commit.as_bytes());
        t.append_message(b"server_nonce", &self.server_nonce.0);
        t.append_message(b"builder_nonce", &self.builder_nonce.0);
        t
    }
}

/// The epoch key held by the sealed side.
pub struct EpochKeypair(Keypair);

impl fmt::Debug for EpochKeypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EpochKeypair")
            .field("public", &self.public())
            .finish_non_exhaustive()
    }
}

/// The published epoch public key.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EpochPublicKey(PublicKey);

impl fmt::Debug for EpochPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EpochPublicKey({})", hex32(&self.to_bytes()))
    }
}

impl EpochKeypair {
    /// Derive a keypair from a 32-byte seed (Ed25519-style expansion).
    ///
    /// # Errors
    /// Not expected for a 32-byte seed; mapped to [`SealError::BadKey`].
    pub fn from_seed(seed: [u8; 32]) -> Result<Self, SealError> {
        let mini = MiniSecretKey::from_bytes(&seed).map_err(|_| SealError::BadKey)?;
        Ok(Self(mini.expand_to_keypair(ExpansionMode::Ed25519)))
    }

    /// Derive a keypair from stored key material (see [`KeyStore`]).
    ///
    /// # Errors
    /// As [`EpochKeypair::from_seed`].
    pub fn from_material(material: &KeyMaterial) -> Result<Self, SealError> {
        let mini = MiniSecretKey::from_bytes(material.expose()).map_err(|_| SealError::BadKey)?;
        Ok(Self(mini.expand_to_keypair(ExpansionMode::Ed25519)))
    }

    /// The public half.
    #[must_use]
    pub fn public(&self) -> EpochPublicKey {
        EpochPublicKey(self.0.public)
    }
}

impl EpochPublicKey {
    /// Parse a published 32-byte public key, so an auditor can verify a
    /// reveal from the published bytes alone.
    ///
    /// # Errors
    /// [`SealError::BadKey`] when the bytes are not a valid public key.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, SealError> {
        PublicKey::from_bytes(bytes)
            .map(Self)
            .map_err(|_| SealError::BadKey)
    }

    /// The 32-byte compressed Ristretto encoding.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }
}

/// The VRF pre-output (32 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VrfOutput(pub [u8; 32]);

/// The VRF proof (64 bytes).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VrfProof(pub [u8; 64]);

impl fmt::Debug for VrfProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VrfProof({}…)", hex32(&self.0[..32]))
    }
}

/// Typed moment-of-test parameters derived from the VRF output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MomentParams {
    /// Seed for any deterministic per-run choice.
    pub seed: u64,
    /// Which lens slot this run draws.
    pub lens_slot: u16,
    /// Salt for the order of encounter steps.
    pub step_order_salt: u32,
    /// The full 32 derived bytes the fields above are read from.
    pub bytes: [u8; 32],
}

impl MomentParams {
    fn from_bytes(bytes: [u8; 32]) -> Self {
        let mut seed = [0u8; 8];
        seed.copy_from_slice(&bytes[0..8]);
        let mut lens = [0u8; 2];
        lens.copy_from_slice(&bytes[8..10]);
        let mut salt = [0u8; 4];
        salt.copy_from_slice(&bytes[10..14]);
        Self {
            seed: u64::from_le_bytes(seed),
            lens_slot: u16::from_le_bytes(lens),
            step_order_salt: u32::from_le_bytes(salt),
            bytes,
        }
    }
}

/// What the server reveals: the VRF output, its proof and the parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reveal {
    /// VRF pre-output.
    pub output: VrfOutput,
    /// VRF proof.
    pub proof: VrfProof,
    /// The parameters derived from the output.
    pub params: MomentParams,
}

/// Why a draw or a verification failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealError {
    /// The key seed was unusable.
    BadKey,
    /// An input was malformed.
    BadInput(&'static str),
    /// The output bytes are not a valid VRF pre-output.
    MalformedOutput,
    /// The proof bytes are not a valid VRF proof.
    MalformedProof,
    /// The proof does not verify for this key, input and output.
    ProofRefused,
    /// The claimed parameters are not the ones the output derives.
    ParamsMismatch,
    /// The server nonce does not open the recorded nonce commitment.
    NonceNotCommitted,
}

impl fmt::Display for SealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadKey => f.write_str("unusable key seed"),
            Self::BadInput(why) => write!(f, "bad input: {why}"),
            Self::MalformedOutput => f.write_str("refused: malformed VRF output"),
            Self::MalformedProof => f.write_str("refused: malformed VRF proof"),
            Self::ProofRefused => f.write_str("refused: VRF proof does not verify"),
            Self::ParamsMismatch => f.write_str("refused: parameters do not match the VRF output"),
            Self::NonceNotCommitted => {
                f.write_str("refused: the server nonce does not open the recorded commitment")
            }
        }
    }
}

impl std::error::Error for SealError {}

/// Draw the moment-of-test parameters for `inputs` under the epoch key.
#[must_use]
pub fn draw(key: &EpochKeypair, inputs: &DrawInputs) -> Reveal {
    let (inout, proof, _batchable) = key.0.vrf_sign(inputs.transcript());
    let params = MomentParams::from_bytes(inout.make_bytes::<[u8; 32]>(PARAMS_CONTEXT));
    Reveal {
        output: VrfOutput(inout.to_preout().to_bytes()),
        proof: VrfProof(proof.to_bytes()),
        params,
    }
}

/// Verify a reveal under the epoch public key and return its parameters.
///
/// # Errors
/// Refuses malformed or tampered output or proof, a proof that does not
/// verify for these inputs and key, and parameters that do not match.
pub fn verify(
    public: &EpochPublicKey,
    inputs: &DrawInputs,
    reveal: &Reveal,
) -> Result<MomentParams, SealError> {
    let preout = VRFPreOut::from_bytes(&reveal.output.0).map_err(|_| SealError::MalformedOutput)?;
    let proof = VRFProof::from_bytes(&reveal.proof.0).map_err(|_| SealError::MalformedProof)?;
    let (inout, _batchable) = public
        .0
        .vrf_verify(inputs.transcript(), &preout, &proof)
        .map_err(|_| SealError::ProofRefused)?;
    let params = MomentParams::from_bytes(inout.make_bytes::<[u8; 32]>(PARAMS_CONTEXT));
    if params != reveal.params {
        return Err(SealError::ParamsMismatch);
    }
    Ok(params)
}

/// Draw only if `inputs.server_nonce` opens the recorded `commitment`. The
/// commitment is checked before the VRF runs, so a refused draw produces no
/// parameters at all.
///
/// # Errors
/// [`SealError::NonceNotCommitted`] when the nonce does not open the commitment.
pub fn draw_committed(
    key: &EpochKeypair,
    inputs: &DrawInputs,
    commitment: &NonceCommitment,
) -> Result<Reveal, SealError> {
    nonce::open(commitment, &inputs.server_nonce)?;
    Ok(draw(key, inputs))
}

/// Verify a reveal as [`verify`] does, after checking that its server nonce
/// opens the recorded `commitment`.
///
/// # Errors
/// [`SealError::NonceNotCommitted`], or any refusal of [`verify`].
pub fn verify_committed(
    public: &EpochPublicKey,
    inputs: &DrawInputs,
    commitment: &NonceCommitment,
    reveal: &Reveal,
) -> Result<MomentParams, SealError> {
    nonce::open(commitment, &inputs.server_nonce)?;
    verify(public, inputs, reveal)
}

fn hex32(bytes: &[u8]) -> String {
    use fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXED_SEED: [u8; 32] = [0x42; 32];

    fn key() -> EpochKeypair {
        EpochKeypair::from_seed(FIXED_SEED).unwrap()
    }

    fn inputs(commit_hex: &str) -> DrawInputs {
        DrawInputs {
            plan_commit: PlanCommit([0x01; 32]),
            evaluated_commit: EvaluatedCommit::from_hex(commit_hex).unwrap(),
            server_nonce: ServerNonce([0x02; 32]),
            builder_nonce: BuilderNonce([0x03; 32]),
        }
    }

    const COMMIT_A: &str = "0123456789abcdef0123456789abcdef01234567";
    const COMMIT_B: &str = "0123456789abcdef0123456789abcdef01234568";

    #[test]
    fn params_depend_on_evaluated_commit() {
        let k = key();
        let a = draw(&k, &inputs(COMMIT_A));
        let b = draw(&k, &inputs(COMMIT_B));
        assert_ne!(a.output, b.output);
        assert_ne!(a.params, b.params);
    }

    #[test]
    fn reveal_proof_verifies() {
        let k = key();
        let i = inputs(COMMIT_A);
        let r = draw(&k, &i);
        assert_eq!(verify(&k.public(), &i, &r), Ok(r.params));
    }

    #[test]
    fn tampered_proof_is_refused() {
        let k = key();
        let i = inputs(COMMIT_A);
        let r = draw(&k, &i);
        for idx in [0usize, 31, 32, 63] {
            let mut t = r;
            t.proof.0[idx] ^= 0x01;
            let got = verify(&k.public(), &i, &t);
            assert!(
                matches!(
                    got,
                    Err(SealError::ProofRefused | SealError::MalformedProof)
                ),
                "proof byte {idx}: {got:?}"
            );
        }
    }

    #[test]
    fn tampered_output_is_refused() {
        let k = key();
        let i = inputs(COMMIT_A);
        let r = draw(&k, &i);
        let mut t = r;
        t.output.0[0] ^= 0x01;
        let got = verify(&k.public(), &i, &t);
        assert!(
            matches!(
                got,
                Err(SealError::ProofRefused | SealError::MalformedOutput)
            ),
            "{got:?}"
        );
    }

    #[test]
    fn tampered_params_are_refused() {
        let k = key();
        let i = inputs(COMMIT_A);
        let mut r = draw(&k, &i);
        r.params.seed ^= 1;
        assert_eq!(verify(&k.public(), &i, &r), Err(SealError::ParamsMismatch));
    }

    #[test]
    fn reveal_for_other_inputs_is_refused() {
        let k = key();
        let r = draw(&k, &inputs(COMMIT_A));
        assert_eq!(
            verify(&k.public(), &inputs(COMMIT_B), &r),
            Err(SealError::ProofRefused)
        );
    }

    #[test]
    fn other_epoch_key_is_refused() {
        let k = key();
        let other = EpochKeypair::from_seed([0x43; 32]).unwrap();
        let i = inputs(COMMIT_A);
        let r = draw(&k, &i);
        assert_eq!(
            verify(&other.public(), &i, &r),
            Err(SealError::ProofRefused)
        );
    }

    #[test]
    fn evaluated_commit_must_be_lowercase_hex_of_git_length() {
        assert!(EvaluatedCommit::from_hex(&"a".repeat(39)).is_err());
        assert!(EvaluatedCommit::from_hex(&"A".repeat(40)).is_err());
        assert!(EvaluatedCommit::from_hex(&"a".repeat(40)).is_ok());
        assert!(EvaluatedCommit::from_hex(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn fixed_vector() {
        let k = key();
        let i = inputs(COMMIT_A);
        let r = draw(&k, &i);
        assert_eq!(hex32(&k.public().to_bytes()), FIXED_PUBLIC);
        assert_eq!(hex32(&r.output.0), FIXED_OUTPUT);
        assert_eq!(hex32(&r.params.bytes), FIXED_PARAM_BYTES);
        assert_eq!(r.params.seed, FIXED_SEED_U64);
        assert_eq!(r.params.lens_slot, FIXED_LENS_SLOT);
        assert_eq!(r.params.step_order_salt, FIXED_STEP_SALT);
        verify(&k.public(), &i, &r).unwrap();
    }

    const FIXED_PUBLIC: &str = "a4d007a95b28ce117e4051904503612abdcbd22b60475121f92a8b5dedac375a";
    const FIXED_OUTPUT: &str = "2a32d485332709a85a6b21ba8b983687ba5f63ecbd0c91aefb76a949f46a4a41";
    const FIXED_PARAM_BYTES: &str =
        "3a3a66837b79673ec8edcf4195eaa888cba35bcdb609ca584274ac5eb90f0bef";
    const FIXED_SEED_U64: u64 = 4_496_696_324_345_182_778;
    const FIXED_LENS_SLOT: u16 = 60_872;
    const FIXED_STEP_SALT: u32 = 3_935_650_255;

    // ---- slice 1: L2 seal, tamper refusal, L2-alone, nonce commitment, key store ----

    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};

    use sha2::{Digest, Sha256};

    use crate::key::REDACTED;
    use crate::keystore::current_uid;
    use crate::seal::{AAD_LEN, PLAN_FORMAT_V1, TAG_LEN, aad_for, seal_with_nonce, unseal};

    const FIXED_NONCE: [u8; 24] = [0x22; 24];

    fn plan() -> Plan {
        Plan {
            id: PlanId([0x5a; 32]),
            cognition: "cognition-0001".to_owned(),
            steps: vec![
                PlanStep {
                    action: "submit an empty form at the boundary".to_owned(),
                    expect: "a typed refusal naming the empty field".to_owned(),
                },
                PlanStep {
                    action: "retry with the field filled".to_owned(),
                    expect: "acceptance with no leftover error".to_owned(),
                },
            ],
        }
    }

    fn seal_key(b: u8) -> SealKey {
        SealKey::new(KeyMaterial::from_bytes([b; 32]))
    }

    fn fixed_sealed() -> SealedPlan {
        seal_with_nonce(&seal_key(0x11), &plan(), FIXED_NONCE).unwrap()
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        hex32(&Sha256::digest(bytes))
    }

    #[test]
    fn seal_roundtrip() {
        let k = SealKey::new(KeyMaterial::generate().unwrap());
        let p = plan();
        let s = seal::seal(&k, &p).unwrap();
        assert_eq!(s.aad, aad_for(&p.id, PLAN_FORMAT_V1));
        assert_eq!(s.aad.len(), AAD_LEN);
        let plain = serde_json::to_vec(&p).unwrap();
        assert_eq!(s.ciphertext.len(), plain.len() + TAG_LEN);
        assert!(
            !s.ciphertext
                .windows(8)
                .any(|w| plain.windows(8).any(|x| x == w)),
            "plaintext bytes appear in the ciphertext"
        );
        let c = plan_commit(&s);
        assert_eq!(unseal(&k, &s, &c), Ok(p.clone()));
        // A second seal draws a fresh nonce, so its bytes and commit differ.
        let s2 = seal::seal(&k, &p).unwrap();
        assert_ne!(s.nonce, s2.nonce);
        assert_ne!(c, plan_commit(&s2));
        assert_eq!(unseal(&k, &s2, &plan_commit(&s2)), Ok(p));
    }

    #[test]
    fn commit_binds_aad() {
        let s = fixed_sealed();
        // The commit is SHA-256(ciphertext ‖ nonce ‖ aad) over the sealed bytes.
        let mut bytes = s.ciphertext.clone();
        bytes.extend_from_slice(&s.nonce);
        bytes.extend_from_slice(&s.aad);
        assert_eq!(hex32(&plan_commit(&s).0), sha256_hex(&bytes));
        // Same ciphertext and nonce, a different plan id or format version in
        // the aad: a different commit.
        let mut other_id = s.clone();
        other_id.aad = aad_for(&PlanId([0x5b; 32]), PLAN_FORMAT_V1);
        let mut other_version = s.clone();
        other_version.aad = aad_for(&plan().id, PLAN_FORMAT_V1 + 1);
        for t in [&other_id, &other_version] {
            assert_eq!(t.ciphertext, s.ciphertext);
            assert_eq!(t.nonce, s.nonce);
            assert_ne!(plan_commit(t), plan_commit(&s));
        }
    }

    #[test]
    fn seal_fixed_vector() {
        let s = fixed_sealed();
        assert_eq!(hex32(&s.aad), FIXED_AAD);
        assert_eq!(s.ciphertext.len(), FIXED_CIPHERTEXT_LEN);
        assert_eq!(sha256_hex(&s.ciphertext), FIXED_CIPHERTEXT_SHA256);
        assert_eq!(hex32(&plan_commit(&s).0), FIXED_PLAN_COMMIT);
        assert_eq!(unseal(&seal_key(0x11), &s, &plan_commit(&s)), Ok(plan()));
    }

    const FIXED_AAD: &str = concat!(
        "686f6c64656e2d6d63702f6c322d706c616e2f6161642f310001",
        "5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a"
    );
    const FIXED_CIPHERTEXT_LEN: usize = 345;
    const FIXED_CIPHERTEXT_SHA256: &str =
        "5620024d30922d6be105433d4b7fae0f6bb49d30c76ccd98ac6e13819f543705";
    const FIXED_PLAN_COMMIT: &str =
        "2d368c47d0cdb02515b2225e1509a885844309a504f6c2109b5e525f014b8bb5";

    #[test]
    fn tampered_ciphertext_is_refused() {
        let k = seal_key(0x11);
        let s = seal::seal(&k, &plan()).unwrap();
        let c = plan_commit(&s);
        let last = s.ciphertext.len() - 1;
        // Body bytes and tag bytes.
        for idx in [0, last / 2, last - TAG_LEN, last - TAG_LEN + 1, last] {
            let mut t = s.clone();
            t.ciphertext[idx] ^= 0x01;
            assert_eq!(unseal(&k, &t, &c), Err(UnsealError::CommitMismatch));
            // A tamperer who also republishes the commit meets the AEAD tag.
            let got = unseal(&k, &t, &plan_commit(&t));
            assert_eq!(got, Err(UnsealError::TagRefused), "ciphertext byte {idx}");
            assert_eq!(UnsealError::TagRefused.check(), "aead_tag");
        }
        let mut short = s.clone();
        short.ciphertext.truncate(last);
        assert_eq!(
            unseal(&k, &short, &plan_commit(&short)),
            Err(UnsealError::TagRefused)
        );
        let mut long = s;
        long.ciphertext.push(0);
        assert_eq!(
            unseal(&k, &long, &plan_commit(&long)),
            Err(UnsealError::TagRefused)
        );
    }

    #[test]
    fn tampered_nonce_is_refused() {
        let k = seal_key(0x11);
        let s = seal::seal(&k, &plan()).unwrap();
        let c = plan_commit(&s);
        for idx in [0, 11, 23] {
            let mut t = s.clone();
            t.nonce[idx] ^= 0x80;
            assert_eq!(unseal(&k, &t, &c), Err(UnsealError::CommitMismatch));
            assert_eq!(
                unseal(&k, &t, &plan_commit(&t)),
                Err(UnsealError::TagRefused),
                "nonce byte {idx}"
            );
        }
    }

    #[test]
    fn tampered_aad_is_refused() {
        let k = seal_key(0x11);
        let s = seal::seal(&k, &plan()).unwrap();
        let c = plan_commit(&s);
        // Domain bytes: the aad check.
        let mut t = s.clone();
        t.aad[0] ^= 0x01;
        assert_eq!(
            unseal(&k, &t, &plan_commit(&t)),
            Err(UnsealError::AadMalformed("wrong domain"))
        );
        // Format version: the format check.
        let mut t = s.clone();
        t.aad[25] ^= 0x01;
        let got = unseal(&k, &t, &plan_commit(&t));
        assert_eq!(got, Err(UnsealError::UnsupportedFormat(0)));
        // Plan id bytes: the commit, then the AEAD tag.
        for idx in [26, 40, AAD_LEN - 1] {
            let mut t = s.clone();
            t.aad[idx] ^= 0x01;
            assert_eq!(unseal(&k, &t, &c), Err(UnsealError::CommitMismatch));
            assert_eq!(
                unseal(&k, &t, &plan_commit(&t)),
                Err(UnsealError::TagRefused),
                "aad byte {idx}"
            );
        }
        // Length.
        let mut t = s;
        t.aad.push(0);
        let got = unseal(&k, &t, &plan_commit(&t));
        assert_eq!(got, Err(UnsealError::AadMalformed("wrong length")));
        assert_eq!(got.map_err(|e| e.check()), Err("aad"));
    }

    #[test]
    fn wrong_key_is_refused() {
        let s = seal::seal(&seal_key(0x11), &plan()).unwrap();
        let c = plan_commit(&s);
        for k in [
            seal_key(0x12),
            SealKey::new(KeyMaterial::generate().unwrap()),
        ] {
            let got = unseal(&k, &s, &c);
            assert_eq!(got, Err(UnsealError::TagRefused));
            assert!(got.unwrap_err().to_string().contains("`aead_tag`"));
        }
        assert_eq!(unseal(&seal_key(0x11), &s, &c), Ok(plan()));
    }

    fn sealed_inputs(s: &SealedPlan, server: ServerNonce) -> DrawInputs {
        DrawInputs::for_sealed(
            s,
            EvaluatedCommit::from_hex(COMMIT_A).unwrap(),
            server,
            BuilderNonce([0x03; 32]),
        )
    }

    #[test]
    fn params_require_server_secret() {
        let s = fixed_sealed();
        let inputs = sealed_inputs(&s, ServerNonce([0x02; 32]));
        assert_eq!(inputs.plan_commit, plan_commit(&s));
        let a = EpochKeypair::from_seed([0x42; 32]).unwrap();
        let b = EpochKeypair::from_seed([0x43; 32]).unwrap();
        let ra = draw(&a, &inputs);
        let rb = draw(&b, &inputs);
        // Same L2, same evaluated commit, different server keys: different
        // parameters.
        assert_ne!(ra.output, rb.output);
        assert_ne!(ra.params, rb.params);
        // Each verifies under its own public key.
        assert_eq!(verify(&a.public(), &inputs, &ra), Ok(ra.params));
        assert_eq!(verify(&b.public(), &inputs, &rb), Ok(rb.params));
        // What L2 and the builder hold, hashed, is not the parameters.
        let mut h = Sha256::new();
        h.update(inputs.plan_commit.0);
        h.update(inputs.evaluated_commit.as_bytes());
        let guess: [u8; 32] = h.finalize().into();
        assert_ne!(guess, ra.params.bytes);
        assert_ne!(guess, rb.params.bytes);
    }

    #[test]
    fn foreign_key_proof_is_refused() {
        let s = fixed_sealed();
        let inputs = sealed_inputs(&s, ServerNonce([0x02; 32]));
        let commitment = NonceCommitment::commit(&inputs.server_nonce);
        let a = EpochKeypair::from_seed([0x42; 32]).unwrap();
        let b = EpochKeypair::from_seed([0x43; 32]).unwrap();
        let ra = draw_committed(&a, &inputs, &commitment).unwrap();
        let rb = draw_committed(&b, &inputs, &commitment).unwrap();
        assert_eq!(
            verify(&b.public(), &inputs, &ra),
            Err(SealError::ProofRefused)
        );
        assert_eq!(
            verify(&a.public(), &inputs, &rb),
            Err(SealError::ProofRefused)
        );
        assert_eq!(
            verify_committed(&b.public(), &inputs, &commitment, &ra),
            Err(SealError::ProofRefused)
        );
        assert_eq!(
            verify_committed(&a.public(), &inputs, &commitment, &ra),
            Ok(ra.params)
        );
    }

    #[test]
    fn nonce_commitment_opens() {
        let fixed = ServerNonce([0x02; 32]);
        let mut preimage = nonce::NONCE_COMMIT_DOMAIN.to_vec();
        preimage.extend_from_slice(&fixed.0);
        assert_eq!(
            hex32(&NonceCommitment::commit(&fixed).0),
            sha256_hex(&preimage)
        );
        let n = ServerNonce::generate().unwrap();
        let c = NonceCommitment::commit(&n);
        assert_eq!(nonce::open(&c, &n), Ok(()));
        let s = fixed_sealed();
        let inputs = sealed_inputs(&s, n);
        let key = EpochKeypair::from_seed([0x42; 32]).unwrap();
        let r = draw_committed(&key, &inputs, &c).unwrap();
        // VRF proofs are randomised; the output and parameters are unique.
        let again = draw(&key, &inputs);
        assert_eq!((r.output, r.params), (again.output, again.params));
        assert_eq!(
            verify_committed(&key.public(), &inputs, &c, &r),
            Ok(r.params)
        );
    }

    #[test]
    fn uncommitted_nonce_is_refused() {
        let committed = ServerNonce([0x02; 32]);
        let c = NonceCommitment::commit(&committed);
        let key = EpochKeypair::from_seed([0x42; 32]).unwrap();
        let s = fixed_sealed();
        for (byte, bit) in [(0usize, 0x01u8), (16, 0x10), (31, 0x80)] {
            let mut other = committed;
            other.0[byte] ^= bit;
            assert_eq!(nonce::open(&c, &other), Err(SealError::NonceNotCommitted));
            let inputs = sealed_inputs(&s, other);
            // Refused before any parameters exist.
            assert_eq!(
                draw_committed(&key, &inputs, &c),
                Err(SealError::NonceNotCommitted)
            );
            // An honest draw for the other nonce does not pass the audit.
            let r = draw(&key, &inputs);
            assert_eq!(
                verify_committed(&key.public(), &inputs, &c, &r),
                Err(SealError::NonceNotCommitted)
            );
        }
        let fresh = ServerNonce::generate().unwrap();
        assert_eq!(nonce::open(&c, &fresh), Err(SealError::NonceNotCommitted));
    }

    fn scratch(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        std::env::temp_dir().join(format!(
            "holden-seal-test-{tag}-{}-{}-{nanos}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ))
    }

    fn mode_of(p: &Path) -> u32 {
        fs::symlink_metadata(p).unwrap().mode() & 0o7777
    }

    fn chmod(p: &Path, mode: u32) {
        fs::set_permissions(p, fs::Permissions::from_mode(mode)).unwrap();
    }

    #[test]
    fn file_store_roundtrip() {
        let root = scratch("roundtrip");
        let dir = root.join("keys");
        let store = FileKeyStore::open(&dir).unwrap();
        assert_eq!(mode_of(&dir), 0o700);
        assert_eq!(fs::symlink_metadata(&dir).unwrap().uid(), current_uid());
        let name = KeyName::new("epoch-1").unwrap();
        let key = KeyMaterial::generate().unwrap();
        store.put(&name, &key).unwrap();
        let path = store.key_path(&name);
        let meta = fs::symlink_metadata(&path).unwrap();
        assert_eq!(meta.mode() & 0o7777, 0o600);
        assert_eq!(meta.uid(), current_uid());
        assert_eq!(meta.len(), 32);
        assert!(store.get(&name).unwrap().ct_eq(&key));
        // A reopened store loads the same key.
        let again = FileKeyStore::open(&dir).unwrap();
        assert!(again.get(&name).unwrap().ct_eq(&key));
        // Keys are never overwritten.
        assert_eq!(
            store.put(&name, &KeyMaterial::generate().unwrap()),
            Err(KeyStoreError::AlreadyExists { path: path.clone() })
        );
        assert!(store.get(&name).unwrap().ct_eq(&key));
        // A stored key seals and unseals.
        let sk = SealKey::new(store.get(&name).unwrap());
        let s = seal::seal(&sk, &plan()).unwrap();
        let reloaded = SealKey::new(again.get(&name).unwrap());
        assert_eq!(unseal(&reloaded, &s, &plan_commit(&s)), Ok(plan()));
        // Names are single plain file names.
        for bad in ["", "../x", "a/b", "A", "x.key"] {
            assert_eq!(KeyName::new(bad), Err(KeyStoreError::BadName), "{bad:?}");
        }
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn world_readable_key_is_refused() {
        let root = scratch("world-readable");
        let store = FileKeyStore::open(root.join("keys")).unwrap();
        let name = KeyName::new("seal").unwrap();
        store.put(&name, &KeyMaterial::generate().unwrap()).unwrap();
        let path = store.key_path(&name);
        for mode in [0o644, 0o604, 0o640, 0o660, 0o606, 0o601] {
            chmod(&path, mode);
            let err = store.get(&name).unwrap_err();
            assert_eq!(
                err,
                KeyStoreError::InsecureMode {
                    path: path.clone(),
                    mode
                }
            );
            let text = err.to_string();
            assert!(text.contains(&path.display().to_string()), "{text}");
            assert!(text.contains(&format!("{mode:04o}")), "{text}");
        }
        for mode in [0o600, 0o400] {
            chmod(&path, mode);
            assert!(store.get(&name).is_ok(), "mode {mode:04o}");
        }
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn accessible_key_dir_is_refused() {
        let root = scratch("open-dir");
        let dir = root.join("keys");
        let store = FileKeyStore::open(&dir).unwrap();
        let name = KeyName::new("seal").unwrap();
        store.put(&name, &KeyMaterial::generate().unwrap()).unwrap();
        for mode in [0o750, 0o705, 0o755, 0o710] {
            chmod(&dir, mode);
            let want = KeyStoreError::InsecureMode {
                path: dir.clone(),
                mode,
            };
            assert_eq!(store.get(&name).unwrap_err(), want);
            assert_eq!(
                store.put(
                    &KeyName::new("other").unwrap(),
                    &KeyMaterial::generate().unwrap()
                ),
                Err(want.clone())
            );
            assert_eq!(FileKeyStore::open(&dir).unwrap_err(), want);
        }
        chmod(&dir, 0o700);
        assert!(store.get(&name).is_ok());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn foreign_owned_dir_is_refused() {
        // The filesystem root is owned by uid 0 on macOS and Linux.
        let root = Path::new("/");
        assert_eq!(fs::symlink_metadata(root).unwrap().uid(), 0);
        let got = FileKeyStore::open(root).unwrap_err();
        if current_uid() == 0 {
            // As root the owner matches; the 0755 mode still refuses it.
            assert!(matches!(got, KeyStoreError::InsecureMode { .. }), "{got}");
        } else {
            assert_eq!(
                got,
                KeyStoreError::WrongOwner {
                    path: root.to_path_buf(),
                    uid: 0,
                    expected: current_uid()
                }
            );
            assert!(got.to_string().contains("owned by uid 0"));
        }
    }

    #[test]
    fn key_debug_is_redacted() {
        let mut bytes = [0u8; 32];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = 0xc0 + u8::try_from(i).unwrap();
        }
        let hex = hex32(&bytes);
        let m = KeyMaterial::from_bytes(bytes);
        let epoch = EpochKeypair::from_material(&m).unwrap();
        let forms = [
            format!("{m:?}"),
            format!("{m}"),
            format!("{:?}", SealKey::new(KeyMaterial::from_bytes(bytes))),
            format!("{}", SealKey::new(KeyMaterial::from_bytes(bytes))),
            format!("{epoch:?}"),
            format!("{:?}", Some(&m)),
            format!("{:#?}", [&m]),
        ];
        for f in &forms {
            assert!(!f.contains(&hex), "{f}");
            assert!(!f.contains(&hex[..8]), "{f}");
            assert!(!f.to_uppercase().contains(&hex[..8].to_uppercase()), "{f}");
            assert!(!f.contains("192, 193"), "{f}");
            assert!(!f.contains("192,\n"), "{f}");
        }
        assert_eq!(format!("{m}"), REDACTED);
        assert_eq!(format!("{m:?}"), format!("KeyMaterial({REDACTED})"));
    }
}
