//! The typed bodies of ledger records.
//!
//! | Kind | Body | Written by |
//! |---|---|---|
//! | `control` | [`ControlRecord`]: one control run through the real evaluator | [`crate::controls`] |
//! | `verdict` | [`VerdictEntry`]: one verdict the MCP server accepted | holden-mcp |
//! | `weights` | [`WeightsRecord`]: lens draw weights recomputed from yield | [`crate::feedback`] |
//! | `replan` | [`ReplanItem`]: a re-plan trigger and the version it bumps to | [`crate::feedback`] |
//! | `selftest` | [`SelftestRecord`]: one reintroduced defect of holden's own code | `cargo xtask selftest` |

use std::collections::BTreeMap;

use holden_eval::binding::Probe;
use holden_eval::confine::Limitation;
use holden_eval::rules::CheckOutcome;
use holden_eval::{Decision, FailReason, VerdictRecord, notes_binding};
use holden_gen::GenPlan;
use serde::{Deserialize, Serialize};

use crate::stats::Rate;

/// Record kinds.
pub mod kind {
    /// A control run.
    pub const CONTROL: &str = "control";
    /// A verdict the server accepted.
    pub const VERDICT: &str = "verdict";
    /// Recomputed draw weights.
    pub const WEIGHTS: &str = "weights";
    /// A re-plan item.
    pub const REPLAN: &str = "replan";
    /// A self-test result.
    pub const SELFTEST: &str = "selftest";
}

/// What kind of control a run was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlKind {
    /// A known-good build (failing it is a false fail).
    KnownGood,
    /// A fixture mutant that breaks a promise.
    FixtureMutant,
    /// The plan's targeted broken variant: its `must_fail` check must fail.
    PlanTargeted,
    /// A product that does nothing for one promise (vacuity control).
    DoNothing,
    /// A re-introduced never-again defect (S-RUNNER review).
    NeverAgain,
}

/// What a known-broken control breaks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum Target {
    /// An approved promise, by its exact text.
    Promise {
        /// The promise text.
        text: String,
    },
    /// An integrity rule, caught when the verdict fails with this reason.
    Reason {
        /// The typed reason.
        reason: FailReason,
    },
    /// A limit: caught when a run is killed at a limit.
    Limit,
}

/// What a control is expected to be.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "expect", rename_all = "snake_case")]
pub enum Expect {
    /// Known good.
    Good,
    /// Known broken.
    Broken {
        /// What it breaks.
        target: Target,
    },
}

/// One check as seen in a control run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckSeen {
    /// The plan's check id.
    pub id: String,
    /// The ledger key (`<plan id>/<check id>`).
    pub key: String,
    /// The promise text the check quotes.
    pub promise_text: String,
    /// The lens concept that shaped its step, hex.
    pub lens: Option<String>,
    /// Its probe.
    pub probe: Option<Probe>,
    /// Its outcome in the verdict.
    pub outcome: CheckOutcome,
    /// Its consequence, when it did not hold.
    pub consequence: Option<String>,
}

/// One control run through the real evaluator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlRecord {
    /// `holden.control/1`.
    pub format: String,
    /// The holdout (`<promise id>:<cognition id>`).
    pub holdout: String,
    /// The holdout version the control ran under.
    pub version: u64,
    /// The plan id, hex.
    pub plan_id: String,
    /// The plan commit, hex.
    pub plan_commit: String,
    /// The plan's promise id, hex.
    pub promise: String,
    /// The plan's cognition id, hex.
    pub cognition: String,
    /// The control's name.
    pub control: String,
    /// The fixture variant it built.
    pub variant: String,
    /// Its kind.
    pub kind: ControlKind,
    /// Its expectation.
    pub expect: Expect,
    /// For the plan's targeted variant, the check that must fail.
    pub must_fail: Option<String>,
    /// The evaluated commit.
    pub evaluated_commit: String,
    /// SHA-256 of the signed verdict record it produced.
    pub verdict_sha256: String,
    /// The world the evaluation ran in.
    pub world_id: String,
    /// The rules version that decided it.
    pub rules_version: String,
    /// The decision.
    pub decision: Decision,
    /// The typed fail reasons.
    pub reasons: Vec<FailReason>,
    /// Every check's outcome.
    pub checks: Vec<CheckSeen>,
    /// The verdict's confinement limitations.
    pub limitations: Vec<Limitation>,
}

