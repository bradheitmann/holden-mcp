//! `holden jit`: the just-in-time sealed holdout run the post-commit shim
//! starts (slice-7 P3). All of its logic is here, none in the hook.
//!
//! For every plan the agent serves, on the commit that was just made:
//!
//! 1. the run uses a nonce the evaluator committed (`commit_nonce`) before
//!    this commit existed: the armed commitment for the plan, recorded at a
//!    HEAD that is a proper ancestor of the new commit;
//! 2. `begin_run` with the new commit's SHA, which must consume exactly
//!    that commitment;
//! 3. `reveal_params`: the moment-of-test parameters, verified (VRF proof,
//!    plan, commit, epoch key) and recorded;
//! 4. the holdout runs confined against the commit, built from source
//!    ([`holden_eval::Evaluator`]);
//! 5. the signed verdict is submitted, and the server appends it to the
//!    signed ledger;
//! 6. after the run closes, `audit_reveal` is verified;
//! 7. the host gets the fixed diagnostic (the promise and the consequence,
//!    never the how), and the next nonce is committed for the next commit.
//!
//! A cognition that is not admitted against the owner's published key
//! hosts no run. Each run's signed record, audit reveal, recorded
//! parameters and trace are written to the agent's state.

use std::path::{Path, PathBuf};

use holden_cognition::{
    Approval, ClientPromises, OwnerTrust, Promise, SituatedCognition, adapters::read_promises,
};
use holden_eval::params::ParamsWire;
use holden_eval::rules::Diagnostic;
use holden_eval::{EvalRequest, Evaluator, McpClient, ServerReveal, Toolchain, notes_binding};
use holden_gen::{GenPlan, unseal_plan};
use holden_mcp::{Role, SealedPlanFile};
use holden_seal::{EvaluatorKey, PlanCommit, SealKey, plan_commit};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::host::{HostConfig, approval_file, cognition_file};
use crate::state::{Armed, State};
use crate::util::{HostGit, io_at, json_bytes, read_json, write_if_changed};
use crate::{Error, refused};

/// The trace of one run (`runs/<run id>/trace.json`, agent-private).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceFile {
    /// `holden.trace/1`.
    pub format: String,
    /// The run id.
    pub run_id: String,
    /// The host's directory name.
    pub host: String,
    /// The evaluated commit.
    pub evaluated_commit: String,
    /// The baseline commit (its parent, or itself for a root commit).
    pub baseline_commit: String,
    /// The owner's promise, as a span of the planning document.
    pub promise: Promise,
    /// The approved cognition's public half.
    pub cognition: SituatedCognition,
    /// The owner's approval of it.
    pub approval: Approval,
    /// The plan id, hex.
    pub plan_id: String,
    /// The plan commit, hex.
    pub plan_commit: String,
    /// The commitment the run consumed, made before the commit existed.
    pub armed: Armed,
    /// The moment-of-test parameters revealed for the run.
    pub params: ParamsWire,
    /// The verdict.
    pub verdict: TraceVerdict,
    /// The verdict's ledger record.
    pub ledger: TraceLedger,
}

/// The verdict part of a trace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceVerdict {
    /// SHA-256 of the signed record.
    pub sha256: String,
    /// `pass` or `fail`.
    pub decision: String,
    /// The typed fail reasons.
    pub reasons: Vec<String>,
    /// The fixed diagnostics.
    pub diagnostics: Vec<Diagnostic>,
    /// The server's answer (decision, authority).
    pub server: Value,
    /// The evaluator's public key, hex.
    pub signer: String,
}

/// The ledger part of a trace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceLedger {
    /// The record's sequence number.
    pub seq: u64,
    /// The record's hash.
    pub hash: String,
}

/// One finished run.
#[derive(Clone, Debug)]
pub struct PlanRun {
    /// The run id.
    pub run_id: String,
    /// The plan id, hex.
    pub plan_id: String,
    /// The promise, in the owner's words.
    pub promise: String,
    /// `pass` or `fail`.
    pub decision: String,
    /// The fixed diagnostics.
    pub diagnostics: Vec<Diagnostic>,
    /// The signed record.
    pub record: PathBuf,
}

