//! The run ledger and the eight operations, independent of the transport.
//! (The run ledger here is the server's in-memory run state; the signed,
//! persistent ledger is `holden-ledger`, which slice-5 appends verdicts to
//! and serves `telemetry` from.)
//!
//! Slice-4 adds `submit_observation` and `verdict` (evaluator only, while
//! the run is open). A verdict record is accepted only when it replays
//! (`holden_eval::replay`) under the configured evaluator key, belongs to
//! this run's reveal, and lists exactly the observation digests submitted
//! for the run; it then closes the run at once. Registration accepts the
//! generated plan type (`holden_gen::GenPlan`) as well as the slice-1
//! `Plan`.
//!
//! The core holds the epoch (VRF) key, the plan seal (AEAD) key, the
//! registered sealed plans, the nonce commitments and the run records. Every
//! operation takes the caller's [`Caller`] and returns either a public
//! response value or a typed [`Refusal`]. A refusal's text is a fixed string
//! per code, so no error path can carry a secret.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use holden_gen::GenPlan;
use holden_ledger::Bars;
use holden_seal::{
    BuilderNonce, DrawInputs, EpochKeypair, EpochPublicKey, EvaluatedCommit, EvaluatorKey,
    EvaluatorPublicKey, FileKeyStore, KeyStore, KeyStoreError, NonceCommitment, Plan, PlanCommit,
    Reveal, SealError, SealKey, SealedPlan, ServerNonce, UnsealError, plan_commit,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::{ConfigError, Role, ServerConfig};

/// Milliseconds since the Unix epoch.
pub type Millis = u64;

/// A clock the server reads step times from. `Manual` is a real clock whose
/// time moves only when told to; it lets a test close a run without waiting.
#[derive(Clone, Debug)]
pub enum Clock {
    /// The system clock.
    System,
    /// A clock advanced explicitly.
    Manual(ManualClock),
}

/// A clock advanced explicitly (see [`Clock::Manual`]).
#[derive(Clone, Debug)]
pub struct ManualClock(Arc<AtomicU64>);

impl ManualClock {
    /// A clock that reads `start_ms`.
    #[must_use]
    pub fn new(start_ms: Millis) -> Self {
        Self(Arc::new(AtomicU64::new(start_ms)))
    }

    /// Move the clock forward by `ms`.
    pub fn advance(&self, ms: Millis) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}

impl Clock {
    /// The current time.
    #[must_use]
    pub fn now_ms(&self) -> Millis {
        match self {
            Self::System => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX)),
            Self::Manual(m) => m.0.load(Ordering::SeqCst),
        }
    }
}

/// Where the server's log lines go. Lines never carry key bytes, plan
/// plaintext, nonces or parameters: only tool names, roles and outcomes.
#[derive(Clone, Debug)]
pub struct Log {
    lines: Arc<Mutex<Vec<String>>>,
    echo_stderr: bool,
}

impl Log {
    /// Keep lines in memory only (they can be read back with [`Log::lines`]).
    #[must_use]
    pub fn capture() -> Self {
        Self {
            lines: Arc::new(Mutex::new(Vec::new())),
            echo_stderr: false,
        }
    }

    /// Keep lines in memory and write each to stderr.
    #[must_use]
    pub fn stderr() -> Self {
        Self {
            lines: Arc::new(Mutex::new(Vec::new())),
            echo_stderr: true,
        }
    }

    /// Record one line.
    pub fn line(&self, line: String) {
        if self.echo_stderr {
            eprintln!("{line}");
        }
        lock(&self.lines).push(line);
    }

    /// Every line recorded so far.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        lock(&self.lines).clone()
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// How a caller reached the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via {
    /// The Unix socket; the uid was read from the kernel.
    Socket,
    /// Stdio (evaluator-only, enabled in config).
    Stdio,
    /// Loopback TCP (no kernel-verified identity).
    Tcp,
}

/// Who is calling: the role from the sealed config, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caller {
    /// The mapped role; `None` for an unmapped caller.
    pub role: Option<Role>,
    /// The kernel-reported uid (socket callers only).
    pub uid: Option<u32>,
    /// The transport.
    pub via: Via,
}

impl Caller {
    fn label(&self) -> &'static str {
        self.role.map_or("unmapped", Role::as_str)
    }
}

