//! The keyed lens draw (slice-3 P6).
//!
//! Lens concepts for a plan are drawn on the sealed side:
//!
//! ```text
//! mac_i = HMAC-SHA-256(draw_key, DRAW_DOMAIN ‖ promise id ‖ cognition id ‖ epoch (u64 BE) ‖ i (u64 BE))
//! ```
//!
//! Pick `i` takes the first 8 bytes of `mac_i` as a big-endian integer `r`
//! and walks the remaining concepts (ordered by id) by corpus weight until
//! the cumulative weight exceeds `r mod total`. Picked concepts are removed,
//! so a draw never repeats a concept. A builder without the draw key cannot
//! compute any `mac_i`, so cannot predict which lens shapes a plan; an
//! auditor holding the key replays the draw from its recorded inputs.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use holden_canon::LensCorpus;
use holden_canon::lens::{self, ContentId};
use holden_seal::DrawKey;
use serde::{Deserialize, Serialize};

/// Domain prefix of every draw message.
pub const DRAW_DOMAIN: &[u8] = b"holden-gen/lens-draw/1";

/// Concepts drawn per plan.
pub const DRAW_PICKS: usize = 2;

/// The draw-weights file of a draw corpus directory.
pub const DRAW_FILE: &str = "draw.json";

/// The property a lens concept implies about the product.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Implication {
    /// The property, in the lens's words (never the owner's).
    pub property: String,
    /// Terms that an approved promise must contain, all of them, for the
    /// property to count as covered by the owner's words.
    pub terms: Vec<String>,
}

/// A concept's draw weight and implication.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrawEntry {
    /// The concept id.
    pub concept: ContentId,
    /// Corpus weight (at least 1).
    pub weight: u32,
    /// The property the concept implies.
    pub implies: Implication,
}

/// The `draw.json` file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrawFile {
    /// Always `holden.lens-draw/1`.
    pub format: String,
    /// Entries, ordered by concept id.
    pub entries: Vec<DrawEntry>,
}

/// A lens corpus with draw weights and implications.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawCorpus {
    /// The typed lens corpus.
    pub corpus: LensCorpus,
    /// Draw entries, ordered by concept id.
    pub entries: Vec<DrawEntry>,
}

/// Why a corpus or draw was refused.
#[derive(Debug)]
pub enum DrawError {
    /// The lens corpus is invalid.
    Lens(lens::LensError),
    /// The draw file is invalid.
    Corpus {
        /// The file.
        file: PathBuf,
        /// What is wrong.
        why: String,
    },
    /// The corpus has fewer drawable concepts than a draw picks.
    TooFewConcepts,
    /// A replayed draw differs from the recorded one.
    ReplayDiffers,
}

impl fmt::Display for DrawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lens(e) => write!(f, "refused: {e}"),
            Self::Corpus { file, why } => write!(f, "refused: {}: {why}", file.display()),
            Self::TooFewConcepts => f.write_str("refused: too few drawable lens concepts"),
            Self::ReplayDiffers => {
                f.write_str("refused: the replayed draw differs from the recorded draw")
            }
        }
    }
}

impl std::error::Error for DrawError {}

impl DrawCorpus {
    /// Load `dir/corpus/` (a typed lens corpus) and `dir/draw.json`.
    ///
    /// # Errors
    /// [`DrawError`] for an invalid corpus or draw file.
    pub fn load(dir: &Path) -> Result<Self, DrawError> {
        let corpus = LensCorpus::load(&dir.join("corpus")).map_err(DrawError::Lens)?;
        let file = dir.join(DRAW_FILE);
        let draw: DrawFile = lens::read_json(&file).map_err(DrawError::Lens)?;
        let refuse = |why: String| DrawError::Corpus {
            file: file.clone(),
            why,
        };
        if draw.format != "holden.lens-draw/1" {
            return Err(refuse(format!(
                "format '{}' is not holden.lens-draw/1",
                draw.format
            )));
        }
        let known: BTreeSet<ContentId> = corpus.concepts.iter().map(|c| c.id).collect();
        let retired: BTreeSet<ContentId> = corpus.retired.iter().copied().collect();
        if !draw.entries.windows(2).all(|w| w[0].concept < w[1].concept) {
            return Err(refuse("entries are not ordered by concept id".to_owned()));
        }
        for e in &draw.entries {
            if !known.contains(&e.concept) {
                return Err(refuse(format!("entry for unknown concept {}", e.concept)));
            }
            if retired.contains(&e.concept) {
                return Err(refuse(format!("entry for retired concept {}", e.concept)));
            }
            if e.weight == 0 {
                return Err(refuse(format!("concept {} has weight 0", e.concept)));
            }
            if e.implies.property.is_empty() || e.implies.terms.is_empty() {
                return Err(refuse(format!("concept {} implies nothing", e.concept)));
            }
        }
        Ok(Self {
            corpus,
            entries: draw.entries,
        })
    }

