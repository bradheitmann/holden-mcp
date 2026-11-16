//! The self-composing MCP workflows (slice-8).
//!
//! A harness needs one call per standard workflow. The workflow uses the
//! same [`Core`] operations as the audit tools, while the implementer sees
//! only public promises, fixed diagnostics, measured status and owner-action
//! commands. Sealed steps, values and thresholds never enter W1–W3 answers.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use holden_eval::rules::Diagnostic;
use holden_eval::{EvalRequest, Evaluator, McpClient, Toolchain, VerdictRecord, notes_binding};
use holden_gen::{GenPlan, unseal_plan};
use holden_seal::{EvaluatorKey, FileKeyStore, KeyStore, PlanCommit, SealKey, plan_commit};
use rmcp::RoleServer;
use rmcp::model::ProgressNotificationParam;
use rmcp::service::RequestContext;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::{Role, ServerConfig};
use crate::ledger::{Caller, Core, Refusal};

/// Initialize instructions: the complete public operating contract.
pub const INSTRUCTIONS: &str = "Holden publishes the why and seals the how. Use these workflows in order; do not call owner-only operations as an implementer.\n\
W1 ONBOARD: call holden_onboard {host} to discover the owner's verbatim promises and proposed cognitions. If it returns OWNER_ACTION_REQUIRED, stop and ask the owner to run the exact owner_command; never approve your own work.\n\
W2 CHECK: after approval, commit the product change and call holden_check {host}. The server composes brief, begin_run, reveal_params, boundary observations, verdict and audit_reveal on the current commit, or on the commit named by optional evaluated_commit. It returns the fixed diagnostic only; never invent a pass after an error. Clients SHOULD pass a progress token: W2 runs a real sealed holdout for minutes and reports each plan's phase.\n\
W3 EXPLAIN: when W2 fails, call holden_explain {host, run_id}. Apply the promise and consequence it returns. It never reveals a sealed step, value or threshold.\n\
W4 STATUS: call holden_status {host} for telemetry, measured authority and pending owner actions. Authority bars are owner-set; stop and ask the owner to approve cognitions or change bars. Low-level tools remain available for audit: brief, commit_nonce, begin_run, reveal_params, submit_observation, verdict, audit_reveal and telemetry.";

/// The four workflow tool names, in workflow order.
pub const WORKFLOW_TOOLS: [&str; 4] = [
    "holden_onboard",
    "holden_check",
    "holden_explain",
    "holden_status",
];

/// A host argument shared by W1, W2 and W4.
#[derive(Clone, Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct HostArgs {
    /// The embedded host repository's absolute path.
    pub host: PathBuf,
}

/// W2 arguments.
#[derive(Clone, Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct CheckArgs {
    /// The embedded host repository's absolute path.
    pub host: PathBuf,
    /// An explicit commit to evaluate instead of the client's HEAD.
    #[serde(default)]
    pub evaluated_commit: Option<String>,
}

/// W3 arguments.
#[derive(Clone, Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct ExplainArgs {
    /// The embedded host repository's absolute path.
    pub host: PathBuf,
    /// The closed run id returned by W2 (64 lowercase hex).
    pub run_id: String,
}

#[derive(Debug, Deserialize)]
struct PublicHost {
    format: String,
    state: PathBuf,
    planning: String,
    promises: Vec<PublicPromise>,
    cognitions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PublicPromise {
    id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct AgentConfig {
    cargo: PathBuf,
    confine_helper: PathBuf,
}

/// The only phases that may be named in a progress notification. In
/// particular, a phase never carries an observation, verdict, parameter,
/// plan body or error string from the sealed evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Preparing,
    Building,
    Evaluating,
    VerdictRecorded,
    AuditClosed,
    NonceRearmed,
}

impl Phase {
    fn text(self, plan: Option<(usize, usize)>) -> String {
        match (self, plan) {
            (Self::Preparing, _) => "preparing".to_owned(),
            (Self::Building, Some((index, count))) => {
                format!("plan {index}/{count}: building")
            }
            (Self::Evaluating, Some((index, count))) => {
                format!("plan {index}/{count}: evaluating")
            }
            (Self::VerdictRecorded, Some((index, count))) => {
                format!("plan {index}/{count}: verdict recorded")
            }
            (Self::AuditClosed, Some((index, count))) => {
                format!("plan {index}/{count}: audit closed")
            }
            (Self::NonceRearmed, Some((index, count))) => {
                format!("plan {index}/{count}: ready for next run")
            }
            _ => "phase unavailable without a plan index".to_owned(),
        }
    }
}

/// A bounded bridge from the real blocking work to the request's owned async
/// notification loop. With no token, no progress machinery is spawned.
#[derive(Clone)]
pub(crate) struct WorkflowProgress {
    sender: tokio::sync::mpsc::Sender<(Phase, Option<(usize, usize)>)>,
}

impl WorkflowProgress {
    const TICK: Duration = Duration::from_secs(30);

