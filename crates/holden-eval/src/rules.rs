//! The decision rules (rung 0): deterministic, from the planted values and
//! the boundary observations alone, so a replay reaches the same decision.
//!
//! Unsure never passes: a check whose promise has no probe, or whose
//! observations are missing, is unsure, and an unsure check fails the run.
//!
//! "Publish the why, seal the how": a diagnostic names the promise and the
//! consequence only. No diagnostic names a step, a planted value, a count or
//! a threshold.

use serde::{Deserialize, Serialize};

use crate::binding::{BoundCheck, Probe};
use crate::canary::CanaryReport;
use crate::confine::ConfinementReport;
use crate::drive::{Observation, Observed};
use crate::plant::Planted;
use crate::spawn::{BoundedBytes, Exit};

/// The rules version recorded in every verdict. Version 2 (slice-5) adds
/// typed fail reasons and the confinement-violation rule.
pub const RULES_VERSION: &str = "holden.rules.notes/2";

/// The signal a confinement backend that observes violations kills the
/// offending process with (`SIGKILL`, 9 on macOS and Linux).
pub const VIOLATION_SIGNAL: i32 = 9;

/// A typed reason a verdict failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailReason {
    /// The product could not be built.
    BuildFailed,
    /// Sealed test material was found in the evaluated tree.
    CanaryLeak,
    /// A product run attempted to act outside its confinement.
    ConfinementViolation,
    /// A check broke.
    CheckBroken,
    /// A check was refused (including a run killed at a limit).
    CheckRefused,
    /// A check could not be observed.
    CheckUnsure,
}

impl FailReason {
    /// Its stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BuildFailed => "build_failed",
            Self::CanaryLeak => "canary_leak",
            Self::ConfinementViolation => "confinement_violation",
            Self::CheckBroken => "check_broken",
            Self::CheckRefused => "check_refused",
            Self::CheckUnsure => "check_unsure",
        }
    }
}

/// The decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Every check held.
    Pass,
    /// A check broke, was refused or was unsure, or integrity failed.
    Fail,
}

/// One check's outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    /// The promise held.
    Held,
    /// The product answered, and the promise did not hold.
    Broken,
    /// The product refused or failed to answer.
    Refused,
    /// The promise could not be observed.
    Unsure,
}

/// One check's result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckResult {
    /// The check id.
    pub id: String,
    /// The outcome.
    pub outcome: CheckOutcome,
    /// The consequence, when it did not hold.
    pub consequence: Option<String>,
}

/// What the builder is told about a failure: the promise and the
/// consequence, never the how.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    /// The promise id, hex; `None` for an integrity failure.
    pub promise: Option<String>,
    /// The promise, in the owner's words.
    pub promise_text: Option<String>,
    /// What went wrong for the person the promise is for.
    pub consequence: String,
}

/// The decision with its reasons.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decided {
    /// Per-check results, in plan order.
    pub results: Vec<CheckResult>,
    /// Builder-visible diagnostics.
    pub diagnostics: Vec<Diagnostic>,
    /// Why it failed, typed (empty for a pass), sorted.
    pub reasons: Vec<FailReason>,
    /// The decision.
    pub decision: Decision,
}

/// Consequences (fixed strings: no values, counts or steps).
pub mod consequence {
    /// The product could not be built.
    pub const BUILD: &str = "the product could not be built from the evaluated commit";
    /// Canary leak.
    pub const CANARY: &str = "sealed test material was found in the evaluated tree";
    /// Confinement violation.
    pub const CONFINEMENT: &str = "the product tried to act outside its confinement";
    /// Not observable.
    pub const UNSURE: &str =
        "the evaluation could not observe this promise at the product's boundary";
    /// A limit was hit.
    pub const LIMIT: &str = "the product did not finish within its limits";
    /// Import refused.
    pub const IMPORT_REFUSED: &str = "the product refused to keep notes it was given";
    /// Interrupted save: unreadable.
    pub const SAVE_UNREADABLE: &str =
        "after an interrupted save, the notes saved before it could not be read back";
    /// Interrupted save: lost.
    pub const SAVE_LOST: &str = "after an interrupted save, a note saved before it was lost";
    /// Interrupted save: changed.
    pub const SAVE_CHANGED: &str =
        "after an interrupted save, a note saved before it came back changed";
    /// Listing refused.
    pub const LIST_REFUSED: &str = "the public listing refused to answer";
    /// Listing incomplete.
    pub const LIST_INCOMPLETE: &str = "the public listing left out a saved note";
    /// Export lied.
    pub const EXPORT_CLAIMED: &str = "a failed export reported success";
    /// Export unexplained.
    pub const EXPORT_UNEXPLAINED: &str =
        "a failed export did not say why it failed or how to retry";
    /// Retry failed.
    pub const RETRY_FAILED: &str = "retrying the export as advised did not succeed";
    /// Retry lost data.
    pub const RETRY_LOST: &str = "retrying the export as advised left out a saved note";
    /// Deleted without confirmation.
    pub const DELETE_UNASKED: &str = "a note was removed without confirmation";
    /// Confirmed delete refused.
    pub const DELETE_REFUSED: &str = "a confirmed delete was refused";
    /// Confirmed delete ignored.
    pub const DELETE_IGNORED: &str = "a confirmed delete did not remove the note";
    /// A real workspace test or CLI entry point failed.
    pub const WORKSPACE_FAILED: &str =
        "the product's public tests or command entry point did not support the approved behavior";
}