/// A typed refusal. Its message is fixed per code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The caller's uid is not mapped to a role.
    UnmappedCaller,
    /// The caller's role may not use this tool.
    ForbiddenRole,
    /// An argument is malformed.
    BadInput(&'static str),
    /// The plan commit names no registered plan.
    UnknownPlan,
    /// No server-nonce commitment was recorded for this plan.
    NoCommitment,
    /// Every recorded server nonce for this plan already opened a run.
    ServerNonceUsed,
    /// No run with this id was begun.
    UnknownRun,
    /// The run window has expired.
    RunClosed,
    /// The run is still open.
    RunOpen,
    /// The random source failed.
    NoRandomness,
    /// The draw refused (the nonce did not open its commitment).
    DrawRefused,
    /// An observation with this id was already submitted for the run.
    DuplicateObservation,
    /// The run already holds the most observations a run may hold.
    TooManyObservations,
    /// No evaluator key is configured, so no verdict can be accepted.
    NoVerdictKey,
    /// The record does not replay: unsigned, tampered or inconsistent.
    VerdictRefused,
    /// The record belongs to another run, plan, commit or reveal.
    VerdictMismatch,
    /// The record's observations are not exactly the ones submitted.
    ObservationsMismatch,
    /// No signed ledger is configured.
    NoLedger,
    /// The signed ledger refused (its tip does not hold, or I/O failed).
    LedgerFailed,
}

impl Refusal {
    /// The wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnmappedCaller => "unmapped_caller",
            Self::ForbiddenRole => "forbidden_role",
            Self::BadInput(_) => "bad_input",
            Self::UnknownPlan => "unknown_plan",
            Self::NoCommitment => "no_commitment",
            Self::ServerNonceUsed => "server_nonce_used",
            Self::UnknownRun => "unknown_run",
            Self::RunClosed => "run_closed",
            Self::RunOpen => "run_open",
            Self::NoRandomness => "no_randomness",
            Self::DrawRefused => "draw_refused",
            Self::DuplicateObservation => "duplicate_observation",
            Self::TooManyObservations => "too_many_observations",
            Self::NoVerdictKey => "no_verdict_key",
            Self::VerdictRefused => "verdict_refused",
            Self::VerdictMismatch => "verdict_mismatch",
            Self::ObservationsMismatch => "observations_mismatch",
            Self::NoLedger => "no_ledger",
            Self::LedgerFailed => "ledger_failed",
        }
    }

    /// The fixed message.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::UnmappedCaller => "the caller's uid is not mapped to a role; only brief is open",
            Self::ForbiddenRole => "the caller's role may not use this tool",
            Self::BadInput(why) => why,
            Self::UnknownPlan => "no registered plan has this plan_commit",
            Self::NoCommitment => "no server-nonce commitment was recorded for this plan",
            Self::ServerNonceUsed => "the recorded server nonce already opened a run",
            Self::UnknownRun => "no run with this id was begun",
            Self::RunClosed => "the run window has expired",
            Self::RunOpen => "the run is still open; audit is available after close",
            Self::NoRandomness => "the random source failed",
            Self::DrawRefused => "the draw was refused",
            Self::DuplicateObservation => "an observation with this id was already submitted",
            Self::TooManyObservations => "the run holds the most observations a run may hold",
            Self::NoVerdictKey => "no evaluator key is configured; no verdict can be accepted",
            Self::VerdictRefused => "the verdict record does not replay",
            Self::VerdictMismatch => "the verdict record belongs to another run",
            Self::ObservationsMismatch => {
                "the verdict record's observations are not the ones submitted"
            }
            Self::NoLedger => "no signed ledger is configured",
            Self::LedgerFailed => "the signed ledger refused the operation",
        }
    }

    /// The structured refusal body.
    #[must_use]
    pub fn to_json(self) -> Value {
        json!({ "refusal": self.code(), "message": self.message() })
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

/// Why the server could not start.
#[derive(Debug)]
pub enum StartError {
    /// The config was refused.
    Config(ConfigError),
    /// The key store refused.
    KeyStore(KeyStoreError),
    /// A key could not be used.
    Key(SealError),
    /// A sealed plan file was refused.
    Plan { path: PathBuf, why: String },
    /// An I/O error on `path`.
    Io { path: PathBuf, kind: io::ErrorKind },
    /// The socket directory is not a 0700 directory owned by this user, or
    /// the socket path holds something other than a socket.
    InsecureSocketPath { path: PathBuf, why: &'static str },
    /// The MCP service failed to start.
    Service(String),
    /// The signed ledger could not be opened.
    Ledger(String),
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(e) => write!(f, "{e}"),
            Self::KeyStore(e) => write!(f, "key store refused: {e}"),
            Self::Key(e) => write!(f, "key refused: {e}"),
            Self::Plan { path, why } => write!(f, "plan {} refused: {why}", path.display()),
            Self::Io { path, kind } => write!(f, "I/O error on {}: {kind:?}", path.display()),
            Self::InsecureSocketPath { path, why } => {
                write!(f, "socket path {} refused: {why}", path.display())
            }
            Self::Service(why) => write!(f, "MCP service failed: {why}"),
            Self::Ledger(why) => write!(f, "signed ledger refused: {why}"),
        }
    }
}

