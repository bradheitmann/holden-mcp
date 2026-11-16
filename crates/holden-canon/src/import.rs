//! Import the S-LENS JSON registries into the typed lens corpus.
//!
//! Only the declared JSON registries are read. Nothing in the source directory
//! is executed, and no other file is opened. The import applies the declared
//! deltas (see `canon/LENS-DELTAS.md`):
//!
//! - D-LENS-1: positional and source ids (`seq`, `gid`, `SC-nnn`) are dropped;
//!   every id becomes the content hash of the record.
//! - D-LENS-2: timestamped provenance (`created`, model, harness, repository
//!   and commit fields) is dropped.
//! - D-LENS-3: the stereotyped situated cognition is rewritten into a
//!   stereotype-free situation with the same cognitive load. The match is by
//!   the SHA-256 of the old text; only hashes are recorded.
//! - D-LENS-4: no executable script is read, copied or run.
//! - D-LENS-5: the source's unseeded draw state (`remainders`) is dropped;
//!   draws come from the VRF in `holden-seal`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::digest::Sha256Digest;
use crate::lens::{
    Concept, ConceptKind, Constituent, ContentId, LensCorpus, LensError, SituatedCognition, files,
    json_bytes, read_json,
};
use crate::pin::{Pin, PinnedPath};

/// The registry files read from the source directory. Nothing else is opened.
pub mod source_files {
    /// Concepts (original and meta).
    pub const CONCEPTS: &str = "concept-registry.json";
    /// Meta concepts with constituent contributions.
    pub const META: &str = "meta-registry.json";
    /// Retired concept source ids.
    pub const RETIRED: &str = "retired.json";
    /// Unseeded draw state.
    pub const REMAINDERS: &str = "remainders.json";
    /// Situated cognitions.
    pub const COGNITIONS: &str = "situated-cognitions.json";
}

/// SHA-256 of the stereotyped circumstance text (D-LENS-3). The text itself
/// is not kept in this repository.
pub const STEREOTYPED_CIRCUMSTANCE_SHA256: &str =
    "0c1c9e705edd6cc4d16017e00d823cde5d83ee1ec9ab4d9deb5630c3ecfb6063";

/// The stereotype-free replacement (D-LENS-3). It keeps the cognitive load:
/// two concurrent, time-critical demands and fragmented attention.
pub const REWRITTEN_CIRCUMSTANCE: &str = "Using the product while handling two simultaneous time-critical tasks that each demand attention at unpredictable moments; attention fragmented";

