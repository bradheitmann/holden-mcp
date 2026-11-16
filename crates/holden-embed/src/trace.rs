//! `holden trace <run>` (slice-7 P3): the whole chain of one run, each link
//! re-verified from the stored inputs, never taken on trust:
//!
//! promise → approved cognition → root → lens → plan → params → verdict
//!
//! - promise: verbatim in the planning document at the evaluated commit;
//! - cognition: its content hash, and the owner's approval verified against
//!   the owner's published key;
//! - root: the doctrine roots that made the cognition, as the plan records;
//! - lens: the plan's lens concepts, and its keyed draw replayed under the
//!   agent's draw key;
//! - plan: the sealed plan opens to this plan id and commit, and rests on
//!   this promise and cognition;
//! - nonce: the commitment the run consumed was made before the evaluated
//!   commit existed;
//! - params: the audit reveal's VRF proof verifies, and equals what the run
//!   recorded;
//! - verdict: the signed record replays to its decision under the
//!   evaluator's key, and the signed ledger holds it and verifies.

use holden_cognition::{HostableCognition, OwnerTrust, PlanningDocument, detect};
use holden_eval::{ServerReveal, parse_record, replay};
use holden_gen::{DrawCorpus, GenPlan, unseal_plan};
use holden_mcp::SealedPlanFile;
use holden_seal::{DrawKey, EvaluatorKey, PlanCommit, SealKey, plan_commit};
use serde_json::Value;

use crate::jit::{TraceFile, TraceLedger, TraceVerdict};
use crate::state::State;
use crate::util::{HostGit, read_json};
use crate::{Error, refused};

fn plan_of(state: &State, plan_commit_hex: &str) -> Result<(GenPlan, PlanCommit), Error> {
    let seal = SealKey::new(state.key("seal")?);
    for f in state.sealed_plan_files()? {
        let wire: SealedPlanFile = read_json(&f)?;
        let sealed = wire.to_sealed().map_err(refused)?;
        let commit = plan_commit(&sealed);
        if hex::encode(commit.0) == plan_commit_hex {
            let plan = unseal_plan(&seal, &sealed, &commit).map_err(|e| refused(e.to_string()))?;
            return Ok((plan, commit));
        }
    }
    Err(refused("plan: no sealed plan has the run's plan commit"))
}

fn link<T, E: std::fmt::Display>(what: &str, r: Result<T, E>) -> Result<T, Error> {
    r.map_err(|e| refused(format!("{what}: {e}")))
}

/// Reconstruct the trace projection for a run composed server-side by W2.
/// The signed record and closed audit are authoritative; the public promise,
/// approval and pre-armed commitment are read from the same host and agent
/// state that JIT traces use. No product or transport is faked.
fn server_trace(
    state: &State,
    run_id: &str,
    record: &holden_eval::VerdictRecord,
    audit: &Value,
) -> Result<TraceFile, Error> {
    let agent = state.agent_config()?;
    let host_config = crate::host::HostConfig::read(&agent.host)?;
    let (plan, _) = plan_of(state, &record.body.plan_commit)?;
    let client = holden_cognition::adapters::read_promises(&agent.host)
        .map_err(|e| refused(e.to_string()))?;
    let trust = link("owner key", OwnerTrust::read_published(&agent.owner_store))?;
    let (promise, cognition, approval) =
        crate::jit::admitted_for(&agent.host, &client, &trust, &plan)?;
    let commitment = audit["commitment"].as_str().unwrap_or_default();
    let mut armed = state
        .armed()?
        .into_iter()
        .find(|a| a.plan_commit == record.body.plan_commit && a.commitment == commitment)
        .ok_or_else(|| refused("nonce: the server run's armed commitment is unavailable"))?;
    armed.consumed_by = Some(run_id.to_owned());
    let sha = record.sha256();
    let ledger =
        holden_ledger::Ledger::open(&state.ledger_db()).map_err(|e| refused(e.to_string()))?;
    let ledger_entry = ledger
        .entries()
        .map_err(|e| refused(e.to_string()))?
        .into_iter()
        .find(|e| {
            e.kind == holden_ledger::records::kind::VERDICT
                && e.decode::<holden_ledger::records::VerdictEntry>()
                    .is_ok_and(|v| v.run_id == run_id && v.verdict_sha256 == sha)
        })
        .ok_or_else(|| refused("ledger: the server run's verdict is absent"))?;
    let decision = match record.body.decided.decision {
        holden_eval::Decision::Pass => "pass",
        holden_eval::Decision::Fail => "fail",
    };
    Ok(TraceFile {
        format: "holden.trace/1".to_owned(),
        run_id: run_id.to_owned(),
        host: host_config.host,
        evaluated_commit: record.body.evaluated_commit.clone(),
        baseline_commit: record.body.baseline_commit.clone(),
        promise,
        cognition,
        approval,
        plan_id: record.body.plan_id.clone(),
        plan_commit: record.body.plan_commit.clone(),
        armed,
        params: record.body.reveal.params.clone(),
        verdict: TraceVerdict {
            sha256: sha,
            decision: decision.to_owned(),
            reasons: record
                .body
                .decided
                .reasons
                .iter()
                .map(|r| r.as_str().to_owned())
                .collect(),
            diagnostics: record.body.decided.diagnostics.clone(),
            server: Value::Null,
            signer: record.evaluator_public_key.clone(),
        },
        ledger: TraceLedger {
            seq: ledger_entry.seq,
            hash: ledger_entry.hash,
        },
    })
}