/// The holdout id of `plan`: `<promise id>:<cognition id>`.
#[must_use]
pub fn holdout_of(plan: &GenPlan) -> String {
    format!(
        "{}:{}",
        plan.provenance.promise.to_hex(),
        plan.provenance.cognition.to_hex()
    )
}

/// The lens concept that shaped each step's check, by check id.
#[must_use]
pub fn lens_by_check(plan: &GenPlan) -> BTreeMap<String, String> {
    plan.steps
        .iter()
        .filter_map(|s| s.lens.map(|l| (s.check.id.clone(), l.to_hex())))
        .collect()
}

/// The exact promise text bound to `probe` in the notes binding.
#[must_use]
pub fn promise_of(probe: Probe) -> Option<&'static str> {
    notes_binding()
        .promises
        .iter()
        .find(|(_, p)| *p == probe)
        .map(|(t, _)| *t)
}

impl ControlRecord {
    /// The control record of `record`, produced by running `control` under
    /// `plan` at holdout `version`. Every outcome is read from the signed
    /// verdict record.
    #[must_use]
    pub fn from_run(
        plan: &GenPlan,
        version: u64,
        control: &crate::controls::Control,
        record: &VerdictRecord,
    ) -> Self {
        let b = &record.body;
        let lenses = lens_by_check(plan);
        let checks = b
            .checks
            .iter()
            .zip(&b.decided.results)
            .map(|(c, r)| CheckSeen {
                id: c.id.clone(),
                key: crate::measure::check_key(&b.plan_id, &c.id),
                promise_text: c.promise_text.clone(),
                lens: lenses.get(&c.id).cloned(),
                probe: c.probe,
                outcome: r.outcome,
                consequence: r.consequence.clone(),
            })
            .collect();
        Self {
            format: "holden.control/1".to_owned(),
            holdout: holdout_of(plan),
            version,
            plan_id: b.plan_id.clone(),
            plan_commit: b.plan_commit.clone(),
            promise: plan.provenance.promise.to_hex(),
            cognition: plan.provenance.cognition.to_hex(),
            control: control.name.clone(),
            variant: control.variant.clone(),
            kind: control.kind,
            expect: control.expect.clone(),
            must_fail: control.must_fail.clone(),
            evaluated_commit: b.evaluated_commit.clone(),
            verdict_sha256: record.sha256(),
            world_id: b.world_id.clone(),
            rules_version: b.rules_version.clone(),
            decision: b.decided.decision,
            reasons: b.decided.reasons.clone(),
            checks,
            limitations: b.confinement.limitations.clone(),
        }
    }

    /// Whether this known-broken control's target is something the plan
    /// checks (a promise one of its checks quotes, or an integrity rule).
    /// Known-good controls are always in scope.
    #[must_use]
    pub fn in_scope(&self) -> bool {
        match &self.expect {
            Expect::Good => true,
            Expect::Broken {
                target: Target::Promise { text },
            } => self.checks.iter().any(|c| &c.promise_text == text),
            Expect::Broken { .. } => true,
        }
    }
}

/// One verdict the MCP server accepted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerdictEntry {
    /// `holden.verdict-entry/1`.
    pub format: String,
    /// The run id.
    pub run_id: String,
    /// The plan id, hex.
    pub plan_id: String,
    /// The plan commit, hex.
    pub plan_commit: String,
    /// The evaluated commit.
    pub evaluated_commit: String,
    /// SHA-256 of the signed verdict record.
    pub verdict_sha256: String,
    /// The decision.
    pub decision: Decision,
    /// The typed fail reasons.
    pub reasons: Vec<FailReason>,
    /// How many canaries the scan found.
    pub canary_findings: u64,
    /// The confinement limitations.
    pub limitations: Vec<Limitation>,
    /// The cognition whose checks this run measured, when it was generated.
    #[serde(default)]
    pub cognition: String,
    /// The observed per-check outcomes, including lens provenance.
    #[serde(default)]
    pub checks: Vec<CheckSeen>,
}