/// What a just-in-time run did.
#[derive(Clone, Debug)]
pub struct JitReport {
    /// The evaluated commit.
    pub commit: String,
    /// The finished runs.
    pub runs: Vec<PlanRun>,
    /// Plans that did not run, and why (fixed text).
    pub refused: Vec<String>,
}

/// The plan files the agent serves, unsealed (the evaluator is the sealed
/// side), keyed by their commits.
fn served_plans(
    state: &State,
    seal: &SealKey,
    registered: &[String],
) -> Result<Vec<(GenPlan, PlanCommit)>, Error> {
    let mut out = Vec::new();
    for f in state.sealed_plan_files()? {
        let wire: SealedPlanFile = read_json(&f)?;
        let sealed = wire.to_sealed().map_err(refused)?;
        let commit = plan_commit(&sealed);
        if !registered.contains(&hex::encode(commit.0)) {
            continue;
        }
        let plan = unseal_plan(seal, &sealed, &commit).map_err(|e| refused(e.to_string()))?;
        out.push((plan, commit));
    }
    Ok(out)
}

/// The reveal of an `audit_reveal` answer: the answer without the run
/// record's step times and the verdict summary the audit adds.
#[must_use]
pub fn reveal_part(audit: &Value) -> Value {
    let mut v = audit.clone();
    if let Some(o) = v.as_object_mut() {
        o.remove("record");
        o.remove("verdict");
    }
    v
}

fn call(client: &McpClient, tool: &str, args: Value) -> Result<Value, Error> {
    client
        .call(tool, args)
        .map_err(|r| refused(format!("{tool}: {r}")))
}

/// The admitted (promise, cognition, approval) a plan was generated from.
pub(crate) fn admitted_for(
    host: &Path,
    client: &ClientPromises,
    trust: &OwnerTrust,
    plan: &GenPlan,
) -> Result<(Promise, SituatedCognition, Approval), Error> {
    let id = plan.provenance.cognition.to_hex();
    let cognition: SituatedCognition = read_json(&cognition_file(host, &id))?;
    let approval: Approval = read_json(&approval_file(host, &id))
        .map_err(|_| refused("the plan's cognition has no owner approval"))?;
    let (admitted, refusals) = client.admit(
        std::slice::from_ref(&cognition),
        std::slice::from_ref(&approval),
        trust,
    );
    let Some(h) = admitted.into_iter().next() else {
        let why = refusals
            .first()
            .map_or_else(|| "refused".to_owned(), |(_, e)| e.to_string());
        return Err(refused(format!(
            "the plan's cognition is not admitted: {why}"
        )));
    };
    if h.promise().id != plan.provenance.promise {
        return Err(refused("the plan's promise is not its cognition's"));
    }
    Ok((
        h.promise().clone(),
        h.cognition().clone(),
        h.approval().clone(),
    ))
}

