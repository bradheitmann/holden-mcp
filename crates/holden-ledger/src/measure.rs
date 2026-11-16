//! Catch and false-fail rates from control records (slice-5 P2).
//!
//! Every count comes from an outcome the real evaluator recorded in a
//! signed verdict; nothing is counted from what a control was expected to
//! do. For each control record:
//!
//! - known good: every check is a false-fail trial (it failed when its
//!   outcome is not `held`), and so are the plan's decision and each
//!   integrity rule;
//! - known broken, targeting a promise the plan checks: every check quoting
//!   that promise is a catch trial (caught when its outcome is not `held`),
//!   and so is the plan's decision (caught when it fails);
//! - known broken, targeting an integrity rule or a limit: that rule is a
//!   catch trial (caught when the verdict fails with that reason, or a run
//!   was killed at a limit), and so is the plan's decision;
//! - known broken, targeting a promise the plan does not check: out of
//!   scope, not counted.
//!
//! Check rates roll up per lens concept (the checks whose step a lens
//! shaped) and per cognition (every check of a plan in that cognition).

use std::collections::{BTreeMap, BTreeSet};

use holden_eval::rules::{CheckOutcome, consequence};
use holden_eval::{Decision, FailReason};
use serde::{Deserialize, Serialize};

use crate::records::{ControlRecord, Expect, SelftestRecord, Target, VerdictEntry, kind};
use crate::stats::CheckStats;
use crate::store::{Entry, LedgerError};

/// The ledger key of a plan check.
#[must_use]
pub fn check_key(plan_id: &str, check_id: &str) -> String {
    format!("{plan_id}/{check_id}")
}

/// The ledger key of an integrity rule, for the reasons that are rules (not
/// check outcomes).
#[must_use]
pub fn integrity_key(reason: FailReason) -> Option<String> {
    match reason {
        FailReason::BuildFailed | FailReason::CanaryLeak | FailReason::ConfinementViolation => {
            Some(format!("integrity/{}", reason.as_str()))
        }
        FailReason::CheckBroken | FailReason::CheckRefused | FailReason::CheckUnsure => None,
    }
}

/// The ledger key of the limits rule.
pub const LIMIT_KEY: &str = "integrity/limit";

/// The integrity rules every control is a trial of.
const RULES: [FailReason; 3] = [
    FailReason::BuildFailed,
    FailReason::CanaryLeak,
    FailReason::ConfinementViolation,
];

/// Measured rates.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Measurements {
    /// Per check (`<plan id>/<check id>`).
    pub checks: BTreeMap<String, CheckStats>,
    /// Per lens concept (hex id).
    pub lenses: BTreeMap<String, CheckStats>,
    /// Per cognition (hex id).
    pub cognitions: BTreeMap<String, CheckStats>,
    /// Per plan (its decision), by plan id.
    pub plans: BTreeMap<String, CheckStats>,
    /// Per integrity rule (`integrity/<reason>`, [`LIMIT_KEY`]).
    pub integrity: BTreeMap<String, CheckStats>,
    /// Per self-test defect.
    pub selftest: BTreeMap<String, CheckStats>,
    /// How many control records were read.
    pub controls: u64,
}

fn killed_at_limit(r: &ControlRecord) -> bool {
    r.checks
        .iter()
        .any(|c| c.consequence.as_deref() == Some(consequence::LIMIT))
}

impl Measurements {
    /// Tally `records`.
    #[must_use]
    pub fn from_records(records: &[ControlRecord]) -> Self {
        let mut m = Self::default();
        for r in records {
            m.add(r);
        }
        m
    }

    /// Tally the control and self-test records of `entries`.
    ///
    /// # Errors
    /// [`LedgerError::Body`] when a record does not decode.
    pub fn from_entries(entries: &[Entry]) -> Result<Self, LedgerError> {
        let mut m = Self::default();
        let control_verdicts: BTreeSet<String> = entries
            .iter()
            .filter(|e| e.kind == kind::CONTROL)
            .map(|e| Ok(e.decode::<ControlRecord>()?.verdict_sha256))
            .collect::<Result<_, LedgerError>>()?;
        for e in entries {
            match e.kind.as_str() {
                kind::CONTROL => m.add(&e.decode::<ControlRecord>()?),
                kind::VERDICT => {
                    let verdict = e.decode::<VerdictEntry>()?;
                    // A control's verdict is already counted through its
                    // richer control record; never count that trial twice.
                    if !control_verdicts.contains(&verdict.verdict_sha256) {
                        m.add_verdict(&verdict);
                    }
                }
                kind::SELFTEST => m.add_selftest(&e.decode::<SelftestRecord>()?),
                _ => {}
            }
        }
        Ok(m)
    }