    pub(crate) fn for_request(
        context: &RequestContext<RoleServer>,
    ) -> (Option<Self>, Option<tokio::task::JoinHandle<()>>) {
        let Some(token) = context.meta.get_progress_token() else {
            return (None, None);
        };
        let (sender, mut receiver) = tokio::sync::mpsc::channel(32);
        let peer = context.peer.clone();
        let cancelled = context.ct.clone();
        let handle = tokio::spawn(async move {
            // Inactive builds and evaluations are inside this exact interval,
            // below the unchanged 120-second byte-idle relay deadline.
            let mut tick = tokio::time::interval(Self::TICK);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            tick.tick().await; // The first interval is a full 30 seconds.
            let mut phase = (Phase::Preparing, None);
            let mut step = 0_u32;
            loop {
                tokio::select! {
                    biased;
                    () = cancelled.cancelled() => break,
                    event = receiver.recv() => {
                        let Some(event) = event else { break };
                        phase = event;
                    }
                    _ = tick.tick() => {}
                }
                let Some(next) = step.checked_add(1) else {
                    break;
                };
                step = next;
                // This is rmcp's real notifications/progress method, not a
                // synthetic line on the tool response stream. Each tick names
                // the real phase the concurrent sealed work is still in.
                let notification = ProgressNotificationParam::new(token.clone(), f64::from(step))
                    .with_message(phase.0.text(phase.1));
                if peer.notify_progress(notification).await.is_err() {
                    break;
                }
            }
        });
        (Some(Self { sender }), Some(handle))
    }

    fn report(&self, phase: Phase, plan: Option<(usize, usize)>) -> WResult<()> {
        self.sender.blocking_send((phase, plan)).map_err(|_| {
            WorkflowError::new(
                "could_not_run",
                "the progress notification transport closed during the holdout",
            )
        })
    }
}

#[derive(Debug)]
pub struct WorkflowError {
    pub code: &'static str,
    pub message: &'static str,
}

impl WorkflowError {
    pub(crate) const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({"refusal": self.code, "message": self.message})
    }
}

type WResult<T> = Result<T, WorkflowError>;

fn error<T>(code: &'static str, message: &'static str) -> WResult<T> {
    Err(WorkflowError::new(code, message))
}

fn user_state_root() -> Option<PathBuf> {
    let base = if let Some(root) = std::env::var_os("AGENT_PRIVATE_ROOT").filter(|v| !v.is_empty())
    {
        PathBuf::from(root)
    } else if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        PathBuf::from(xdg).join("agent-private")
    } else {
        std::env::home_dir()?.join(".local/share/agent-private")
    };
    Some(base.join("holden-mcp"))
}

fn host_config(host: &Path) -> WResult<(PathBuf, PublicHost)> {
    if !host.is_absolute() {
        return error("bad_input", "host must be an absolute path");
    }
    let host = fs::canonicalize(host).map_err(|_| {
        WorkflowError::new("host_unavailable", "the host repository cannot be opened")
    })?;
    let bytes = fs::read(host.join(".holden/config.json")).map_err(|_| {
        WorkflowError::new(
            "not_embedded",
            "the host has no holden public run file; ask the owner to embed and approve holden",
        )
    })?;
    let config: PublicHost = serde_json::from_slice(&bytes).map_err(|_| {
        WorkflowError::new("not_embedded", "the host's public run file is malformed")
    })?;
    if config.format != "holden.embed/1"
        || config.state.as_os_str().is_empty()
        || config.promises.len() != config.cognitions.len()
        || config.planning.is_empty()
    {
        return error("not_embedded", "the host's public run file is not valid");
    }
    let mut config = config;
    if !config.state.is_absolute() {
        if config
            .state
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return error("not_embedded", "the host's public state name is not valid");
        }
        let root = user_state_root().ok_or_else(|| {
            WorkflowError::new(
                "host_unavailable",
                "the user state directory cannot be resolved",
            )
        })?;
        let alias = root.join(&config.state);
        config.state = fs::canonicalize(&alias).map_err(|_| {
            WorkflowError::new("host_unavailable", "the agent state cannot be opened")
        })?;
    }
    Ok((host, config))
}