/// The canon version label written into a lens manifest.
pub const LENS_CORPUS_VERSION: &str = "holden.lens/1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // positional and timestamped fields are read only to be dropped
struct SourceProvenance {
    source: String,
    created: String,
    harness: String,
    repo: Option<String>,
    commit: Option<String>,
    constituents: Option<Vec<String>>,
    north_star_check: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct SourceConcept {
    seq: u64,
    gid: String,
    #[serde(rename = "type")]
    kind: String,
    name: String,
    cat: String,
    q: String,
    generation: Option<u32>,
    provenance: SourceProvenance,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct SourceConstituent {
    gid: String,
    name: String,
    contribution: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct SourceMeta {
    gid: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    generation: u32,
    q: String,
    cat: String,
    constituents: Vec<SourceConstituent>,
    model: String,
    harness: String,
    repo: String,
    commit: String,
    created: String,
    north_star_check: String,
    cycle: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceCognition {
    id: String,
    role: String,
    level: String,
    circumstance: String,
}

/// One applied delta. Records carry hashes and counts, never source text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "delta", rename_all = "snake_case", deny_unknown_fields)]
pub enum Delta {
    /// D-LENS-1.
    DropPositionalIds {
        /// Concepts whose `seq`/`gid` were dropped.
        concepts: usize,
        /// Cognitions whose positional id was dropped.
        cognitions: usize,
    },
    /// D-LENS-2.
    DropTimestampedProvenance {
        /// Records whose provenance was dropped.
        records: usize,
    },
    /// D-LENS-3.
    RewriteStereotypedCognition {
        /// SHA-256 of the old circumstance text.
        old_text_sha256: Sha256Digest,
        /// SHA-256 of the new circumstance text.
        new_text_sha256: Sha256Digest,
        /// Content id of the rewritten cognition; `None` when the source did
        /// not contain the stereotyped text (the delta did not apply).
        new_cognition_id: Option<ContentId>,
    },
    /// D-LENS-4.
    NoExecutableScript {
        /// Number of source-directory entries that were not read.
        entries_not_read: usize,
    },
    /// D-LENS-5.
    DropUnseededDrawState {
        /// Number of draw-state entries dropped.
        entries: usize,
    },
}

/// The outcome of an import.
#[derive(Debug)]
pub struct ImportOutcome {
    /// The typed corpus (validated).
    pub corpus: LensCorpus,
    /// The deltas applied.
    pub deltas: Vec<Delta>,
}

fn src(why: impl Into<String>) -> LensError {
    LensError::Source(why.into())
}

/// Convert the S-LENS registries in `from` into a typed corpus.
///
/// # Errors
/// Returns an error when a registry is missing, does not match the expected
/// typed shape, or breaks the corpus model.
pub fn import_lens(from: &Path) -> Result<ImportOutcome, LensError> {
    let concepts: Vec<SourceConcept> = read_json(&from.join(source_files::CONCEPTS))?;
    let metas: Vec<SourceMeta> = read_json(&from.join(source_files::META))?;
    let retired_src: Vec<String> = read_json(&from.join(source_files::RETIRED))?;
    let remainders: Vec<String> = read_json(&from.join(source_files::REMAINDERS))?;
    let cognitions_src: Vec<SourceCognition> = read_json(&from.join(source_files::COGNITIONS))?;

    let read_set = [
        source_files::CONCEPTS,
        source_files::META,
        source_files::RETIRED,
        source_files::REMAINDERS,
        source_files::COGNITIONS,
    ];
    let entries_not_read = fs::read_dir(from)
        .map_err(|source| LensError::Io {
            path: from.to_path_buf(),
            source,
        })?
        .filter_map(Result::ok)
        .filter(|e| !read_set.iter().any(|n| e.file_name() == *n))
        .count();

    let meta_by_gid: BTreeMap<&str, &SourceMeta> =
        metas.iter().map(|m| (m.gid.as_str(), m)).collect();

    // Originals first, so metas can reference them by content id.
    let mut gid_to_id: BTreeMap<String, ContentId> = BTreeMap::new();
    let mut out_concepts: Vec<Concept> = Vec::new();
    for c in concepts.iter().filter(|c| c.kind == "original") {
        if c.generation.is_some() || c.provenance.constituents.is_some() {
            return Err(src(format!("original {} carries meta fields", c.seq)));
        }
        let concept = Concept::new(
            c.name.clone(),
            c.cat.clone(),
            c.q.clone(),
            ConceptKind::Original,
        );
        if gid_to_id.insert(c.gid.clone(), concept.id).is_some() {
            return Err(src(format!("duplicate source id at seq {}", c.seq)));
        }
        out_concepts.push(concept);
    }
    let mut metas_seen = 0usize;
    for c in concepts.iter().filter(|c| c.kind != "original") {
        if c.kind != "meta" {
            return Err(src(format!("unknown concept type at seq {}", c.seq)));
        }
        let meta = meta_by_gid
            .get(c.gid.as_str())
            .ok_or_else(|| src(format!("meta concept at seq {} has no meta record", c.seq)))?;
        metas_seen += 1;
        let listed: Vec<&str> = c
            .provenance
            .constituents
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(String::as_str)
            .collect();
        let detailed: Vec<&str> = meta.constituents.iter().map(|k| k.gid.as_str()).collect();
        if listed != detailed {
            return Err(src(format!(
                "meta concept at seq {} lists different constituents in its two records",
                c.seq
            )));
        }
        if c.generation != Some(meta.generation) || c.q != meta.q || c.name != meta.name {
            return Err(src(format!(
                "meta concept at seq {} disagrees with its meta record",
                c.seq
            )));
        }
        let mut constituents = Vec::new();
        for k in &meta.constituents {
            let id = gid_to_id.get(&k.gid).ok_or_else(|| {
                src(format!(
                    "meta concept at seq {} references an unknown concept",
                    c.seq
                ))
            })?;
            constituents.push(Constituent {
                concept: *id,
                contribution: k.contribution.clone(),
            });
        }
        let concept = Concept::new(
            c.name.clone(),
            c.cat.clone(),
            c.q.clone(),
            ConceptKind::Meta {
                generation: meta.generation,
                constituents,
            },
        );
        if gid_to_id.insert(c.gid.clone(), concept.id).is_some() {
            return Err(src(format!("duplicate source id at seq {}", c.seq)));
        }
        out_concepts.push(concept);
    }
    if metas_seen != metas.len() {
        return Err(src(
            "meta registry has records absent from the concept registry",
        ));
    }

    let mut retired = Vec::new();
    for gid in &retired_src {
        retired.push(
            *gid_to_id
                .get(gid)
                .ok_or_else(|| src("retired list references an unknown concept"))?,
        );
    }

    let stereotyped: Sha256Digest = STEREOTYPED_CIRCUMSTANCE_SHA256
        .parse()
        .map_err(|_| src("internal: bad stereotype hash constant"))?;
    let new_text_sha256 = Sha256Digest::of(REWRITTEN_CIRCUMSTANCE.as_bytes());
    let mut rewrite: Option<ContentId> = None;
    let mut out_cognitions = Vec::new();
    for s in &cognitions_src {
        if s.id.is_empty() {
            return Err(src("cognition without a source id"));
        }
        let circumstance = if Sha256Digest::of(s.circumstance.as_bytes()) == stereotyped {
            REWRITTEN_CIRCUMSTANCE.to_owned()
        } else {
            s.circumstance.clone()
        };
        let cog = SituatedCognition::new(s.role.clone(), s.level.clone(), circumstance);
        if Sha256Digest::of(s.circumstance.as_bytes()) == stereotyped {
            rewrite = Some(cog.id);
        }
        out_cognitions.push(cog);
    }

    out_concepts.sort_by_key(|c| c.id);
    out_cognitions.sort_by_key(|c| c.id);
    retired.sort();
    retired.dedup();

    let corpus = LensCorpus {
        concepts: out_concepts,
        cognitions: out_cognitions,
        retired,
    };
    corpus.validate()?;

    let deltas = vec![
        Delta::DropPositionalIds {
            concepts: concepts.len(),
            cognitions: cognitions_src.len(),
        },
        Delta::DropTimestampedProvenance {
            records: concepts.len() + metas.len(),
        },
        Delta::RewriteStereotypedCognition {
            old_text_sha256: stereotyped,
            new_text_sha256,
            new_cognition_id: rewrite,
        },
        Delta::NoExecutableScript { entries_not_read },
        Delta::DropUnseededDrawState {
            entries: remainders.len(),
        },
    ];
    Ok(ImportOutcome { corpus, deltas })
}

/// Write an imported corpus to `to`: the corpus files under `to/corpus/` and
/// the pin of those files at `to/MANIFEST.sha256`. Returns the manifest.
///
/// # Errors
/// Refuses a non-empty `to`, and returns I/O errors.
pub fn write_corpus(to: &Path, outcome: &ImportOutcome) -> Result<Pin, LensError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| LensError::Io { path, source }
    };
    if to.exists() {
        let mut entries = fs::read_dir(to).map_err(io(to))?;
        if entries.next().is_some() {
            return Err(src(format!(
                "destination {} is not empty; refusing to overwrite",
                to.display()
            )));
        }
    }
    let corpus_dir = to.join("corpus");
    fs::create_dir_all(&corpus_dir).map_err(io(&corpus_dir))?;

    let mut contents = outcome.corpus.to_files()?;
    contents.insert(files::DELTAS, json_bytes(&outcome.deltas)?);

    let mut pinned = BTreeMap::new();
    for (name, bytes) in &contents {
        let path = corpus_dir.join(name);
        fs::write(&path, bytes).map_err(io(&path))?;
        let p = PinnedPath::new(name).map_err(|e| src(format!("internal: {e}")))?;
        pinned.insert(p, Sha256Digest::of(bytes));
    }
    let pin = Pin {
        canon_version: LENS_CORPUS_VERSION.to_owned(),
        validator_hash: None,
        files: pinned,
    };
    let manifest = to.join("MANIFEST.sha256");
    let text = pin.render(
        "Lens corpus manifest (holden.pin/1). Hashes only; the corpus itself is private.\nVerify: holden canon verify --root <corpus dir> --pin <this file>",
    );
    fs::write(&manifest, text).map_err(io(&manifest))?;
    Ok(pin)
}
