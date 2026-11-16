//! Candidate situated cognitions (docs/ARCHITECTURE.md §3.1 steps 2 and 3).
//!
//! For each promise, every doctrine generator rule proposes one candidate
//! per constituency it yields. The candidate's circumstance takes one typed
//! level on each axis the rule yields. Levels are chosen from a SHA-256 of
//! the inputs (promise id, root, constituency and, with a pack, the domain
//! overlay), so the same inputs always give the same candidates and nothing
//! random enters here: all randomness is at test time (L3).
//!
//! Each candidate's id is the SHA-256 of its canonical bytes, and it cites
//! the root and axes that produced it and the promise span it rests on.

use holden_canon::Sha256Digest;
use holden_canon::doctrine::{AxisLevel, CircumstanceAxis, Constituency, Doctrine, RootId};
use serde::{Deserialize, Serialize};

use crate::forge_pack::DomainOverlay;
use crate::promise::{ContentId, Promise};

/// The promise span a cognition rests on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// SHA-256 of the source file.
    pub source: Sha256Digest,
    /// Start byte offset.
    pub start: u64,
    /// End byte offset.
    pub end: u64,
}

/// Which pack fields shaped a cognition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayTrace {
    /// The pack name.
    pub pack_name: String,
    /// SHA-256 of the pack file.
    pub pack_digest: Sha256Digest,
    /// The pack's domain line.
    pub domain: String,
    /// JSON pointer of the worldview that frames the role.
    pub worldview: String,
    /// JSON pointer of the signature failure the participant cannot assume away.
    pub failure: String,
}

/// A candidate situated cognition: who meets the product, in what
/// circumstance, relying on which promise.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SituatedCognition {
    /// `H(canonical bytes)` of the fields below.
    pub id: ContentId,
    /// The promise it relies on.
    pub promise: ContentId,
    /// The constituency.
    pub constituency: Constituency,
    /// Who the participant is.
    pub role: String,
    /// One typed level per axis the generating rule yields.
    pub circumstance: Vec<AxisLevel>,
    /// What the participant cannot assume.
    pub cannot_assume: Vec<String>,
    /// What they are trying to do.
    pub goal: String,
    /// The pressure they are under.
    pub pressure: String,
    /// The root(s) that produced it.
    pub roots: Vec<RootId>,
    /// The axes that produced it.
    pub axes: Vec<CircumstanceAxis>,
    /// The promise spans it rests on.
    pub evidence: Vec<Evidence>,
    /// The domain overlay that shaped it, if any.
    pub overlay: Option<OverlayTrace>,
}

#[derive(Serialize)]
struct Body<'a> {
    promise: &'a ContentId,
    constituency: &'a Constituency,
    role: &'a str,
    circumstance: &'a [AxisLevel],
    cannot_assume: &'a [String],
    goal: &'a str,
    pressure: &'a str,
    roots: &'a [RootId],
    axes: &'a [CircumstanceAxis],
    evidence: &'a [Evidence],
    overlay: &'a Option<OverlayTrace>,
}

const ID_TAG: &[u8] = b"holden.cognition/1\n";

impl SituatedCognition {
    /// The id this cognition's content hashes to.
    #[must_use]
    pub fn computed_id(&self) -> ContentId {
        let body = Body {
            promise: &self.promise,
            constituency: &self.constituency,
            role: &self.role,
            circumstance: &self.circumstance,
            cannot_assume: &self.cannot_assume,
            goal: &self.goal,
            pressure: &self.pressure,
            roots: &self.roots,
            axes: &self.axes,
            evidence: &self.evidence,
            overlay: &self.overlay,
        };
        // Serializing this plain struct cannot fail; an empty body would
        // still hash to an id no real cognition has.
        let mut bytes = ID_TAG.to_vec();
        bytes.extend(serde_json::to_vec(&body).unwrap_or_default());
        Sha256Digest::of(&bytes)
    }
}

fn base_role(c: Constituency) -> &'static str {
    match c {
        Constituency::Human => "a person meeting the product",
        Constituency::Agent => "an AI agent meeting the product through its schemas and text",
        Constituency::Seam => "the hand-off where a human and an agent pass this work between them",
        Constituency::Successor => {
            "a future participant who inherits the product without having been present when its rules were chosen"
        }
    }
}

fn seed(parts: &[&[u8]]) -> [u8; 32] {
    let mut b = Vec::new();
    for p in parts {
        b.extend_from_slice(&(p.len() as u64).to_be_bytes());
        b.extend_from_slice(p);
    }
    *Sha256Digest::of(&b).as_bytes()
}

fn pick(seed: &[u8; 32], slot: usize, n: usize) -> usize {
    let i = (slot * 2) % 32;
    let v = u16::from_be_bytes([seed[i], seed[(i + 1) % 32]]);
    usize::from(v) % n.max(1)
}

/// The candidate cognitions the doctrine generators propose for `promise`,
/// optionally shaped by a pack's domain overlay. Sorted by id.
#[must_use]
pub fn candidates(
    doctrine: &Doctrine,
    promise: &Promise,
    overlay: Option<&DomainOverlay>,
) -> Vec<SituatedCognition> {
    let mut out = Vec::new();
    for rule in &doctrine.generators {
        let Some(root) = doctrine.root(rule.root) else {
            continue;
        };
        for &constituency in &rule.constituencies {
            let base = seed(&[
                b"holden.cognition/1 seed",
                promise.id.as_bytes(),
                rule.root.as_str().as_bytes(),
                constituency.as_str().as_bytes(),
            ]);
            let mut role = base_role(constituency).to_owned();
            let mut cannot_assume = root.cannot_assume.clone();
            let mut trace = None;
            let mut levels_seed = base;
            if let Some(o) = overlay
                && let (Some(w), Some(f)) = (
                    o.worldviews.get(pick(&base, 0, o.worldviews.len())),
                    o.failures.get(pick(&base, 1, o.failures.len())),
                )
            {
                role = format!("{role}, in this domain: {}", o.domain);
                cannot_assume.push(format!(
                    "that this cannot happen: {} (guard: {})",
                    f.failure, f.guard
                ));
                levels_seed = seed(&[
                    &base,
                    o.domain.as_bytes(),
                    w.id.as_bytes(),
                    w.name.as_bytes(),
                    f.failure.as_bytes(),
                ]);
                trace = Some(OverlayTrace {
                    pack_name: o.pack_name.clone(),
                    pack_digest: o.pack_digest,
                    domain: o.domain.clone(),
                    worldview: w.trace.clone(),
                    failure: f.trace.clone(),
                });
            }
            let circumstance: Vec<AxisLevel> = rule
                .axes
                .iter()
                .enumerate()
                .filter_map(|(i, axis)| {
                    let levels = axis.levels();
                    levels.get(pick(&levels_seed, i + 2, levels.len())).copied()
                })
                .collect();
            let pressure = circumstance
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            let mut c = SituatedCognition {
                id: Sha256Digest::from_bytes([0; 32]),
                promise: promise.id,
                constituency,
                role,
                circumstance,
                cannot_assume,
                goal: format!("to rely on the owner's promise: \"{}\"", promise.text),
                pressure,
                roots: vec![rule.root],
                axes: rule.axes.clone(),
                evidence: vec![Evidence {
                    source: promise.source,
                    start: promise.start,
                    end: promise.end,
                }],
                overlay: trace,
            };
            c.id = c.computed_id();
            out.push(c);
        }
    }
    out.sort_by_key(|c| c.id);
    out.dedup_by(|a, b| a.id == b.id);
    out
}