impl VerdictEntry {
    /// The entry of an accepted `record`, with generated-plan provenance
    /// when this server registered that kind of plan.
    #[must_use]
    pub fn of(run_id: &str, record: &VerdictRecord, plan: Option<&GenPlan>) -> Self {
        let b = &record.body;
        let lenses = plan.map_or_else(BTreeMap::new, lens_by_check);
        let checks = b
            .checks
            .iter()
            .zip(&b.decided.results)
            .map(|(c, r)| CheckSeen {
                id: c.id.clone(),
                key: crate::measure::check_key(&b.plan_id, &c.id),
                promise_text: c.promise_text.clone(),
                lens: lenses.get(&c.id).cloned(),
                probe: c.probe,
                outcome: r.outcome,
                consequence: r.consequence.clone(),
            })
            .collect();
        Self {
            format: "holden.verdict-entry/1".to_owned(),
            run_id: run_id.to_owned(),
            plan_id: b.plan_id.clone(),
            plan_commit: b.plan_commit.clone(),
            evaluated_commit: b.evaluated_commit.clone(),
            verdict_sha256: record.sha256(),
            decision: b.decided.decision,
            reasons: b.decided.reasons.clone(),
            canary_findings: b.canary.findings.len() as u64,
            limitations: b.confinement.limitations.clone(),
            cognition: plan.map_or_else(String::new, |p| p.provenance.cognition.to_hex()),
            checks,
        }
    }
}

/// Lens draw weights recomputed from ledger yield.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeightsRecord {
    /// `holden.lens-weights/1`.
    pub format: String,
    /// The corpus weight of each concept (the base).
    pub corpus: BTreeMap<String, u32>,
    /// The recomputed weight of each concept (never 0).
    pub weights: BTreeMap<String, u32>,
    /// The yield each weight was computed from (catches of broken
    /// controls by the checks each lens shaped).
    pub basis: BTreeMap<String, RateRecord>,
    /// The ledger tip the weights were computed at (`<seq>:<hash>`).
    pub at: Option<String>,
}

/// A rate as recorded (counts only; bounds are recomputed when read).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateRecord {
    /// Successes.
    pub k: u64,
    /// Trials.
    pub n: u64,
}

impl From<Rate> for RateRecord {
    fn from(r: Rate) -> Self {
        Self { k: r.k, n: r.n }
    }
}

/// A re-plan trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// Sealed material appeared in an evaluated tree: the plan leaked.
    CanaryLeak,
    /// The plan passed a product that does nothing for its promise.
    Vacuity,
    /// A control proved a verdict false (a known-good build failed, or a
    /// known-broken build passed).
    ProvenFalse,
}

/// A re-plan item: a trigger, its evidence, and the version bump.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplanItem {
    /// `holden.replan/1`.
    pub format: String,
    /// The holdout.
    pub holdout: String,
    /// The trigger.
    pub trigger: Trigger,
    /// The ledger record that fired it.
    pub evidence_seq: u64,
    /// What fired it (control name and verdict hash, or verdict hash).
    pub evidence: String,
    /// The version it was found at.
    pub from_version: u64,
    /// The version after the bump.
    pub to_version: u64,
    /// What must be re-planned, in words.
    pub item: String,
}

/// One reintroduced defect of holden's own code, run by
/// `cargo xtask selftest`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelftestRecord {
    /// `holden.selftest/1`.
    pub format: String,
    /// The defect's id.
    pub defect: String,
    /// The crate the mutant changes.
    pub crate_name: String,
    /// The file it changes.
    pub file: String,
    /// The never-again test that must catch it.
    pub test: String,
    /// The commit of the code under test.
    pub commit: String,
    /// Whether the test passed on the unmutated code (known good).
    pub baseline_passed: bool,
    /// How many tests the baseline run executed.
    pub baseline_ran: u64,
    /// How many tests the mutant run executed.
    pub mutant_ran: u64,
    /// Whether the test failed on the mutant (the defect was caught).
    pub caught: bool,
}
