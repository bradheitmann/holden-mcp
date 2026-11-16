//! Authority from the pessimistic bound (slice-5 P3, P6(c)).
//!
//! A check's authority is computed only from its measured Wilson bounds
//! against owner-set [`Bars`], which live in the sealed server config:
//!
//! - `block` needs the catch-rate LOWER bound ≥ the block bar AND the
//!   false-fail UPPER bound ≤ the block bar;
//! - `recommend` needs the same against the recommend bars;
//! - anything else, and any check with fewer known-broken or known-good
//!   control trials than the minimum, is `advise` only.
//!
//! The point estimate is never used. A typed confinement limitation on the
//! verdict caps authority at `recommend`. A verdict's merge-blocking power is
//! the minimum authority across the checks that decided it.

use std::fmt;

use holden_eval::confine::Limitation;
use holden_eval::rules::CheckOutcome;
use holden_eval::{Decision, VerdictRecord};
use serde::{Deserialize, Serialize};

use crate::measure::{Measurements, check_key, integrity_key};
use crate::stats::CheckStats;

/// How much a check may do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Authority {
    /// It may only inform.
    Advise,
    /// It may recommend against merging.
    Recommend,
    /// It may block a merge.
    Block,
}

impl fmt::Display for Authority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Advise => "advise",
            Self::Recommend => "recommend",
            Self::Block => "block",
        })
    }
}

/// One authority bar.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bar {
    /// The least catch-rate lower bound.
    pub catch_lower_min: f64,
    /// The most false-fail upper bound.
    pub false_fail_upper_max: f64,
}

/// The owner-set bars.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bars {
    /// The block bar.
    pub block: Bar,
    /// The recommend bar.
    pub recommend: Bar,
    /// The fewest known-broken control trials a measured check has.
    pub min_broken_trials: u64,
    /// The fewest known-good control trials a measured check has.
    pub min_good_trials: u64,
}

impl Bars {
    /// The reasoned default (slice-5 done list): block at catch lower bound
    /// ≥ 0.90 and false-fail upper bound ≤ 0.05. The recommend bar
    /// (≥ 0.50, ≤ 0.25) and the minimum samples (10 and 10) are this
    /// slice's reasoned defaults, open to the owner.
    pub const DEFAULT: Self = Self {
        block: Bar {
            catch_lower_min: 0.90,
            false_fail_upper_max: 0.05,
        },
        recommend: Bar {
            catch_lower_min: 0.50,
            false_fail_upper_max: 0.25,
        },
        min_broken_trials: 10,
        min_good_trials: 10,
    };

    /// Refuse bars that are out of range, or where block is weaker than
    /// recommend.
    ///
    /// # Errors
    /// What is wrong.
    pub fn validate(&self) -> Result<(), String> {
        let unit = |v: f64| (0.0..=1.0).contains(&v);
        for (name, b) in [("block", self.block), ("recommend", self.recommend)] {
            if !unit(b.catch_lower_min) || !unit(b.false_fail_upper_max) {
                return Err(format!("the {name} bar is outside [0, 1]"));
            }
        }
        if self.block.catch_lower_min < self.recommend.catch_lower_min
            || self.block.false_fail_upper_max > self.recommend.false_fail_upper_max
        {
            return Err("the block bar is weaker than the recommend bar".to_owned());
        }
        if self.min_broken_trials == 0 || self.min_good_trials == 0 {
            return Err("the minimum trials must be at least 1".to_owned());
        }
        Ok(())
    }
}

impl Default for Bars {
    fn default() -> Self {
        Self::DEFAULT
    }
}

fn meets(stats: &CheckStats, bar: &Bar) -> bool {
    stats.catch.lower() >= bar.catch_lower_min
        && stats.false_fail.upper() <= bar.false_fail_upper_max
}

/// The authority `stats` earn under `bars`, from the pessimistic bounds
/// only.
#[must_use]
pub fn authority(stats: &CheckStats, bars: &Bars) -> Authority {
    if stats.catch.n < bars.min_broken_trials || stats.false_fail.n < bars.min_good_trials {
        return Authority::Advise;
    }
    if meets(stats, &bars.block) {
        Authority::Block
    } else if meets(stats, &bars.recommend) {
        Authority::Recommend
    } else {
        Authority::Advise
    }
}

/// `a`, capped at `recommend` when there is any limitation.
#[must_use]
pub fn capped(a: Authority, limitations: &[Limitation]) -> Authority {
    if limitations.is_empty() {
        a
    } else {
        a.min(Authority::Recommend)
    }
}

/// A verdict's merge-blocking power.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerdictAuthority {
    /// The minimum across the deciding checks, after the cap.
    pub authority: Authority,
    /// Each deciding check (or integrity rule) and its own authority.
    pub deciding: Vec<(String, Authority)>,
    /// The limitations that capped it.
    pub capped_by: Vec<Limitation>,
}

/// The authority of `record`: the minimum authority across the checks that
/// decided it (every check for a pass; for a fail, each check that did not
/// hold and each integrity rule that failed it), capped by the record's
/// confinement limitations. With no measurement, every check is `advise`.
#[must_use]
pub fn verdict_authority(
    record: &VerdictRecord,
    measured: &Measurements,
    bars: &Bars,
) -> VerdictAuthority {
    let b = &record.body;
    let mut deciding = Vec::new();
    let of = |key: &str, stats: Option<&CheckStats>| {
        (
            key.to_owned(),
            stats.map_or(Authority::Advise, |s| authority(s, bars)),
        )
    };
    for (c, r) in b.checks.iter().zip(&b.decided.results) {
        if b.decided.decision == Decision::Pass || r.outcome != CheckOutcome::Held {
            let key = check_key(&b.plan_id, &c.id);
            deciding.push(of(&key, measured.checks.get(&key)));
        }
    }
    for reason in &b.decided.reasons {
        if let Some(key) = integrity_key(*reason) {
            deciding.push(of(&key, measured.integrity.get(&key)));
        }
    }
    let least = deciding
        .iter()
        .map(|(_, a)| *a)
        .min()
        .unwrap_or(Authority::Advise);
    let limitations = b.confinement.limitations.clone();
    VerdictAuthority {
        authority: capped(least, &limitations),
        deciding,
        capped_by: limitations,
    }
}