fn approval_path(host: &Path, cognition: &str) -> PathBuf {
    host.join(".holden/cognitions")
        .join(format!("{cognition}.approval.json"))
}

fn owner_required(host: &Path, public: &PublicHost) -> Option<Value> {
    let pending = pending_approvals(host, public);
    (!pending.is_empty()).then(|| {
        json!({
            "status": "OWNER_ACTION_REQUIRED",
            "pending_cognition_ids": pending.iter().map(|i| &public.cognitions[*i]).collect::<Vec<_>>(),
            "owner_command": owner_command(host),
            "next": "stop and ask the owner to perform owner_command; an implementer cannot approve or change bars",
        })
    })
}

fn pending_approvals(host: &Path, config: &PublicHost) -> Vec<usize> {
    config
        .cognitions
        .iter()
        .enumerate()
        .filter_map(|(i, id)| (!approval_path(host, id).is_file()).then_some(i))
        .collect()
}

fn owner_command(host: &Path) -> String {
    format!("holden approve --host {}", host.display())
}

fn proposals(host: &Path, config: &PublicHost) -> Vec<Value> {
    config
        .promises
        .iter()
        .zip(&config.cognitions)
        .map(|(p, cognition)| {
            json!({
                "promise_id": p.id,
                "promise": p.text,
                "cognition_id": cognition,
                "approved": approval_path(host, cognition).is_file(),
            })
        })
        .collect()
}

/// W1: discover promises and proposed cognitions; approval remains the owner's.
pub fn onboard(host: &Path) -> WResult<Value> {
    let (host, config) = host_config(host)?;
    let pending = pending_approvals(&host, &config);
    let proposals = proposals(&host, &config);
    if pending.is_empty() {
        Ok(json!({
            "workflow": "W1 ONBOARD",
            "status": "OWNER_APPROVED",
            "host": host,
            "proposals": proposals,
            "next": "W2 CHECK: make a commit, then call holden_check",
        }))
    } else {
        Ok(json!({
            "workflow": "W1 ONBOARD",
            "status": "OWNER_ACTION_REQUIRED",
            "host": host,
            "proposals": proposals,
            "pending_cognition_ids": pending.iter().map(|i| &config.cognitions[*i]).collect::<Vec<_>>(),
            "owner_command": owner_command(&host),
            "next": "stop and ask the owner to run owner_command; an implementer cannot approve",
        }))
    }
}

fn refusal(r: Refusal) -> WorkflowError {
    WorkflowError::new(
        "workflow_refused",
        match r {
            Refusal::NoCommitment | Refusal::ServerNonceUsed => {
                "no unused pre-commit nonce is armed for this plan; start the holden agent before the product commit"
            }
            Refusal::RunOpen => "the run is still open; wait for its verdict before W3 EXPLAIN",
            Refusal::UnknownRun => "the run id is unknown to this server",
            Refusal::ForbiddenRole | Refusal::UnmappedCaller => {
                "this caller's role may not perform the required audit operation"
            }
            _ => "a low-level holden operation refused the workflow; no pass is inferred",
        },
    )
}

fn load_plan(config: &ServerConfig, commit: PlanCommit) -> WResult<GenPlan> {
    let store = FileKeyStore::open(config.key_dir()).map_err(|_| {
        WorkflowError::new(
            "workflow_unavailable",
            "the evaluator key store cannot be opened",
        )
    })?;
    let seal = SealKey::new(store.get(config.seal_key()).map_err(|_| {
        WorkflowError::new("workflow_unavailable", "the plan seal key is unavailable")
    })?);
    for entry in fs::read_dir(config.plans_dir())
        .map_err(|_| WorkflowError::new("workflow_unavailable", "sealed plans cannot be read"))?
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else { continue };
        let Ok(file) = serde_json::from_slice::<crate::SealedPlanFile>(&bytes) else {
            continue;
        };
        let Ok(sealed) = file.to_sealed() else {
            continue;
        };
        if plan_commit(&sealed) != commit {
            continue;
        }
        return unseal_plan(&seal, &sealed, &commit).map_err(|_| {
            WorkflowError::new(
                "workflow_unavailable",
                "the registered sealed plan cannot be opened",
            )
        });
    }
    error(
        "unknown_plan",
        "no registered generated plan has this plan_commit",
    )
}

