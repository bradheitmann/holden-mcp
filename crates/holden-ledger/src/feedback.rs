//! Feedback from the ledger into future behaviour (slice-5 P4).
//!
//! (a) Lens draw weights. Each lens concept's yield is the share of
//! known-broken control trials its checks caught, smoothed so that an
//! unmeasured lens sits at one half: `(caught + 1) / (trials + 2)`. Its
//! weight is
//!
//! ```text
//! weight = max(1, round(4 × corpus weight × yield))
//! ```
//!
//! so an unmeasured lens keeps twice its corpus weight, a lens whose checks
//! catch everything approaches four times it, and a lens whose checks miss is
//! demoted but never below 1: it is never made invisible. The next keyed
//! draw picks with these weights ([`holden_gen::DrawCorpus::with_weights`]);
//! the draw key still decides every pick.
//!
//! (b) Re-plan triggers, read from the ledger:
//!
//! - canary leak: a verdict the server accepted (not a control run) failed
//!   with `canary_leak`: the sealed plan leaked;
//! - vacuity: a plan passed a product that does nothing for a promise it
//!   checks;
//! - proven false: a known-good control failed, or an in-scope known-broken
//!   control passed.
//!
//! Each trigger at a holdout's current version becomes a [`ReplanItem`]
//! that cites its evidence record and bumps the holdout version; the next
//! plan is drawn at the new version (the draw epoch). A trigger whose
//! evidence is already cited, or from an earlier version, does not fire
//! again.

use std::collections::{BTreeMap, BTreeSet};

use holden_canon::lens::ContentId;
use holden_eval::{Decision, FailReason};
use holden_gen::DrawCorpus;
use holden_seal::EvaluatorKey;

use crate::measure::Measurements;
use crate::records::{
    ControlKind, ControlRecord, Expect, RateRecord, ReplanItem, Trigger, VerdictEntry,
    WeightsRecord, kind,
};
use crate::stats::Rate;
use crate::store::{Entry, Ledger, LedgerError};

/// The weight scale (see the module docs).
pub const WEIGHT_SCALE: u64 = 4;

/// The weight of a lens with corpus weight `base` and yield `rate`.
#[must_use]
pub fn weight_of(base: u32, rate: &Rate) -> u32 {
    // round(SCALE × base × (k + 1) / (n + 2)) in integers.
    let num = WEIGHT_SCALE * u64::from(base) * (rate.k.min(rate.n) + 1);
    let den = rate.n + 2;
    let w = (num + den / 2) / den;
    u32::try_from(w.max(1)).unwrap_or(u32::MAX)
}

/// Every concept's weight in `corpus`, from the lens yields in `measured`.
#[must_use]
pub fn lens_weights(measured: &Measurements, corpus: &DrawCorpus) -> BTreeMap<ContentId, u32> {
    corpus
        .entries
        .iter()
        .map(|e| {
            let rate = measured
                .lenses
                .get(&e.concept.to_hex())
                .map(|s| s.catch)
                .unwrap_or_default();
            (e.concept, weight_of(e.weight, &rate))
        })
        .collect()
}

/// The weights record for `corpus` from `measured`.
#[must_use]
pub fn weights_record(
    measured: &Measurements,
    corpus: &DrawCorpus,
    at: Option<String>,
) -> WeightsRecord {
    let weights = lens_weights(measured, corpus);
    WeightsRecord {
        format: "holden.lens-weights/1".to_owned(),
        corpus: corpus
            .entries
            .iter()
            .map(|e| (e.concept.to_hex(), e.weight))
            .collect(),
        weights: weights.iter().map(|(c, w)| (c.to_hex(), *w)).collect(),
        basis: corpus
            .entries
            .iter()
            .map(|e| {
                let r = measured
                    .lenses
                    .get(&e.concept.to_hex())
                    .map(|s| s.catch)
                    .unwrap_or_default();
                (e.concept.to_hex(), RateRecord::from(r))
            })
            .collect(),
        at,
    }
}

/// Recompute the lens weights from the ledger, append them as a `weights`
/// record, and return the corpus the next draw uses.
///
/// # Errors
/// [`LedgerError`].
pub fn reweigh(
    ledger: &mut Ledger,
    key: &EvaluatorKey,
    corpus: &DrawCorpus,
) -> Result<(DrawCorpus, WeightsRecord), LedgerError> {
    let entries = ledger.entries()?;
    let measured = Measurements::from_entries(&entries)?;
    let at = entries.last().map(|e| format!("{}:{}", e.seq, e.hash));
    let record = weights_record(&measured, corpus, at);
    let weighted = corpus
        .with_weights(&lens_weights(&measured, corpus))
        .map_err(|e| LedgerError::Body(e.to_string()))?;
    ledger.append_json(key, kind::WEIGHTS, &record)?;
    Ok((weighted, record))
}

/// The current version of `holdout`: 1, plus one per re-plan bump.
///
/// # Errors
/// [`LedgerError::Body`] when a re-plan record does not decode.
pub fn holdout_version(entries: &[Entry], holdout: &str) -> Result<u64, LedgerError> {
    let mut v = 1;
    for e in entries.iter().filter(|e| e.kind == kind::REPLAN) {
        let item: ReplanItem = e.decode()?;
        if item.holdout == holdout {
            v = v.max(item.to_version);
        }
    }
    Ok(v)
}