/// Inputs to the rules besides the observations.
#[derive(Clone, Copy, Debug)]
pub struct Integrity<'a> {
    /// The canary scan of the evaluated tree.
    pub canary: &'a CanaryReport,
    /// The confinement summary (whether product runs observe violations).
    pub confinement: &'a ConfinementReport,
}

/// Whether a product run was killed for a confinement violation: under a
/// confinement that kills on violation, a product run (not the build) that
/// ended by [`VIOLATION_SIGNAL`]. holden's own kills at a limit are recorded
/// as `killed`, never as a signal.
#[must_use]
pub fn confinement_violated(obs: &[Observation], confinement: &ConfinementReport) -> bool {
    confinement.product_violations_kill
        && obs.iter().any(|o| {
            o.probe.is_some()
                && matches!(
                    o.observed,
                    Observed::Process {
                        exit: Exit::Signal {
                            signal: VIOLATION_SIGNAL
                        },
                        ..
                    }
                )
        })
}

struct Proc<'a> {
    exit: Exit,
    stdout: &'a BoundedBytes,
    stderr: &'a BoundedBytes,
}

fn proc_of<'a>(obs: &'a [Observation], probe: Option<Probe>, label: &str) -> Option<Proc<'a>> {
    obs.iter()
        .find(|o| o.probe == probe && o.label == label)
        .and_then(|o| match &o.observed {
            Observed::Process {
                exit,
                stdout,
                stderr,
                ..
            } => Some(Proc {
                exit: *exit,
                stdout,
                stderr,
            }),
            Observed::File { .. } => None,
        })
}

fn file_of(obs: &[Observation], probe: Probe, label: &str) -> Option<Option<Vec<u8>>> {
    obs.iter()
        .find(|o| o.probe == Some(probe) && o.label == label)
        .and_then(|o| match &o.observed {
            Observed::File {
                present, content, ..
            } => Some(if *present {
                content.as_ref().and_then(BoundedBytes::bytes)
            } else {
                None
            }),
            Observed::Process { .. } => None,
        })
}

type Verdict = Result<(), (CheckOutcome, &'static str)>;

fn unsure<T>(v: Option<T>) -> Result<T, (CheckOutcome, &'static str)> {
    v.ok_or((CheckOutcome::Unsure, consequence::UNSURE))
}

/// A process that must have run to completion; returns it.
fn ran<'a>(
    obs: &'a [Observation],
    p: Probe,
    label: &str,
) -> Result<Proc<'a>, (CheckOutcome, &'static str)> {
    let pr = unsure(proc_of(obs, Some(p), label))?;
    if matches!(pr.exit, Exit::Killed { .. }) {
        return Err((CheckOutcome::Refused, consequence::LIMIT));
    }
    Ok(pr)
}

fn lines(b: &BoundedBytes) -> Vec<String> {
    b.text().lines().map(str::to_owned).collect()
}

fn imported(obs: &[Observation], p: Probe) -> Verdict {
    if ran(obs, p, "import")?.exit.success() {
        Ok(())
    } else {
        Err((CheckOutcome::Refused, consequence::IMPORT_REFUSED))
    }
}