fn promise_for(config: &PublicHost, plan: &GenPlan) -> String {
    let id = plan.provenance.promise.to_hex();
    config.promises.iter().find(|p| p.id == id).map_or_else(
        || "the owner's approved promise".to_owned(),
        |p| p.text.clone(),
    )
}

fn selected_plans(
    core: &Core,
    config: &ServerConfig,
    public: &PublicHost,
) -> WResult<Vec<(PlanCommit, GenPlan)>> {
    let brief = core.brief(&Caller {
        role: Some(Role::Evaluator),
        uid: None,
        via: crate::Via::Socket,
    });
    let mut out = Vec::new();
    for value in brief["plans"].as_array().into_iter().flatten() {
        let Some(hex_id) = value.as_str() else {
            continue;
        };
        let Ok(bytes) = hex::decode(hex_id) else {
            continue;
        };
        let Ok(bytes) = <[u8; 32]>::try_from(bytes) else {
            continue;
        };
        let commit = PlanCommit(bytes);
        let plan = load_plan(config, commit)?;
        if public
            .promises
            .iter()
            .any(|p| p.id == plan.provenance.promise.to_hex())
        {
            out.push((commit, plan));
        }
    }
    if out.is_empty() {
        return error(
            "no_approved_holdout",
            "no owner-approved generated holdout is registered; complete W1 and start the holden agent",
        );
    }
    Ok(out)
}

fn write_run_json<T: Serialize>(state: &Path, run_id: &str, name: &str, value: &T) -> WResult<()> {
    let dir = state.join("runs").join(run_id);
    fs::create_dir_all(&dir)
        .map_err(|_| WorkflowError::new("record_failed", "the run record cannot be written"))?;
    let bytes = serde_json::to_vec(value)
        .map_err(|_| WorkflowError::new("record_failed", "the run record cannot be encoded"))?;
    fs::write(dir.join(name), bytes)
        .map_err(|_| WorkflowError::new("record_failed", "the run record cannot be written"))
}

fn write_record(state: &Path, run_id: &str, record: &VerdictRecord) -> WResult<()> {
    write_run_json(state, run_id, "record.json", record)
}

/// The sealed configuration of this embedded server, not the caller's
/// connection configuration. The evaluator config is the only surface that
/// names ledger keys and the plan seal. Implementer W2/W4 never return it.
fn server_config(public: &PublicHost) -> WResult<Arc<ServerConfig>> {
    let path = public.state.join("server/evaluator.json");
    ServerConfig::load(&path).map(Arc::new).map_err(|_| {
        WorkflowError::new(
            "workflow_unavailable",
            "the embedded server's sealed configuration is unavailable",
        )
    })
}

fn evaluator_caller() -> Caller {
    Caller {
        role: Some(Role::Evaluator),
        uid: None,
        via: crate::Via::Socket,
    }
}

/// Aggregate measured authority only: no raw ledger records, nonces,
/// parameter outputs, proofs, plan bodies or key material.
fn public_telemetry(core: &Core, config: &ServerConfig) -> WResult<Value> {
    let report = core.telemetry(&evaluator_caller()).map_err(refusal)?;
    Ok(json!({
        "format": report["format"],
        "controls": report["controls"],
        "verdicts": report["verdicts"],
        "checks": report["checks"],
        "lenses": report["lenses"],
        "weights": report["weights"],
        "cognitions": report["cognitions"],
        "plans": report["plans"],
        "integrity": report["integrity"],
        "authority_bars": config.bars(),
    }))
}

