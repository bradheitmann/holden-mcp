//! Planted values: what the evaluator puts into the product, derived only
//! from the server's moment-of-test parameters (slice-4 P2).
//!
//! The builder sees the [`BoundaryCard`]: the product's public boundary and
//! one worked example. The evaluator never plants the card's example; it
//! plants values derived from the revealed parameter bytes, so they differ
//! for every evaluated commit and every run, and no builder can know them
//! before the test.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Domain of the derivation.
pub const PLANT_DOMAIN: &[u8] = b"holden-eval/plant/1";

/// One planted note.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlantedNote {
    /// Its name.
    pub name: String,
    /// Its body.
    pub body: String,
}

/// The values planted in one evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Planted {
    /// The notes, in import order.
    pub notes: Vec<PlantedNote>,
    /// How many complete records precede the interruption.
    pub interrupt_after: usize,
    /// The name of the export file.
    pub export_name: String,
    /// Which note the delete probe deletes.
    pub delete_index: usize,
}

/// The builder-visible card: the product's public boundary and one worked
/// example. Publishing it is "publish the why"; the planted values are the
/// sealed "how".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryCard {
    /// The product's public commands.
    pub commands: Vec<String>,
    /// The worked example.
    pub example: Planted,
}

fn stream(bytes: &[u8; 32], label: &str, i: u64) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(PLANT_DOMAIN);
    h.update([0]);
    h.update(label.as_bytes());
    h.update([0]);
    h.update(bytes);
    h.update(i.to_be_bytes());
    h.finalize().into()
}

/// `n` characters of `alphabet` chosen by the stream `label`.
fn chars(bytes: &[u8; 32], label: &str, n: usize, alphabet: &[u8]) -> String {
    let mut out = String::with_capacity(n);
    let mut block = 0u64;
    while out.len() < n {
        for b in stream(bytes, label, block) {
            if out.len() == n {
                break;
            }
            out.push(char::from(alphabet[usize::from(b) % alphabet.len()]));
        }
        block += 1;
    }
    out
}

const NAME_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
const BODY_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz ,.0123456789";

impl Planted {
    /// Derive the planted values from the revealed parameter bytes.
    #[must_use]
    pub fn derive(param_bytes: &[u8; 32]) -> Self {
        let pick = |label: &str| stream(param_bytes, label, 0)[0];
        let count = 4 + usize::from(pick("count")) % 4;
        let interrupt_after = 2 + usize::from(pick("interrupt")) % (count - 2);
        let long = usize::from(pick("long")) % interrupt_after;
        let notes = (0..count)
            .map(|k| {
                let size = stream(param_bytes, &format!("size-{k}"), 0)[0];
                let len = if k == long {
                    80 + usize::from(size) % 80
                } else {
                    12 + usize::from(size) % 40
                };
                PlantedNote {
                    name: format!(
                        "n{k}-{}",
                        chars(param_bytes, &format!("name-{k}"), 10, NAME_ALPHABET)
                    ),
                    body: chars(param_bytes, &format!("body-{k}"), len, BODY_ALPHABET),
                }
            })
            .collect();
        Self {
            notes,
            interrupt_after,
            export_name: format!("x-{}", chars(param_bytes, "export", 8, NAME_ALPHABET)),
            delete_index: usize::from(pick("delete")) % count,
        }
    }

    /// Every planted string (names, bodies, export name), for leak checks.
    #[must_use]
    pub fn secrets(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .notes
            .iter()
            .flat_map(|n| [n.name.clone(), n.body.clone()])
            .collect();
        out.push(self.export_name.clone());
        out
    }
}

/// The notes product's card.
#[must_use]
pub fn notes_card() -> BoundaryCard {
    let note = |name: &str, body: &str| PlantedNote {
        name: name.to_owned(),
        body: body.to_owned(),
    };
    BoundaryCard {
        commands: vec![
            "notes --store <dir> import   (records name<TAB>body<LF> on stdin)".to_owned(),
            "notes --store <dir> list".to_owned(),
            "notes --store <dir> get <name>".to_owned(),
            "notes --store <dir> delete <name> [--yes]".to_owned(),
            "notes --store <dir> export <path>".to_owned(),
        ],
        example: Planted {
            notes: vec![
                note("groceries", "milk, eggs, bread"),
                note("dentist", "call before noon on friday"),
                note("ideas", "a shelf for the hallway"),
            ],
            interrupt_after: 2,
            export_name: "backup".to_owned(),
            delete_index: 0,
        },
    }
}