fn interrupted_save(obs: &[Observation], planted: &Planted) -> Verdict {
    let p = Probe::InterruptedSave;
    ran(obs, p, "import")?;
    let list = ran(obs, p, "list")?;
    if !list.exit.success() {
        return Err((CheckOutcome::Refused, consequence::SAVE_UNREADABLE));
    }
    let names = lines(list.stdout);
    for (k, n) in planted
        .notes
        .iter()
        .take(planted.interrupt_after)
        .enumerate()
    {
        if !names.contains(&n.name) {
            return Err((CheckOutcome::Broken, consequence::SAVE_LOST));
        }
        let get = ran(obs, p, &format!("get-{k}"))?;
        if !get.exit.success() {
            return Err((CheckOutcome::Refused, consequence::SAVE_UNREADABLE));
        }
        if get.stdout.bytes().as_deref() != Some(n.body.as_bytes()) {
            return Err((CheckOutcome::Broken, consequence::SAVE_CHANGED));
        }
    }
    Ok(())
}

fn list_every(obs: &[Observation], planted: &Planted) -> Verdict {
    let p = Probe::ListEvery;
    imported(obs, p)?;
    let list = ran(obs, p, "list")?;
    if !list.exit.success() {
        return Err((CheckOutcome::Refused, consequence::LIST_REFUSED));
    }
    let names = lines(list.stdout);
    if planted.notes.iter().any(|n| !names.contains(&n.name)) {
        return Err((CheckOutcome::Broken, consequence::LIST_INCOMPLETE));
    }
    Ok(())
}

fn export_failure_explained(obs: &[Observation], planted: &Planted) -> Verdict {
    let p = Probe::ExportFailureExplained;
    imported(obs, p)?;
    let missing = ran(obs, p, "export-missing")?;
    if missing.exit.success() {
        return Err((CheckOutcome::Broken, consequence::EXPORT_CLAIMED));
    }
    let said = missing.stderr.text().to_ascii_lowercase();
    if !(said.contains("retry") && (said.contains("why") || said.contains("because"))) {
        return Err((CheckOutcome::Broken, consequence::EXPORT_UNEXPLAINED));
    }
    if !ran(obs, p, "export-retry")?.exit.success() {
        return Err((CheckOutcome::Refused, consequence::RETRY_FAILED));
    }
    let Some(file) = unsure(file_of(obs, p, "export-file"))? else {
        return Err((CheckOutcome::Refused, consequence::RETRY_FAILED));
    };
    let text = String::from_utf8_lossy(&file);
    let exported: Vec<&str> = text.lines().collect();
    for n in &planted.notes {
        let line = format!("{}\t{}", n.name, n.body);
        if !exported.contains(&line.as_str()) {
            return Err((CheckOutcome::Broken, consequence::RETRY_LOST));
        }
    }
    Ok(())
}

fn delete_asks_first(obs: &[Observation], planted: &Planted) -> Verdict {
    let p = Probe::DeleteAsksFirst;
    imported(obs, p)?;
    let name = &unsure(planted.notes.get(planted.delete_index))?.name;
    let unasked = ran(obs, p, "delete")?;
    let after = ran(obs, p, "list-after-delete")?;
    if unasked.exit.success() || !lines(after.stdout).contains(name) {
        return Err((CheckOutcome::Broken, consequence::DELETE_UNASKED));
    }
    if !ran(obs, p, "delete-confirmed")?.exit.success() {
        return Err((CheckOutcome::Refused, consequence::DELETE_REFUSED));
    }
    let after = ran(obs, p, "list-after-confirmed")?;
    if !after.exit.success() || lines(after.stdout).contains(name) {
        return Err((CheckOutcome::Broken, consequence::DELETE_IGNORED));
    }
    Ok(())
}

fn cargo_workspace(obs: &[Observation]) -> Verdict {
    let mut tests = obs
        .iter()
        .filter(|o| {
            o.probe == Some(Probe::CargoWorkspace) && o.label.starts_with("workspace-tests")
        })
        .peekable();
    if tests.peek().is_none() {
        return Err((CheckOutcome::Unsure, consequence::UNSURE));
    }
    for observation in tests {
        let test = ran(obs, Probe::CargoWorkspace, &observation.label)?;
        if !test.exit.success() {
            return Err((CheckOutcome::Broken, consequence::WORKSPACE_FAILED));
        }
    }
    let mut entry_points = obs
        .iter()
        .filter(|o| o.probe == Some(Probe::CargoWorkspace) && o.label.starts_with("cli-"))
        .peekable();
    if entry_points.peek().is_none() {
        return Err((CheckOutcome::Unsure, consequence::UNSURE));
    }
    for observation in entry_points {
        let process = unsure(proc_of(
            obs,
            Some(Probe::CargoWorkspace),
            &observation.label,
        ))?;
        if matches!(process.exit, Exit::Killed { .. }) {
            return Err((CheckOutcome::Refused, consequence::LIMIT));
        }
        if !matches!(process.exit, Exit::Code { .. }) {
            return Err((CheckOutcome::Broken, consequence::WORKSPACE_FAILED));
        }
    }
    Ok(())
}