impl std::error::Error for StartError {}

impl From<ConfigError> for StartError {
    fn from(e: ConfigError) -> Self {
        Self::Config(e)
    }
}

/// The wire form of a sealed plan file (`*.sealed.json` in `plans_dir`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedPlanFile {
    /// Always [`SEALED_PLAN_FORMAT`].
    pub format: String,
    /// The aad, hex.
    pub aad: String,
    /// The 24-byte nonce, hex.
    pub nonce: String,
    /// Ciphertext and tag, hex.
    pub ciphertext: String,
}

/// The format id of a sealed plan file.
pub const SEALED_PLAN_FORMAT: &str = "holden.sealed-plan/1";

impl SealedPlanFile {
    /// The wire form of `sealed`.
    #[must_use]
    pub fn from_sealed(sealed: &SealedPlan) -> Self {
        Self {
            format: SEALED_PLAN_FORMAT.to_owned(),
            aad: hex::encode(&sealed.aad),
            nonce: hex::encode(sealed.nonce),
            ciphertext: hex::encode(&sealed.ciphertext),
        }
    }

    /// Back to a [`SealedPlan`].
    ///
    /// # Errors
    /// A description of the malformed field.
    pub fn to_sealed(&self) -> Result<SealedPlan, &'static str> {
        if self.format != SEALED_PLAN_FORMAT {
            return Err("unknown format");
        }
        let aad = hex::decode(&self.aad).map_err(|_| "aad is not hex")?;
        let nonce = hex::decode(&self.nonce).map_err(|_| "nonce is not hex")?;
        let nonce: [u8; 24] = nonce.try_into().map_err(|_| "nonce is not 24 bytes")?;
        let ciphertext = hex::decode(&self.ciphertext).map_err(|_| "ciphertext is not hex")?;
        Ok(SealedPlan {
            aad,
            nonce,
            ciphertext,
        })
    }
}

struct CommitmentRecord {
    plan: PlanCommit,
    commitment: NonceCommitment,
    nonce: ServerNonce,
    committed_at_ms: Millis,
    used: bool,
}

/// One run: the commitment, both nonces, the evaluated commit, the time of
/// each step and the draw.
#[derive(Clone, Debug)]
pub struct RunRecord {
    /// The run id (the commitment bytes).
    pub run_id: [u8; 32],
    /// The plan.
    pub plan_commit: PlanCommit,
    /// The server-nonce commitment, published before the run.
    pub commitment: NonceCommitment,
    /// The server nonce (secret until reveal or close).
    pub server_nonce: ServerNonce,
    /// The builder's nonce.
    pub builder_nonce: BuilderNonce,
    /// The evaluated commit.
    pub evaluated_commit: EvaluatedCommit,
    /// When `commit_nonce` recorded the commitment.
    pub committed_at_ms: Millis,
    /// When `begin_run` opened the run.
    pub begun_at_ms: Millis,
    /// When the run window ends.
    pub closes_at_ms: Millis,
    /// When `reveal_params` first answered, if it did.
    pub revealed_at_ms: Option<Millis>,
    /// The draw.
    pub reveal: Reveal,
    /// Observation digests submitted for the run: id → (sha256, length).
    pub observations: BTreeMap<String, ([u8; 32], u64)>,
    /// The accepted verdict, if any.
    pub verdict: Option<VerdictSummary>,
}

/// An accepted verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerdictSummary {
    /// SHA-256 of the record, hex.
    pub sha256: String,
    /// The decision (`pass` or `fail`).
    pub decision: &'static str,
    /// When it was accepted (the run closed then).
    pub at_ms: Millis,
}

/// The most observations one run may hold.
pub const MAX_OBSERVATIONS: usize = 4096;

/// The kind of a registered plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanKind {
    /// A generated plan (`holden_gen::GenPlan`, slice-3): the format the
    /// generator emits and the evaluator runs.
    Generated,
    /// A slice-1 `holden_seal::Plan`.
    Slice1,
}