/// Run the just-in-time holdouts on the host's HEAD (see the module docs).
///
/// # Errors
/// [`Error`] when the host, the agent or the state refuse before any run
/// starts. A plan that cannot run is reported in [`JitReport::refused`].
#[allow(clippy::too_many_lines)]
pub fn jit(host: &Path) -> Result<JitReport, Error> {
    let host = std::fs::canonicalize(host).map_err(io_at(host))?;
    let config = HostConfig::read(&host)?;
    let state = config.state_dir()?;
    let agent = state.agent_config()?;
    if agent.host != host {
        return Err(refused(
            "this host is not the one the agent was embedded in",
        ));
    }
    let git = HostGit::new(&host, &state.git_home());
    let head = git.run(&["rev-parse", "--verify", "HEAD^{commit}"])?;
    let parent = git
        .run(&["rev-parse", "--verify", "--quiet", "HEAD^1^{commit}"])
        .ok();
    let client = McpClient::connect(&state.socket(Role::Evaluator)).map_err(|_| {
        refused("the holden agent is not running (holden agent serve); no holdout ran")
    })?;
    let brief = call(&client, "brief", json!({}))?;
    let epoch = brief["epoch_public_key"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let registered: Vec<String> = brief["plans"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let seal = SealKey::new(state.key("seal")?);
    let plans = served_plans(&state, &seal, &registered)?;
    let mut report = JitReport {
        commit: head.clone(),
        runs: Vec::new(),
        refused: Vec::new(),
    };
    if plans.is_empty() {
        report.refused.push(
            "no owner-approved cognition hosts a holdout; no holdout ran (holden approve)"
                .to_owned(),
        );
        return Ok(report);
    }
    let trust = OwnerTrust::read_published(&agent.owner_store)
        .map_err(|_| refused("the owner has published no key; no holdout ran (holden approve)"))?;
    let client_planning = read_promises(&host).map_err(|e| refused(e.to_string()))?;
    let key = EvaluatorKey::from_material(&state.key("evaluator")?);
    let signer = hex::encode(key.public().0);
    let evaluator = Evaluator::new(
        key,
        Toolchain {
            cargo: agent.cargo.clone(),
        },
        agent.confine_helper.clone(),
        state.worlds_dir(),
        notes_binding(),
    );
    let mut armed = state.armed()?;
    for (plan, commit) in &plans {
        let pc = hex::encode(commit.0);
        let plan_id = plan.id.to_hex();
        let (promise, cognition, approval) =
            match admitted_for(&host, &client_planning, &trust, plan) {
                Ok(x) => x,
                Err(e) => {
                    report.refused.push(format!("plan {plan_id}: {e}"));
                    continue;
                }
            };
        // 1. A nonce committed before this commit existed.
        let Some(slot) = armed
            .iter()
            .position(|a| a.plan_commit == pc && a.consumed_by.is_none())
        else {
            report.refused.push(format!(
                "plan {plan_id}: no nonce was committed for it before this commit; no holdout ran"
            ));
            continue;
        };
        let before = armed[slot].head != head
            && (armed[slot].head == "none"
                || git.succeeds(&["merge-base", "--is-ancestor", &armed[slot].head, &head]));
        if !before {
            report.refused.push(format!(
                "plan {plan_id}: its nonce was not committed before this commit; no holdout ran"
            ));
            continue;
        }
        // 2. Open the run on the new commit with that commitment.
        let builder = holden_seal::random_bytes::<32>()
            .map_err(|_| refused("the OS random source failed"))?;
        let begun = call(
            &client,
            "begin_run",
            json!({
                "plan_commit": pc,
                "evaluated_commit": head,
                "builder_nonce": hex::encode(builder),
            }),
        )?;
        let run_id = begun["run_id"].as_str().unwrap_or_default().to_owned();
        if begun["commitment"].as_str() != Some(armed[slot].commitment.as_str()) {
            report.refused.push(format!(
                "plan {plan_id}: the server opened the run with another commitment; refused"
            ));
            continue;
        }
        armed[slot].consumed_by = Some(run_id.clone());
        state.write_armed(&armed)?;
        let run_dir = state.run_dir(&run_id);
        crate::util::private_dir(&run_dir)?;
        // 3. The moment-of-test parameters, from the server only.
        let revealed = call(&client, "reveal_params", json!({ "run_id": run_id }))?;
        let reveal = ServerReveal::from_reveal_body(&revealed, &epoch, commit, &head)
            .map_err(|e| refused(format!("reveal_params: {e}")))?;
        let params = reveal.wire().params.clone();
        write_if_changed(&run_dir.join("params.json"), &json_bytes(&params)?, 0o600)?;
        // 4–5. The confined run, the signed verdict, the ledger.
        let baseline = parent.clone().unwrap_or_else(|| head.clone());
        let req = EvalRequest {
            repo: &host,
            plan,
            plan_commit: *commit,
            evaluated_commit: &head,
            baseline_commit: &baseline,
        };
        let evaluation = match evaluator.evaluate_run(&client, &req, &run_id) {
            Ok(e) => e,
            Err(e) => {
                report.refused.push(format!(
                    "plan {plan_id}: the evaluation could not finish: {e}"
                ));
                continue;
            }
        };
        let record = evaluation.record;
        evaluation.world.remove();
        if record.body.reveal != *reveal.wire() {
            report.refused.push(format!(
                "plan {plan_id}: the run did not use the parameters revealed for it; refused"
            ));
            continue;
        }
        let record_path = run_dir.join("record.json");
        write_if_changed(&record_path, &json_bytes(&record)?, 0o600)?;
        let answer = match evaluation.server {
            Ok(a) => a,
            Err(r) => {
                report.refused.push(format!(
                    "plan {plan_id}: the server refused the verdict: {r}"
                ));
                continue;
            }
        };
        // 6. The audit reveal, after the run closed.
        let audit = call(&client, "audit_reveal", json!({ "run_id": run_id }))?;
        let audited = ServerReveal::from_reveal_body(&reveal_part(&audit), &epoch, commit, &head)
            .map_err(|e| refused(format!("audit_reveal: {e}")))?;
        if audited.wire() != reveal.wire() {
            return Err(refused("audit_reveal does not match the run's reveal"));
        }
        write_if_changed(&run_dir.join("audit.json"), &json_bytes(&audit)?, 0o600)?;
        let sha = record.sha256();
        let ledger = holden_ledger::Ledger::open(&state.ledger_db())
            .and_then(|l| l.entries())
            .map_err(|e| refused(format!("ledger: {e}")))?
            .into_iter()
            .find(|e| {
                e.kind == holden_ledger::records::kind::VERDICT
                    && e.decode::<holden_ledger::records::VerdictEntry>()
                        .is_ok_and(|v| v.verdict_sha256 == sha && v.run_id == run_id)
            })
            .ok_or_else(|| refused("the verdict is not in the signed ledger"))?;
        let decided = &record.body.decided;
        let decision = match decided.decision {
            holden_eval::Decision::Pass => "pass",
            holden_eval::Decision::Fail => "fail",
        };
        let trace = TraceFile {
            format: "holden.trace/1".to_owned(),
            run_id: run_id.clone(),
            host: config.host.clone(),
            evaluated_commit: head.clone(),
            baseline_commit: baseline,
            promise: promise.clone(),
            cognition,
            approval,
            plan_id: plan_id.clone(),
            plan_commit: pc.clone(),
            armed: armed[slot].clone(),
            params,
            verdict: TraceVerdict {
                sha256: sha,
                decision: decision.to_owned(),
                reasons: decided
                    .reasons
                    .iter()
                    .map(|r| r.as_str().to_owned())
                    .collect(),
                diagnostics: decided.diagnostics.clone(),
                server: answer,
                signer: signer.clone(),
            },
            ledger: TraceLedger {
                seq: ledger.seq,
                hash: ledger.hash,
            },
        };
        write_if_changed(&run_dir.join("trace.json"), &json_bytes(&trace)?, 0o600)?;
        report.runs.push(PlanRun {
            run_id: run_id.clone(),
            plan_id: plan_id.clone(),
            promise: promise.text.clone(),
            decision: decision.to_owned(),
            diagnostics: decided.diagnostics.clone(),
            record: record_path,
        });
        // 7. The next nonce, committed before the next commit exists.
        let next = call(&client, "commit_nonce", json!({ "plan_commit": pc }))?;
        armed.push(Armed {
            plan_commit: pc,
            commitment: next["commitment"].as_str().unwrap_or_default().to_owned(),
            committed_at_ms: next["committed_at_ms"].as_u64().unwrap_or_default(),
            head: head.clone(),
            consumed_by: None,
        });
        state.write_armed(&armed)?;
    }
    Ok(report)
}

/// The fixed diagnostic the host sees for a report: per run the promise,
/// the decision and each consequence; never a step, value or threshold.
#[must_use]
pub fn diagnostic_lines(report: &JitReport) -> Vec<String> {
    let short = report.commit.get(..12).unwrap_or(&report.commit);
    let mut out = Vec::new();
    for r in &report.runs {
        out.push(format!("holden: {short}: {} \"{}\"", r.decision, r.promise));
        for d in &r.diagnostics {
            match &d.promise_text {
                Some(p) => out.push(format!("holden:   \"{p}\": {}", d.consequence)),
                None => out.push(format!("holden:   {}", d.consequence)),
            }
        }
        out.push(format!("holden:   trace: holden trace {}", r.run_id));
    }
    for why in &report.refused {
        out.push(format!("holden: {short}: {why}"));
    }
    out
}