    /// The entry of `concept`.
    #[must_use]
    pub fn entry(&self, concept: &ContentId) -> Option<&DrawEntry> {
        self.entries.iter().find(|e| &e.concept == concept)
    }

    /// This corpus with the draw weight of every concept named in `weights`
    /// replaced (slice-5 P4(a): weights recomputed from ledger yield). A
    /// concept not named keeps its corpus weight. The keyed draw then picks
    /// by these weights; the draw key still decides every pick.
    ///
    /// # Errors
    /// [`DrawError::Corpus`] when a weight is 0 (a concept may be demoted,
    /// never made invisible) or names a concept the corpus does not hold.
    pub fn with_weights(
        &self,
        weights: &std::collections::BTreeMap<ContentId, u32>,
    ) -> Result<Self, DrawError> {
        let refuse = |why: String| DrawError::Corpus {
            file: PathBuf::from(DRAW_FILE),
            why,
        };
        for (concept, w) in weights {
            if self.entry(concept).is_none() {
                return Err(refuse(format!("weight for unknown concept {concept}")));
            }
            if *w == 0 {
                return Err(refuse(format!("concept {concept} has weight 0")));
            }
        }
        let mut out = self.clone();
        for e in &mut out.entries {
            if let Some(w) = weights.get(&e.concept) {
                e.weight = *w;
            }
        }
        Ok(out)
    }

    /// Every concept's current draw weight, by concept id.
    #[must_use]
    pub fn weights(&self) -> std::collections::BTreeMap<ContentId, u32> {
        self.entries.iter().map(|e| (e.concept, e.weight)).collect()
    }

    /// The concept record of `concept`.
    #[must_use]
    pub fn concept(&self, concept: &ContentId) -> Option<&holden_canon::Concept> {
        self.corpus.concepts.iter().find(|c| &c.id == concept)
    }
}

/// What a draw is keyed over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrawInputs {
    /// The approved promise.
    pub promise: ContentId,
    /// The approved cognition.
    pub cognition: ContentId,
    /// The draw epoch.
    pub epoch: u64,
}

/// A recorded lens draw: its inputs, the MAC of every pick and the picks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LensDraw {
    /// The inputs.
    pub inputs: DrawInputs,
    /// `mac_i` for each pick, 64 lowercase hex characters each.
    pub macs: Vec<String>,
    /// The picked concepts, in pick order.
    pub picks: Vec<ContentId>,
}

/// The message of pick `i`.
#[must_use]
pub fn draw_message(inputs: &DrawInputs, i: u64) -> Vec<u8> {
    let mut m = DRAW_DOMAIN.to_vec();
    m.extend_from_slice(inputs.promise.as_bytes());
    m.extend_from_slice(inputs.cognition.as_bytes());
    m.extend_from_slice(&inputs.epoch.to_be_bytes());
    m.extend_from_slice(&i.to_be_bytes());
    m
}

/// The keyed function every pick uses.
fn keyed(key: &DrawKey, message: &[u8]) -> [u8; 32] {
    key.mac(message)
}

/// Draw [`DRAW_PICKS`] lens concepts for `inputs` under `key`.
///
/// # Errors
/// [`DrawError::TooFewConcepts`] when the corpus cannot supply a full draw.
pub fn draw(
    key: &DrawKey,
    corpus: &DrawCorpus,
    inputs: &DrawInputs,
) -> Result<LensDraw, DrawError> {
    let mut remaining: Vec<&DrawEntry> = corpus.entries.iter().collect();
    if remaining.len() < DRAW_PICKS {
        return Err(DrawError::TooFewConcepts);
    }
    let mut macs = Vec::with_capacity(DRAW_PICKS);
    let mut picks = Vec::with_capacity(DRAW_PICKS);
    for i in 0..DRAW_PICKS as u64 {
        let mac = keyed(key, &draw_message(inputs, i));
        let total: u64 = remaining.iter().map(|e| u64::from(e.weight)).sum();
        let mut head = [0u8; 8];
        head.copy_from_slice(&mac[..8]);
        let mut r = u64::from_be_bytes(head) % total.max(1);
        let mut at = remaining.len() - 1;
        for (j, e) in remaining.iter().enumerate() {
            let w = u64::from(e.weight);
            if r < w {
                at = j;
                break;
            }
            r -= w;
        }
        picks.push(remaining.remove(at).concept);
        macs.push(hex::encode(mac));
    }
    Ok(LensDraw {
        inputs: *inputs,
        macs,
        picks,
    })
}

/// Replay `recorded` from its inputs under `key` and refuse any difference.
///
/// # Errors
/// [`DrawError::ReplayDiffers`] when the replay differs.
pub fn replay(key: &DrawKey, corpus: &DrawCorpus, recorded: &LensDraw) -> Result<(), DrawError> {
    let again = draw(key, corpus, &recorded.inputs)?;
    if &again == recorded {
        Ok(())
    } else {
        Err(DrawError::ReplayDiffers)
    }
}