impl PlanKind {
    /// The wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Generated => "holden.gen-plan/1",
            Self::Slice1 => "holden.plan/1",
        }
    }
}

/// Open a sealed plan under `key` as one of the registered plan types. A
/// generated plan must also hash to its own id.
///
/// # Errors
/// The [`UnsealError`] of the first failed check.
pub fn open_plan(
    key: &SealKey,
    sealed: &SealedPlan,
    commit: &PlanCommit,
) -> Result<PlanKind, UnsealError> {
    match holden_seal::seal::unseal_document::<GenPlan>(key, sealed, commit) {
        Ok(plan) if plan.id == plan.computed_id() => Ok(PlanKind::Generated),
        Ok(_) => Err(UnsealError::PlanIdMismatch),
        Err(UnsealError::PlanMalformed) => {
            holden_seal::seal::unseal_document::<Plan>(key, sealed, commit)
                .map(|_| PlanKind::Slice1)
        }
        Err(e) => Err(e),
    }
}

#[derive(Default)]
struct Ledger {
    commitments: Vec<CommitmentRecord>,
    runs: BTreeMap<[u8; 32], RunRecord>,
}

/// The signed ledger the core appends accepted verdicts to (slice-5).
struct Signed {
    ledger: holden_ledger::Ledger,
    key: EvaluatorKey,
}

/// The server core. One core may back several listeners, each with its own
/// sealed config (for example a socket listener and a stdio session).
pub struct Core {
    epoch: EpochKeypair,
    // Held so that only plans sealed under this server's key are served.
    _seal_key: SealKey,
    plans: Vec<PlanCommit>,
    generated: Vec<(PlanCommit, GenPlan)>,
    run_window_ms: Millis,
    verdict_key: Option<EvaluatorPublicKey>,
    clock: Clock,
    log: Log,
    ledger: Mutex<Ledger>,
    signed: Option<Mutex<Signed>>,
    bars: Bars,
}

impl fmt::Debug for Core {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Core")
            .field("epoch_public", &self.epoch.public())
            .field("plans", &self.plans.len())
            .field("run_window_ms", &self.run_window_ms)
            .finish_non_exhaustive()
    }
}

fn parse32(s: &str, what: &'static str) -> Result<[u8; 32], Refusal> {
    if s.len() != 64
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Refusal::BadInput(what));
    }
    let v = hex::decode(s).map_err(|_| Refusal::BadInput(what))?;
    v.try_into().map_err(|_| Refusal::BadInput(what))
}

type GeneratedPlans = Vec<(PlanCommit, GenPlan)>;

fn load_plans(dir: &Path, key: &SealKey) -> Result<(Vec<PlanCommit>, GeneratedPlans), StartError> {
    let io_err = |path: &Path| {
        let path = path.to_path_buf();
        move |e: io::Error| StartError::Io {
            path,
            kind: e.kind(),
        }
    };
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(io_err(dir))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".sealed.json"))
        })
        .collect();
    paths.sort();
    let mut plans = Vec::new();
    let mut generated = Vec::new();
    for path in paths {
        let refused = |why: String| StartError::Plan {
            path: path.clone(),
            why,
        };
        let text = fs::read_to_string(&path).map_err(io_err(&path))?;
        let file: SealedPlanFile =
            serde_json::from_str(&text).map_err(|e| refused(e.to_string()))?;
        let sealed = file.to_sealed().map_err(|w| refused(w.to_owned()))?;
        let commit = plan_commit(&sealed);
        // Authenticity: the plan must open under this server's seal key, as
        // a generated plan or a slice-1 plan. The plaintext is dropped at
        // once; no tool returns it.
        let kind = open_plan(key, &sealed, &commit).map_err(|e| refused(e.to_string()))?;
        if kind == PlanKind::Generated {
            let plan = holden_gen::unseal_plan(key, &sealed, &commit)
                .map_err(|e| refused(e.to_string()))?;
            generated.push((commit, plan));
        }
        if !plans.contains(&commit) {
            plans.push(commit);
        }
    }
    Ok((plans, generated))
}