/// Recompute the signed feedback weights from the ledger after W2's real
/// runs. The corpus remains keyed and sealed-side; only aggregate weights
/// are exposed by the public telemetry projection.
fn reweigh(public: &PublicHost, config: &ServerConfig) -> WResult<()> {
    let ledger_config = config.ledger().ok_or_else(|| {
        WorkflowError::new(
            "workflow_unavailable",
            "W2 requires the signed evaluator ledger",
        )
    })?;
    let store = FileKeyStore::open(config.key_dir()).map_err(|_| {
        WorkflowError::new("feedback_failed", "the feedback key store cannot be opened")
    })?;
    let key = EvaluatorKey::load(&store, &ledger_config.key).map_err(|_| {
        WorkflowError::new("feedback_failed", "the feedback signing key is unavailable")
    })?;
    let corpus = holden_gen::DrawCorpus::load(&public.state.join("lens")).map_err(|_| {
        WorkflowError::new(
            "feedback_failed",
            "the feedback lens corpus cannot be loaded",
        )
    })?;
    let mut ledger = holden_ledger::Ledger::open(&ledger_config.db).map_err(|_| {
        WorkflowError::new("feedback_failed", "the feedback ledger cannot be opened")
    })?;
    holden_ledger::feedback::reweigh(&mut ledger, &key, &corpus)
        .map(|_| ())
        .map_err(|_| {
            WorkflowError::new(
                "feedback_failed",
                "the run was recorded but its feedback weights could not be signed",
            )
        })
}

/// The blocking part of W2. The internal client is a real rmcp client on a
/// real evaluator-only Unix socket backed by this same core.
pub(crate) fn check_blocking(
    core: Arc<Core>,
    _config: Arc<ServerConfig>,
    host: PathBuf,
    evaluated_commit: Option<String>,
    socket: &Path,
    progress: Option<&WorkflowProgress>,
) -> WResult<Value> {
    if let Some(progress) = progress {
        progress.report(Phase::Preparing, None)?;
    }
    let (host, public) = host_config(&host)?;
    if let Some(required) = owner_required(&host, &public) {
        return Ok(required);
    }
    let config = server_config(&public)?;
    let agent_bytes = fs::read(public.state.join("agent.json")).map_err(|_| {
        WorkflowError::new(
            "workflow_unavailable",
            "the embedded evaluator is not configured",
        )
    })?;
    let agent: AgentConfig = serde_json::from_slice(&agent_bytes).map_err(|_| {
        WorkflowError::new(
            "workflow_unavailable",
            "the embedded evaluator config is malformed",
        )
    })?;
    let git_home = public.state.join("git-home");
    fs::create_dir_all(&git_home).map_err(|_| {
        WorkflowError::new("workflow_unavailable", "the evaluator state is unavailable")
    })?;
    let git = holden_eval::git::Git::new(&host, &git_home);
    let revision = evaluated_commit.as_deref().unwrap_or("HEAD");
    let head = git.resolve_commit(revision).map_err(|_| {
        WorkflowError::new(
            "no_current_commit",
            "the requested commit does not name a commit in the host repository",
        )
    })?;
    let baseline = git
        .resolve_commit(&format!("{head}^1"))
        .unwrap_or_else(|_| head.clone());
    if config.ledger().is_none() {
        return error(
            "workflow_unavailable",
            "W2 requires the signed evaluator ledger",
        );
    }
    let store = FileKeyStore::open(config.key_dir()).map_err(|_| {
        WorkflowError::new(
            "workflow_unavailable",
            "the evaluator key store cannot be opened",
        )
    })?;
    let evaluator_key_name = holden_seal::KeyName::new("evaluator").map_err(|_| {
        WorkflowError::new(
            "workflow_unavailable",
            "the evaluator key name is unavailable",
        )
    })?;
    let key = EvaluatorKey::load(&store, &evaluator_key_name).map_err(|_| {
        WorkflowError::new(
            "workflow_unavailable",
            "the evaluator signing key is unavailable",
        )
    })?;
    let evaluator = Evaluator::new(
        key,
        Toolchain { cargo: agent.cargo },
        agent.confine_helper,
        public.state.join("worlds"),
        notes_binding(),
    );
    let plans = selected_plans(&core, &config, &public)?;
    let plan_count = plans.len();
    let host = host.as_path();
    let head = head.as_str();
    let baseline = baseline.as_str();
    let runs = std::thread::scope(|scope| {
        let handles: Vec<_> = plans
            .into_iter()
            .enumerate()
            .map(|(index, (commit, plan))| {
                let core = &core;
                let evaluator = &evaluator;
                let public = &public;
                scope.spawn(move || {
                    evaluate_one(
                        core,
                        public,
                        host,
                        evaluator,
                        socket,
                        head,
                        baseline,
                        commit,
                        plan,
                        progress,
                        (index + 1, plan_count),
                    )
                })
            })
            .collect();
        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.join() {
                Ok(result) => results.push(result?),
                Err(_) => {
                    return Err(WorkflowError::new(
                        "could_not_run",
                        "an evaluating thread stopped unexpectedly; this is not a pass",
                    ));
                }
            }
        }
        Ok(results)
    })?;
    reweigh(&public, &config)?;
    Ok(json!({
        "workflow": "W2 CHECK",
        "status": "CHECK_COMPLETE",
        "evaluated_commit": head,
        "runs": runs,
        "telemetry": public_telemetry(&core, &config)?,
        "next": "for each failed run, call holden_explain with its run_id; then call holden_status",
    }))
}

