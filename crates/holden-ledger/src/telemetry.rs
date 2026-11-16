//! The telemetry report (slice-5 P4(c)): every measured rate with its
//! sample size and 95% Wilson bounds, each check's authority under the
//! bars, the latest lens weights, holdout versions and re-plan items.
//! Served by the MCP `telemetry` tool and `holden ledger telemetry`.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::authority::{Bars, authority};
use crate::measure::Measurements;
use crate::records::{ReplanItem, VerdictEntry, WeightsRecord, kind};
use crate::stats::{CheckStats, Rate};
use crate::store::{Entry, LedgerError};

/// The report format.
pub const TELEMETRY_FORMAT: &str = "holden.telemetry/1";

/// A rate as JSON: `k`, the sample size `n`, the point estimate and the
/// 95% Wilson bounds.
#[must_use]
pub fn rate_json(r: &Rate) -> Value {
    json!({
        "k": r.k,
        "n": r.n,
        "point": r.point(),
        "lower": r.lower(),
        "upper": r.upper(),
    })
}

fn stats_json(key_name: &str, key: &str, s: &CheckStats) -> Value {
    json!({
        key_name: key,
        "catch": rate_json(&s.catch),
        "false_fail": rate_json(&s.false_fail),
    })
}

fn table(key_name: &str, m: &BTreeMap<String, CheckStats>, bars: Option<&Bars>) -> Vec<Value> {
    m.iter()
        .map(|(k, s)| {
            let mut v = stats_json(key_name, k, s);
            if let (Some(b), Some(o)) = (bars, v.as_object_mut()) {
                o.insert("authority".to_owned(), json!(authority(s, b)));
            }
            v
        })
        .collect()
}

/// The telemetry report of `entries` under `bars`.
///
/// # Errors
/// [`LedgerError::Body`] when a record does not decode.
pub fn report(entries: &[Entry], bars: &Bars) -> Result<Value, LedgerError> {
    let m = Measurements::from_entries(entries)?;
    let mut weights: Option<WeightsRecord> = None;
    let mut replans: Vec<ReplanItem> = Vec::new();
    let mut verdicts = 0u64;
    for e in entries {
        match e.kind.as_str() {
            kind::WEIGHTS => weights = Some(e.decode()?),
            kind::REPLAN => replans.push(e.decode()?),
            kind::VERDICT => {
                let _: VerdictEntry = e.decode()?;
                verdicts += 1;
            }
            _ => {}
        }
    }
    let mut lenses = table("concept", &m.lenses, None);
    for l in &mut lenses {
        let w = l["concept"]
            .as_str()
            .and_then(|c| weights.as_ref().and_then(|wr| wr.weights.get(c)))
            .copied();
        if let Some(o) = l.as_object_mut() {
            o.insert("weight".to_owned(), json!(w));
        }
    }
    let mut versions: BTreeMap<String, u64> = BTreeMap::new();
    for r in &replans {
        let v = versions.entry(r.holdout.clone()).or_insert(1);
        *v = (*v).max(r.to_version);
    }
    Ok(json!({
        "format": TELEMETRY_FORMAT,
        "ledger": {
            "records": entries.len(),
            "tip": entries.last().map(|e| format!("{}:{}", e.seq, e.hash)),
        },
        "bars": bars,
        "controls": m.controls,
        "verdicts": verdicts,
        "checks": table("check", &m.checks, Some(bars)),
        "lenses": lenses,
        "cognitions": table("cognition", &m.cognitions, None),
        "plans": table("plan", &m.plans, Some(bars)),
        "integrity": table("rule", &m.integrity, Some(bars)),
        "selftest": table("defect", &m.selftest, None),
        "weights": weights,
        "holdout_versions": versions,
        "replan_items": replans,
    }))
}