/// The public reveal body (parameters, output, proof and every draw input).
fn reveal_body(run: &RunRecord, epoch: &EpochPublicKey) -> Value {
    let p = &run.reveal.params;
    json!({
        "run_id": hex::encode(run.run_id),
        "plan_commit": hex::encode(run.plan_commit.0),
        "evaluated_commit": hex::encode(run.evaluated_commit.as_bytes()),
        "commitment": hex::encode(run.commitment.0),
        "server_nonce": hex::encode(run.server_nonce.0),
        "builder_nonce": hex::encode(run.builder_nonce.0),
        "epoch_public_key": hex::encode(epoch.to_bytes()),
        "output": hex::encode(run.reveal.output.0),
        "proof": hex::encode(run.reveal.proof.0),
        "params": {
            "seed": p.seed,
            "lens_slot": p.lens_slot,
            "step_order_salt": p.step_order_salt,
            "bytes": hex::encode(p.bytes),
        },
    })
}

/// The eight tool names, in listing order (`telemetry` since slice-5).
pub const TOOLS: [&str; 8] = [
    "brief",
    "commit_nonce",
    "begin_run",
    "reveal_params",
    "submit_observation",
    "verdict",
    "audit_reveal",
    "telemetry",
];

impl Core {
    /// Open a core from `config`: load both keys from the file key store and
    /// register every sealed plan in `plans_dir` that opens under the seal key.
    ///
    /// # Errors
    /// [`StartError`] naming what was refused.
    pub fn open(config: &ServerConfig, clock: Clock, log: Log) -> Result<Self, StartError> {
        let store = FileKeyStore::open(config.key_dir()).map_err(StartError::KeyStore)?;
        let epoch_material = store
            .get(config.epoch_key())
            .map_err(StartError::KeyStore)?;
        let epoch = EpochKeypair::from_material(&epoch_material).map_err(StartError::Key)?;
        let seal_key = SealKey::new(store.get(config.seal_key()).map_err(StartError::KeyStore)?);
        let (plans, generated) = load_plans(config.plans_dir(), &seal_key)?;
        let signed = match config.ledger() {
            None => None,
            Some(l) => {
                let key = EvaluatorKey::load(&store, &l.key).map_err(StartError::KeyStore)?;
                let ledger = holden_ledger::Ledger::open(&l.db)
                    .map_err(|e| StartError::Ledger(e.to_string()))?;
                Some(Mutex::new(Signed { ledger, key }))
            }
        };
        log.line(format!(
            "holden-mcp: core open: {} plan(s), run window {} ms, signed ledger {}",
            plans.len(),
            config.run_window_ms(),
            if signed.is_some() { "on" } else { "off" }
        ));
        Ok(Self {
            epoch,
            _seal_key: seal_key,
            plans,
            generated,
            run_window_ms: config.run_window_ms(),
            verdict_key: config.verdict_key(),
            clock,
            log,
            ledger: Mutex::new(Ledger::default()),
            signed,
            bars: *config.bars(),
        })
    }

    /// The published epoch public key.
    #[must_use]
    pub fn epoch_public(&self) -> EpochPublicKey {
        self.epoch.public()
    }

    /// The log.
    #[must_use]
    pub fn log(&self) -> &Log {
        &self.log
    }

    fn record(&self, tool: &str, caller: &Caller, outcome: &Result<Value, Refusal>) {
        let out = match outcome {
            Ok(_) => "ok".to_owned(),
            Err(r) => format!("refused:{}", r.code()),
        };
        self.log.line(format!(
            "holden-mcp: tool={tool} caller={} via={:?} outcome={out}",
            caller.label(),
            caller.via
        ));
    }

    fn require_mapped(caller: &Caller) -> Result<Role, Refusal> {
        caller.role.ok_or(Refusal::UnmappedCaller)
    }

    fn require_evaluator(caller: &Caller) -> Result<(), Refusal> {
        match Self::require_mapped(caller)? {
            Role::Evaluator => Ok(()),
            Role::Implementer => Err(Refusal::ForbiddenRole),
        }
    }

    fn known_plan(&self, s: &str) -> Result<PlanCommit, Refusal> {
        let plan = PlanCommit(parse32(s, "plan_commit must be 64 lowercase hex")?);
        if self.plans.contains(&plan) {
            Ok(plan)
        } else {
            Err(Refusal::UnknownPlan)
        }
    }

    /// `brief`: public facts for any caller.
    pub fn brief(&self, caller: &Caller) -> Value {
        let role = caller.label();
        let allowed = |tool: &str| match (caller.role, tool) {
            (_, "brief") => true,
            (None, _) => false,
            (Some(Role::Evaluator), _) => true,
            (Some(Role::Implementer), "commit_nonce") => true,
            (Some(Role::Implementer), _) => false,
        };
        let tools: Vec<Value> = TOOLS
            .iter()
            .map(|t| json!({ "name": t, "allowed": allowed(t) }))
            .collect();
        let out = json!({
            "server": "holden-mcp",
            "version": env!("CARGO_PKG_VERSION"),
            "caller_role": role,
            "epoch_public_key": hex::encode(self.epoch.public().to_bytes()),
            "plans": self.plans.iter().map(|p| hex::encode(p.0)).collect::<Vec<_>>(),
            "run_window_ms": self.run_window_ms,
            "tools": tools,
        });
        self.record("brief", caller, &Ok(Value::Null));
        out
    }