/// A trigger found in the ledger, not yet acted on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    /// The holdout.
    pub holdout: String,
    /// The trigger.
    pub trigger: Trigger,
    /// The record that fired it.
    pub evidence_seq: u64,
    /// What fired it.
    pub evidence: String,
}

/// Every trigger in `entries` whose evidence is not yet cited by a re-plan
/// item and is at its holdout's current version.
///
/// # Errors
/// [`LedgerError::Body`] when a record does not decode.
pub fn pending_triggers(entries: &[Entry]) -> Result<Vec<Pending>, LedgerError> {
    let mut cited = BTreeSet::new();
    let mut versions: BTreeMap<String, u64> = BTreeMap::new();
    let mut controls = Vec::new();
    let mut verdicts = Vec::new();
    for e in entries {
        match e.kind.as_str() {
            kind::REPLAN => {
                let item: ReplanItem = e.decode()?;
                cited.insert(item.evidence_seq);
                let v = versions.entry(item.holdout).or_insert(1);
                *v = (*v).max(item.to_version);
            }
            kind::CONTROL => controls.push((e.seq, e.decode::<ControlRecord>()?)),
            kind::VERDICT => verdicts.push((e.seq, e.decode::<VerdictEntry>()?)),
            _ => {}
        }
    }
    let control_verdicts: BTreeSet<&str> = controls
        .iter()
        .map(|(_, c)| c.verdict_sha256.as_str())
        .collect();
    let holdout_of_plan: BTreeMap<&str, &str> = controls
        .iter()
        .map(|(_, c)| (c.plan_id.as_str(), c.holdout.as_str()))
        .collect();
    let mut out = Vec::new();
    for (seq, c) in &controls {
        let current = versions.get(&c.holdout).copied().unwrap_or(1);
        if cited.contains(seq) || c.version < current || !c.in_scope() {
            continue;
        }
        let passed = c.decision == Decision::Pass;
        let trigger = match (&c.expect, c.kind) {
            (Expect::Broken { .. }, ControlKind::DoNothing | ControlKind::PlanTargeted)
                if passed =>
            {
                Some(Trigger::Vacuity)
            }
            (Expect::Broken { .. }, _) if passed => Some(Trigger::ProvenFalse),
            (Expect::Good, _) if !passed => Some(Trigger::ProvenFalse),
            _ => None,
        };
        if let Some(trigger) = trigger {
            out.push(Pending {
                holdout: c.holdout.clone(),
                trigger,
                evidence_seq: *seq,
                evidence: format!("control {} verdict {}", c.control, c.verdict_sha256),
            });
        }
    }
    for (seq, v) in &verdicts {
        if cited.contains(seq)
            || control_verdicts.contains(v.verdict_sha256.as_str())
            || !v.reasons.contains(&FailReason::CanaryLeak)
        {
            continue;
        }
        let holdout = holdout_of_plan
            .get(v.plan_id.as_str())
            .map_or_else(|| format!("plan:{}", v.plan_id), |h| (*h).to_owned());
        out.push(Pending {
            holdout,
            trigger: Trigger::CanaryLeak,
            evidence_seq: *seq,
            evidence: format!("verdict {}", v.verdict_sha256),
        });
    }
    Ok(out)
}

fn item_text(t: Trigger) -> &'static str {
    match t {
        Trigger::CanaryLeak => {
            "the sealed plan leaked into an evaluated tree: draw and seal a new plan at the new version"
        }
        Trigger::Vacuity => {
            "the plan passed a product that does nothing for a promise it checks: re-plan, and review the binding of that promise"
        }
        Trigger::ProvenFalse => {
            "a control proved a verdict false: re-plan, and review the probe that missed or misfired"
        }
    }
}

/// Act on every pending trigger: append one re-plan item per trigger, each
/// bumping its holdout to the next version (all triggers of one holdout
/// found together share the bump).
///
/// # Errors
/// [`LedgerError`].
pub fn replan(ledger: &mut Ledger, key: &EvaluatorKey) -> Result<Vec<ReplanItem>, LedgerError> {
    let entries = ledger.entries()?;
    let pending = pending_triggers(&entries)?;
    let mut bumps: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for p in &pending {
        if !bumps.contains_key(&p.holdout) {
            let v = holdout_version(&entries, &p.holdout)?;
            bumps.insert(p.holdout.clone(), (v, v + 1));
        }
    }
    let mut out = Vec::new();
    for p in pending {
        let (from, to) = bumps.get(&p.holdout).copied().unwrap_or((1, 2));
        let item = ReplanItem {
            format: "holden.replan/1".to_owned(),
            holdout: p.holdout,
            trigger: p.trigger,
            evidence_seq: p.evidence_seq,
            evidence: p.evidence,
            from_version: from,
            to_version: to,
            item: item_text(p.trigger).to_owned(),
        };
        ledger.append_json(key, kind::REPLAN, &item)?;
        out.push(item);
    }
    Ok(out)
}