/// Verify and describe the chain of run `run_id` in `state`. Returns the
/// printable lines, one per link.
///
/// # Errors
/// [`Error::Refused`] naming the first link that does not hold.
#[allow(clippy::too_many_lines)]
pub fn trace(state: &State, run_id: &str) -> Result<Vec<String>, Error> {
    if run_id.len() != 64 || !run_id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(refused("a run id is 64 hex characters"));
    }
    let dir = state.run_dir(run_id);
    let record_text = std::fs::read_to_string(dir.join("record.json"))
        .map_err(|e| refused(format!("record: {e}")))?;
    let record = link("verdict", parse_record(&record_text))?;
    let audit: Value = read_json(&dir.join("audit.json"))?;
    let trace_path = dir.join("trace.json");
    let t: TraceFile = if trace_path.is_file() {
        read_json(&trace_path)?
    } else {
        server_trace(state, run_id, &record, &audit)?
    };
    let agent = state.agent_config()?;
    let git = HostGit::new(&agent.host, &state.git_home());
    let commit = &t.evaluated_commit;
    let mut lines = vec![format!(
        "run       {run_id} on host {} at commit {commit}",
        t.host
    )];

    // promise: verbatim at the evaluated commit, in the source document
    // whose whole-file hash the promise records (not an arbitrary match in
    // another selected planning file).
    let sources = link("promise", detect(&agent.host))?.documents;
    let mut source = None;
    for relative in sources {
        let name = relative.display().to_string();
        let Ok(bytes) = git.raw(&["cat-file", "blob", &format!("{commit}:{name}")]) else {
            continue;
        };
        let doc = PlanningDocument { name, bytes };
        if doc.sha256() == t.promise.source {
            link("promise", t.promise.verify_against(&doc))?;
            source = Some(doc);
            break;
        }
    }
    let doc = source.ok_or_else(|| refused("promise: its source is not at this commit"))?;
    let source_name = doc.name.clone();
    link("promise", t.promise.verify_against(&doc))?;
    lines.push(format!(
        "promise   {} \"{}\" ({} bytes {}..{}, sha256 {}): verbatim at {commit}",
        t.promise.id.to_hex(),
        t.promise.text,
        source_name,
        t.promise.start,
        t.promise.end,
        t.promise.source.to_hex()
    ));

    // approved cognition.
    let trust = link("owner key", OwnerTrust::read_published(&agent.owner_store))?;
    let hosted = link(
        "cognition",
        HostableCognition::admit(
            &doc,
            &t.promise,
            &t.cognition,
            Some(&t.approval),
            trust.public(),
        ),
    )?;
    let c = hosted.cognition();
    lines.push(format!(
        "cognition {} ({}: {}): approved by owner {}, signature verified",
        c.id.to_hex(),
        c.constituency.as_str(),
        c.role,
        hex::encode(trust.public().0)
    ));

    // plan (opened from the sealed side), root and lens.
    let (plan, plan_commit) = plan_of(state, &t.plan_commit)?;
    let p = &plan.provenance;
    if plan.computed_id() != plan.id
        || plan.id.to_hex() != t.plan_id
        || p.promise != t.promise.id
        || p.cognition != c.id
        || p.roots != c.roots
    {
        return Err(refused(
            "plan: it does not rest on this promise, cognition and roots",
        ));
    }
    lines.push(format!(
        "root      {}",
        p.roots
            .iter()
            .map(|r| r.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    let corpus = link("lens", DrawCorpus::load(&state.lens_dir()))?;
    link(
        "lens",
        holden_gen::replay(&DrawKey::new(state.key("draw")?), &corpus, &p.draw),
    )?;
    lines.push(format!(
        "lens      {}: the keyed draw replays",
        p.lens_concepts
            .iter()
            .map(|l| l.to_hex())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    lines.push(format!(
        "plan      {} commit {}: opens from the sealed side",
        t.plan_id, t.plan_commit
    ));

    // nonce committed before the commit.
    let a = &t.armed;
    let before = a.consumed_by.as_deref() == Some(run_id)
        && audit["commitment"].as_str() == Some(a.commitment.as_str())
        && a.head != *commit
        && (a.head == "none" || git.succeeds(&["merge-base", "--is-ancestor", &a.head, commit]));
    if !before {
        return Err(refused(
            "nonce: the run's commitment was not made before its commit",
        ));
    }
    lines.push(format!(
        "nonce     commitment {} committed at {} ms while HEAD was {}",
        a.commitment, a.committed_at_ms, a.head
    ));

    // params.
    let epoch = audit["epoch_public_key"].as_str().unwrap_or_default();
    let reveal = link(
        "params",
        ServerReveal::from_reveal_body(
            &crate::jit::reveal_part(&audit),
            epoch,
            &plan_commit,
            commit,
        ),
    )?;
    if *reveal.wire() != record.body.reveal || reveal.wire().params != t.params {
        return Err(refused("params: the audit reveal is not what the run used"));
    }
    let w = reveal.wire();
    lines.push(format!(
        "params    seed {} lens_slot {} step_order_salt {} bytes {}: VRF proof verified under epoch key {}",
        w.params.seed, w.params.lens_slot, w.params.step_order_salt, w.params.bytes, w.epoch_public_key
    ));

    // verdict and ledger.
    let signer = EvaluatorKey::from_material(&state.key("evaluator")?).public();
    let decided = link("verdict", replay(&record, Some(&signer)))?;
    if record.sha256() != t.verdict.sha256 || decided.decision != record.body.decided.decision {
        return Err(refused("verdict: the record is not the run's"));
    }
    let ledger = link("ledger", holden_ledger::Ledger::open(&state.ledger_db()))?;
    let ledger_signer = EvaluatorKey::from_material(&state.key("ledger")?).public();
    let report = link("ledger", ledger.verify(Some(&ledger_signer), None))?;
    let entries = link("ledger", ledger.entries())?;
    let held = entries.iter().any(|e| {
        e.seq == t.ledger.seq
            && e.hash == t.ledger.hash
            && e.decode::<holden_ledger::records::VerdictEntry>()
                .is_ok_and(|v| v.verdict_sha256 == t.verdict.sha256 && v.run_id == run_id)
    });
    if !report.holds() || !held {
        return Err(refused(
            "ledger: the verdict is not held by a ledger that verifies",
        ));
    }
    lines.push(format!(
        "verdict   {} {:?} record {} signed by {}: replays",
        t.verdict.decision,
        t.verdict.reasons,
        t.verdict.sha256,
        hex::encode(signer.0)
    ));
    for d in &t.verdict.diagnostics {
        lines.push(format!(
            "          diagnostic: {}{}",
            d.promise_text
                .as_ref()
                .map_or_else(String::new, |p| format!("\"{p}\": ")),
            d.consequence
        ));
    }
    lines.push(format!(
        "ledger    seq {} hash {}: the chain of {} record(s) verifies",
        t.ledger.seq, t.ledger.hash, report.records
    ));
    Ok(lines)
}