    /// `commit_nonce`: draw a fresh server nonce for a plan and publish its
    /// commitment. Any mapped role.
    ///
    /// # Errors
    /// A [`Refusal`].
    pub fn commit_nonce(&self, caller: &Caller, plan_commit: &str) -> Result<Value, Refusal> {
        let out = (|| {
            Self::require_mapped(caller)?;
            let plan = self.known_plan(plan_commit)?;
            let nonce = ServerNonce::generate().map_err(|_| Refusal::NoRandomness)?;
            let commitment = NonceCommitment::commit(&nonce);
            let now = self.clock.now_ms();
            lock(&self.ledger).commitments.push(CommitmentRecord {
                plan,
                commitment,
                nonce,
                committed_at_ms: now,
                used: false,
            });
            Ok(json!({
                "plan_commit": hex::encode(plan.0),
                "commitment": hex::encode(commitment.0),
                "committed_at_ms": now,
            }))
        })();
        self.record("commit_nonce", caller, &out);
        out
    }

    /// `begin_run`: evaluator only. Consumes the oldest unused server-nonce
    /// commitment for the plan; a plan with no commitment, or whose every
    /// commitment already opened a run, is refused.
    ///
    /// # Errors
    /// A [`Refusal`].
    pub fn begin_run(
        &self,
        caller: &Caller,
        plan_commit: &str,
        evaluated_commit: &str,
        builder_nonce: &str,
    ) -> Result<Value, Refusal> {
        let out = (|| {
            Self::require_evaluator(caller)?;
            let plan = self.known_plan(plan_commit)?;
            let evaluated = EvaluatedCommit::from_hex(evaluated_commit).map_err(|_| {
                Refusal::BadInput("evaluated_commit must be 40 or 64 lowercase hex")
            })?;
            let builder = BuilderNonce(parse32(
                builder_nonce,
                "builder_nonce must be 64 lowercase hex",
            )?);
            let now = self.clock.now_ms();
            let mut ledger = lock(&self.ledger);
            let mut any = false;
            let mut slot = None;
            for (i, c) in ledger.commitments.iter().enumerate() {
                if c.plan == plan {
                    any = true;
                    if !c.used {
                        slot = Some(i);
                        break;
                    }
                }
            }
            let Some(i) = slot else {
                return Err(if any {
                    Refusal::ServerNonceUsed
                } else {
                    Refusal::NoCommitment
                });
            };
            let c = &ledger.commitments[i];
            let (commitment, nonce, committed_at_ms) = (c.commitment, c.nonce, c.committed_at_ms);
            let inputs = DrawInputs {
                plan_commit: plan,
                evaluated_commit: evaluated.clone(),
                server_nonce: nonce,
                builder_nonce: builder,
            };
            let reveal = holden_seal::draw_committed(&self.epoch, &inputs, &commitment)
                .map_err(|_| Refusal::DrawRefused)?;
            // One server nonce opens at most one run.
            ledger.commitments[i].used = true;
            let closes_at_ms = now.saturating_add(self.run_window_ms);
            ledger.runs.insert(
                commitment.0,
                RunRecord {
                    run_id: commitment.0,
                    plan_commit: plan,
                    commitment,
                    server_nonce: nonce,
                    builder_nonce: builder,
                    evaluated_commit: evaluated.clone(),
                    committed_at_ms,
                    begun_at_ms: now,
                    closes_at_ms,
                    revealed_at_ms: None,
                    reveal,
                    observations: BTreeMap::new(),
                    verdict: None,
                },
            );
            Ok(json!({
                "run_id": hex::encode(commitment.0),
                "plan_commit": hex::encode(plan.0),
                "commitment": hex::encode(commitment.0),
                "evaluated_commit": hex::encode(evaluated.as_bytes()),
                "builder_nonce": hex::encode(builder.0),
                "committed_at_ms": committed_at_ms,
                "begun_at_ms": now,
                "closes_at_ms": closes_at_ms,
            }))
        })();
        self.record("begin_run", caller, &out);
        out
    }