    /// Count a real accepted run as observed effectiveness evidence. Every
    /// check contributes one catch trial; a non-held outcome is a catch.
    /// Lens and cognition rollups use the generated plan provenance captured
    /// in the entry, so signed W2 runs feed exactly the same draw feedback.
    fn add_verdict(&mut self, v: &VerdictEntry) {
        // Pre-slice-14 entries have no per-check observations. Their
        // companion control records remain the sole measurement source.
        if v.checks.is_empty() {
            return;
        }
        let failed = v.decision == Decision::Fail;
        for c in &v.checks {
            let caught = c.outcome != CheckOutcome::Held;
            self.checks
                .entry(c.key.clone())
                .or_default()
                .catch
                .add(caught);
            if let Some(lens) = &c.lens {
                self.lenses
                    .entry(lens.clone())
                    .or_default()
                    .catch
                    .add(caught);
            }
            if !v.cognition.is_empty() {
                self.cognitions
                    .entry(v.cognition.clone())
                    .or_default()
                    .catch
                    .add(caught);
            }
        }
        self.plans
            .entry(v.plan_id.clone())
            .or_default()
            .catch
            .add(failed);
    }

    fn add_selftest(&mut self, s: &SelftestRecord) {
        let st = self.selftest.entry(s.defect.clone()).or_default();
        st.catch.add(s.caught);
        st.false_fail.add(!s.baseline_passed);
    }

    /// Tally one control record.
    pub fn add(&mut self, r: &ControlRecord) {
        self.controls += 1;
        let failed = r.decision == Decision::Fail;
        match &r.expect {
            Expect::Good => {
                for c in &r.checks {
                    let bad = c.outcome != CheckOutcome::Held;
                    self.checks
                        .entry(c.key.clone())
                        .or_default()
                        .false_fail
                        .add(bad);
                    if let Some(l) = &c.lens {
                        self.lenses
                            .entry(l.clone())
                            .or_default()
                            .false_fail
                            .add(bad);
                    }
                    self.cognitions
                        .entry(r.cognition.clone())
                        .or_default()
                        .false_fail
                        .add(bad);
                }
                self.plans
                    .entry(r.plan_id.clone())
                    .or_default()
                    .false_fail
                    .add(failed);
                for reason in RULES {
                    if let Some(k) = integrity_key(reason) {
                        self.integrity
                            .entry(k)
                            .or_default()
                            .false_fail
                            .add(r.reasons.contains(&reason));
                    }
                }
                self.integrity
                    .entry(LIMIT_KEY.to_owned())
                    .or_default()
                    .false_fail
                    .add(killed_at_limit(r));
            }
            Expect::Broken { target } => {
                if !r.in_scope() {
                    return;
                }
                match target {
                    Target::Promise { text } => {
                        for c in r.checks.iter().filter(|c| &c.promise_text == text) {
                            let caught = c.outcome != CheckOutcome::Held;
                            self.checks
                                .entry(c.key.clone())
                                .or_default()
                                .catch
                                .add(caught);
                            if let Some(l) = &c.lens {
                                self.lenses.entry(l.clone()).or_default().catch.add(caught);
                            }
                            self.cognitions
                                .entry(r.cognition.clone())
                                .or_default()
                                .catch
                                .add(caught);
                        }
                    }
                    Target::Reason { reason } => {
                        if let Some(k) = integrity_key(*reason) {
                            self.integrity
                                .entry(k)
                                .or_default()
                                .catch
                                .add(r.reasons.contains(reason));
                        }
                    }
                    Target::Limit => {
                        self.integrity
                            .entry(LIMIT_KEY.to_owned())
                            .or_default()
                            .catch
                            .add(killed_at_limit(r));
                    }
                }
                self.plans
                    .entry(r.plan_id.clone())
                    .or_default()
                    .catch
                    .add(failed);
            }
        }
    }
}