// One approved plan executes on its own scoped native thread. The caller is
// server-side evaluator and every operation is still the real low-level flow.
#[allow(clippy::too_many_arguments)]
fn evaluate_one(
    core: &Core,
    public: &PublicHost,
    host: &Path,
    evaluator: &Evaluator,
    socket: &Path,
    head: &str,
    baseline: &str,
    commit: PlanCommit,
    plan: GenPlan,
    progress: Option<&WorkflowProgress>,
    plan_number: (usize, usize),
) -> WResult<Value> {
    let phase = |phase| match progress {
        Some(progress) => progress.report(phase, Some(plan_number)),
        None => Ok(()),
    };
    phase(Phase::Building)?;
    let pc = hex::encode(commit.0);
    let builder = holden_seal::random_bytes::<32>().map_err(|_| {
        WorkflowError::new(
            "no_randomness",
            "the OS random source failed; no holdout ran",
        )
    })?;
    let begun = core
        .begin_run(&evaluator_caller(), &pc, head, &hex::encode(builder))
        .map_err(refusal)?;
    let run_id = begun["run_id"].as_str().unwrap_or_default().to_owned();
    let request = EvalRequest {
        repo: host,
        plan: &plan,
        plan_commit: commit,
        evaluated_commit: head,
        baseline_commit: baseline,
    };
    let client = McpClient::connect(socket).map_err(|_| {
        WorkflowError::new(
            "workflow_unavailable",
            "the plan's evaluator transport cannot be opened",
        )
    })?;
    phase(Phase::Evaluating)?;
    let evaluated = evaluator
        .evaluate_run(&client, &request, &run_id)
        .map_err(|error| {
            if std::env::var_os("HOLDEN_E2E_TRANSPORT_DIAGNOSTIC").is_some() {
                eprintln!("diagnostic evaluator failure: plan={pc} error={error}");
            }
            WorkflowError::new(
                "could_not_run",
                "the sealed evaluation could not finish; this is not a pass",
            )
        })?;
    let Eval {
        record,
        server,
        world,
    } = Eval::from(evaluated);
    world.remove();
    let server = server.map_err(|_| {
        WorkflowError::new(
            "verdict_refused",
            "the server refused the signed verdict; this is not a pass",
        )
    })?;
    phase(Phase::VerdictRecorded)?;
    let audit = core
        .audit_reveal(&evaluator_caller(), &run_id)
        .map_err(refusal)?;
    if audit["verdict"]["decision"].as_str() != Some(record_decision(&record)) {
        return error(
            "audit_mismatch",
            "the closed audit does not match the verdict; no result is returned",
        );
    }
    write_record(&public.state, &run_id, &record)?;
    write_run_json(&public.state, &run_id, "audit.json", &audit)?;
    phase(Phase::AuditClosed)?;
    core.commit_nonce(&evaluator_caller(), &pc)
        .map_err(refusal)?;
    phase(Phase::NonceRearmed)?;
    Ok(json!({
        "run_id": run_id,
        "plan_commit": pc,
        "promise": promise_for(public, &plan),
        "decision": record_decision(&record),
        "authority": server["authority"],
        "diagnostics": record.body.decided.diagnostics,
    }))
}