    /// `reveal_params`: evaluator only, during the run window.
    ///
    /// # Errors
    /// A [`Refusal`].
    pub fn reveal_params(&self, caller: &Caller, run_id: &str) -> Result<Value, Refusal> {
        let out = (|| {
            Self::require_evaluator(caller)?;
            let id = parse32(run_id, "run_id must be 64 lowercase hex")?;
            let now = self.clock.now_ms();
            let mut ledger = lock(&self.ledger);
            let run = ledger.runs.get_mut(&id).ok_or(Refusal::UnknownRun)?;
            if now >= run.closes_at_ms {
                return Err(Refusal::RunClosed);
            }
            if run.revealed_at_ms.is_none() {
                run.revealed_at_ms = Some(now);
            }
            Ok(reveal_body(run, &self.epoch.public()))
        })();
        self.record("reveal_params", caller, &out);
        out
    }

    /// `audit_reveal`: any mapped role, after the run window has closed.
    /// Returns the reveal plus the run record's step times.
    ///
    /// # Errors
    /// A [`Refusal`].
    pub fn audit_reveal(&self, caller: &Caller, run_id: &str) -> Result<Value, Refusal> {
        let out = (|| {
            Self::require_mapped(caller)?;
            let id = parse32(run_id, "run_id must be 64 lowercase hex")?;
            let now = self.clock.now_ms();
            let ledger = lock(&self.ledger);
            let run = ledger.runs.get(&id).ok_or(Refusal::UnknownRun)?;
            if now < run.closes_at_ms {
                return Err(Refusal::RunOpen);
            }
            let mut body = reveal_body(run, &self.epoch.public());
            body["record"] = json!({
                "committed_at_ms": run.committed_at_ms,
                "begun_at_ms": run.begun_at_ms,
                "revealed_at_ms": run.revealed_at_ms,
                "closes_at_ms": run.closes_at_ms,
                "observations": run.observations.len(),
            });
            body["verdict"] = run.verdict.as_ref().map_or(
                Value::Null,
                |v| json!({ "sha256": v.sha256, "decision": v.decision, "at_ms": v.at_ms }),
            );
            Ok(body)
        })();
        self.record("audit_reveal", caller, &out);
        out
    }

    /// `submit_observation`: evaluator only, while the run is open. Records
    /// one boundary observation's digest for the run.
    ///
    /// # Errors
    /// A [`Refusal`].
    pub fn submit_observation(
        &self,
        caller: &Caller,
        run_id: &str,
        observation_id: &str,
        sha256: &str,
        len: u64,
    ) -> Result<Value, Refusal> {
        let out = (|| {
            Self::require_evaluator(caller)?;
            let id = parse32(run_id, "run_id must be 64 lowercase hex")?;
            let digest = parse32(sha256, "sha256 must be 64 lowercase hex")?;
            let ok_id = !observation_id.is_empty()
                && observation_id.len() <= 64
                && observation_id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
            if !ok_id {
                return Err(Refusal::BadInput(
                    "observation_id must be 1 to 64 characters of [a-z0-9-]",
                ));
            }
            let now = self.clock.now_ms();
            let mut ledger = lock(&self.ledger);
            let run = ledger.runs.get_mut(&id).ok_or(Refusal::UnknownRun)?;
            if now >= run.closes_at_ms {
                return Err(Refusal::RunClosed);
            }
            if run.observations.contains_key(observation_id) {
                return Err(Refusal::DuplicateObservation);
            }
            if run.observations.len() >= MAX_OBSERVATIONS {
                return Err(Refusal::TooManyObservations);
            }
            run.observations
                .insert(observation_id.to_owned(), (digest, len));
            Ok(json!({
                "run_id": run_id,
                "observation_id": observation_id,
                "observations": run.observations.len(),
            }))
        })();
        self.record("submit_observation", caller, &out);
        out
    }