fn probe_verdict(p: Probe, obs: &[Observation], planted: &Planted) -> Verdict {
    match p {
        Probe::InterruptedSave => interrupted_save(obs, planted),
        Probe::ListEvery => list_every(obs, planted),
        Probe::ExportFailureExplained => export_failure_explained(obs, planted),
        Probe::DeleteAsksFirst => delete_asks_first(obs, planted),
        Probe::CargoWorkspace => cargo_workspace(obs),
    }
}

/// Whether the build observation shows a successful build.
fn built(obs: &[Observation]) -> bool {
    let mut builds = obs
        .iter()
        .filter(|o| o.probe.is_none() && o.label.starts_with("build"))
        .peekable();
    builds.peek().is_some()
        && builds.all(|o| proc_of(obs, None, &o.label).is_some_and(|b| b.exit.success()))
}

/// Decide from the bound checks, the planted values, the observations and
/// the integrity inputs.
#[must_use]
pub fn decide(
    checks: &[BoundCheck],
    planted: &Planted,
    obs: &[Observation],
    integrity: Integrity<'_>,
) -> Decided {
    let mut diagnostics = Vec::new();
    let build_ok = built(obs);
    if !build_ok {
        diagnostics.push(Diagnostic {
            promise: None,
            promise_text: None,
            consequence: consequence::BUILD.to_owned(),
        });
    }
    let canary_ok = integrity.canary.findings.is_empty();
    if !canary_ok {
        diagnostics.push(Diagnostic {
            promise: None,
            promise_text: None,
            consequence: consequence::CANARY.to_owned(),
        });
    }
    let confined_ok = !confinement_violated(obs, integrity.confinement);
    if !confined_ok {
        diagnostics.push(Diagnostic {
            promise: None,
            promise_text: None,
            consequence: consequence::CONFINEMENT.to_owned(),
        });
    }
    let results: Vec<CheckResult> = checks
        .iter()
        .map(|c| {
            let v = match c.probe {
                None => Err((CheckOutcome::Unsure, consequence::UNSURE)),
                Some(_) if !build_ok => Err((CheckOutcome::Unsure, consequence::UNSURE)),
                Some(p) => probe_verdict(p, obs, planted),
            };
            match v {
                Ok(()) => CheckResult {
                    id: c.id.clone(),
                    outcome: CheckOutcome::Held,
                    consequence: None,
                },
                Err((outcome, why)) => {
                    diagnostics.push(Diagnostic {
                        promise: Some(c.promise.clone()),
                        promise_text: Some(c.promise_text.clone()),
                        consequence: why.to_owned(),
                    });
                    CheckResult {
                        id: c.id.clone(),
                        outcome,
                        consequence: Some(why.to_owned()),
                    }
                }
            }
        })
        .collect();
    diagnostics.sort();
    diagnostics.dedup();
    let mut reasons = Vec::new();
    if !build_ok {
        reasons.push(FailReason::BuildFailed);
    }
    if !canary_ok {
        reasons.push(FailReason::CanaryLeak);
    }
    if !confined_ok {
        reasons.push(FailReason::ConfinementViolation);
    }
    if results.is_empty() {
        reasons.push(FailReason::CheckUnsure);
    }
    for r in &results {
        match r.outcome {
            CheckOutcome::Held => {}
            CheckOutcome::Broken => reasons.push(FailReason::CheckBroken),
            CheckOutcome::Refused => reasons.push(FailReason::CheckRefused),
            CheckOutcome::Unsure => reasons.push(FailReason::CheckUnsure),
        }
    }
    reasons.sort();
    reasons.dedup();
    let decision = if reasons.is_empty() {
        Decision::Pass
    } else {
        Decision::Fail
    };
    Decided {
        results,
        diagnostics,
        reasons,
        decision,
    }
}