// A small destructuring adapter keeps the evaluation fields explicit without
// exposing any of them in the workflow response.
struct Eval {
    record: VerdictRecord,
    server: Result<Value, holden_eval::ToolRefusal>,
    world: holden_eval::world::World,
}

impl From<holden_eval::Evaluation> for Eval {
    fn from(e: holden_eval::Evaluation) -> Self {
        Self {
            record: e.record,
            server: e.server,
            world: e.world,
        }
    }
}

fn record_decision(record: &VerdictRecord) -> &'static str {
    match record.body.decided.decision {
        holden_eval::Decision::Pass => "pass",
        holden_eval::Decision::Fail => "fail",
    }
}

/// W3: audit closure, then return only the fixed consequence and promise.
pub fn explain(
    core: &Core,
    _caller: &Caller,
    _config: &ServerConfig,
    host: &Path,
    run_id: &str,
) -> WResult<Value> {
    let (host, public) = host_config(host)?;
    if let Some(required) = owner_required(&host, &public) {
        return Ok(required);
    }
    let config = server_config(&public)?;
    let audit = core
        .audit_reveal(&evaluator_caller(), run_id)
        .map_err(refusal)?;
    let decision = audit["verdict"]["decision"].as_str().unwrap_or_default();
    if decision.is_empty() {
        return error(
            "no_verdict",
            "the closed run has no accepted verdict; there is nothing to explain",
        );
    }
    let bytes =
        fs::read(public.state.join("runs").join(run_id).join("record.json")).map_err(|_| {
            WorkflowError::new(
                "record_unavailable",
                "the run's signed diagnostic record is unavailable",
            )
        })?;
    let record: VerdictRecord = serde_json::from_slice(&bytes).map_err(|_| {
        WorkflowError::new(
            "record_unavailable",
            "the run's signed diagnostic record is malformed",
        )
    })?;
    holden_eval::replay(&record, config.verdict_key().as_ref()).map_err(|_| {
        WorkflowError::new(
            "record_unavailable",
            "the run's signed diagnostic record does not replay",
        )
    })?;
    let pc = record.body.plan_commit.as_str();
    let bytes = hex::decode(pc).map_err(|_| {
        WorkflowError::new("record_unavailable", "the run record names no valid plan")
    })?;
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| {
        WorkflowError::new("record_unavailable", "the run record names no valid plan")
    })?;
    let plan = load_plan(&config, PlanCommit(bytes))?;
    let diagnostics: Vec<&Diagnostic> = record.body.decided.diagnostics.iter().collect();
    Ok(json!({
        "workflow": "W3 EXPLAIN",
        "status": if decision == "fail" { "FIX_REQUIRED" } else { "NO_FAILURE" },
        "run_id": run_id,
        "decision": decision,
        "promise": promise_for(&public, &plan),
        "fixes": diagnostics.iter().map(|d| json!({
            "promise": d.promise_text,
            "consequence": d.consequence,
        })).collect::<Vec<_>>(),
        "next": if decision == "fail" { "repair the promise consequence, commit, and call holden_check" } else { "call holden_status" },
    }))
}

/// W4: public pending actions plus evaluator telemetry and measured authority.
pub fn status(core: &Core, caller: &Caller, _config: &ServerConfig, host: &Path) -> WResult<Value> {
    let (host, public) = host_config(host)?;
    if let Some(required) = owner_required(&host, &public) {
        return Ok(required);
    }
    let config = server_config(&public)?;
    let brief = core.brief(caller);
    let pending = pending_approvals(&host, &public);
    let telemetry = public_telemetry(core, &config)?;
    Ok(json!({
        "workflow": "W4 STATUS",
        "status": if pending.is_empty() { "OWNER_ACTIONS_CLEAR" } else { "OWNER_ACTION_REQUIRED" },
        "host": host,
        "registered_plans": brief["plans"],
        "authority_bars": config.bars(),
        "telemetry": telemetry,
        "pending_owner_actions": pending.iter().map(|i| json!({
            "action": "approve_cognition",
            "cognition_id": public.cognitions[*i],
            "owner_command": owner_command(&host),
        })).collect::<Vec<_>>(),
        "owner_controls": "approval and authority bars are owner-only; implementer calls cannot change either",
    }))
}