    /// `verdict`: evaluator only, while the run is open. Accepts a signed
    /// verdict record that replays under the configured evaluator key,
    /// belongs to this run's reveal and lists exactly the submitted
    /// observations; then closes the run.
    ///
    /// # Errors
    /// A [`Refusal`].
    pub fn verdict(&self, caller: &Caller, run_id: &str, record: &str) -> Result<Value, Refusal> {
        let out = (|| {
            Self::require_evaluator(caller)?;
            let id = parse32(run_id, "run_id must be 64 lowercase hex")?;
            let key = self.verdict_key.ok_or(Refusal::NoVerdictKey)?;
            let now = self.clock.now_ms();
            let mut ledger = lock(&self.ledger);
            let run = ledger.runs.get_mut(&id).ok_or(Refusal::UnknownRun)?;
            if now >= run.closes_at_ms {
                return Err(Refusal::RunClosed);
            }
            let rec = holden_eval::parse_record(record).map_err(|_| Refusal::VerdictRefused)?;
            let decided =
                holden_eval::replay(&rec, Some(&key)).map_err(|_| Refusal::VerdictRefused)?;
            let r = &rec.body.reveal;
            let same_run = r.run_id == run_id
                && rec.body.plan_commit == hex::encode(run.plan_commit.0)
                && rec.body.evaluated_commit == hex::encode(run.evaluated_commit.as_bytes())
                && r.commitment == hex::encode(run.commitment.0)
                && r.server_nonce == hex::encode(run.server_nonce.0)
                && r.builder_nonce == hex::encode(run.builder_nonce.0)
                && r.output == hex::encode(run.reveal.output.0)
                && r.proof == hex::encode(run.reveal.proof.0)
                && r.epoch_public_key == hex::encode(self.epoch.public().to_bytes());
            if !same_run {
                return Err(Refusal::VerdictMismatch);
            }
            let listed: BTreeMap<String, String> = rec
                .body
                .observations
                .iter()
                .zip(&rec.body.observation_digests)
                .map(|(o, d)| (o.id.clone(), d.clone()))
                .collect();
            let submitted: BTreeMap<String, String> = run
                .observations
                .iter()
                .map(|(k, (d, _))| (k.clone(), hex::encode(d)))
                .collect();
            if listed.len() != rec.body.observations.len() || listed != submitted {
                return Err(Refusal::ObservationsMismatch);
            }
            let decision = match decided.decision {
                holden_eval::Decision::Pass => "pass",
                holden_eval::Decision::Fail => "fail",
            };
            // Slice-5: append to the signed ledger first (a refused append
            // refuses the verdict, and the run stays open), then measure the
            // verdict's authority from the ledger's controls.
            let authority = match &self.signed {
                Some(s) => {
                    let mut s = lock(s);
                    let Signed { ledger: l, key } = &mut *s;
                    l.append_json(
                        key,
                        holden_ledger::records::kind::VERDICT,
                        &holden_ledger::records::VerdictEntry::of(
                            run_id,
                            &rec,
                            self.generated
                                .iter()
                                .find(|(commit, _)| *commit == run.plan_commit)
                                .map(|(_, plan)| plan),
                        ),
                    )
                    .map_err(|_| Refusal::LedgerFailed)?;
                    let entries = l.entries().map_err(|_| Refusal::LedgerFailed)?;
                    let measured = holden_ledger::Measurements::from_entries(&entries)
                        .map_err(|_| Refusal::LedgerFailed)?;
                    holden_ledger::verdict_authority(&rec, &measured, &self.bars)
                }
                None => holden_ledger::verdict_authority(
                    &rec,
                    &holden_ledger::Measurements::default(),
                    &self.bars,
                ),
            };
            let summary = VerdictSummary {
                sha256: rec.sha256(),
                decision,
                at_ms: now,
            };
            // A verdict closes the run early.
            run.closes_at_ms = now;
            run.verdict = Some(summary.clone());
            Ok(json!({
                "run_id": run_id,
                "decision": summary.decision,
                "reasons": decided.reasons,
                "authority": authority.authority,
                "authority_capped_by": authority.capped_by,
                "verdict_sha256": summary.sha256,
                "closed_at_ms": now,
            }))
        })();
        self.record("verdict", caller, &out);
        out
    }

    /// `telemetry`: evaluator only (it names checks and lens concepts,
    /// which are sealed). Every measured catch and false-fail rate in the
    /// signed ledger with its sample size and 95% Wilson bounds, each
    /// check's authority under the configured bars, the latest lens weights,
    /// holdout versions and re-plan items (`holden_ledger::telemetry`).
    ///
    /// # Errors
    /// A [`Refusal`].
    pub fn telemetry(&self, caller: &Caller) -> Result<Value, Refusal> {
        let out = (|| {
            Self::require_evaluator(caller)?;
            let signed = self.signed.as_ref().ok_or(Refusal::NoLedger)?;
            let entries = lock(signed)
                .ledger
                .entries()
                .map_err(|_| Refusal::LedgerFailed)?;
            holden_ledger::telemetry::report(&entries, &self.bars)
                .map_err(|_| Refusal::LedgerFailed)
        })();
        self.record("telemetry", caller, &out);
        out
    }
}
