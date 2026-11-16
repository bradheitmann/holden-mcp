//! The lens corpus model: design concepts and situated cognitions.
//!
//! Every id is the SHA-256 of the record's canonical bytes. There are no
//! timestamps and no positional ids, so the same content always has the same
//! id and an id cannot be forged without changing the content.
//!
//! Canonical bytes (`holden.lens/1`): a domain tag, then each field as an
//! 8-byte big-endian length followed by its UTF-8 bytes, in the order listed
//! on each type. Fixed-size values (ids, integers) are written raw, big-endian.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::digest::Sha256Digest;

/// File names of a corpus directory.
pub mod files {
    /// Concepts, ordered by id.
    pub const CONCEPTS: &str = "concepts.json";
    /// Situated cognitions, ordered by id.
    pub const COGNITIONS: &str = "cognitions.json";
    /// Ids of retired concepts, ordered.
    pub const RETIRED: &str = "retired.json";
    /// The deltas applied when the corpus was imported.
    pub const DELTAS: &str = "deltas.json";
}

/// A content-hash id.
pub type ContentId = Sha256Digest;

/// One constituent of a meta concept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constituent {
    /// The constituent concept's id.
    pub concept: ContentId,
    /// What the constituent contributes to the meta concept.
    pub contribution: String,
}

/// Whether a concept is original or synthesized from others.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ConceptKind {
    /// An original concept.
    Original,
    /// A concept synthesized from other concepts.
    Meta {
        /// Synthesis generation (1 = built from originals).
        generation: u32,
        /// The constituents, in source order.
        constituents: Vec<Constituent>,
    },
}

/// A design concept (a lens).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Concept {
    /// `H(canonical bytes)` of the fields below.
    pub id: ContentId,
    /// Short name.
    pub name: String,
    /// Category label.
    pub category: String,
    /// The question the lens asks.
    pub question: String,
    /// Original or meta.
    pub kind: ConceptKind,
}

/// A situated cognition: who meets the product, and in what situation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SituatedCognition {
    /// `H(canonical bytes)` of the fields below.
    pub id: ContentId,
    /// The role of the participant.
    pub role: String,
    /// Their level or stance.
    pub level: String,
    /// The circumstance they are in.
    pub circumstance: String,
}

struct Canon(Vec<u8>);

impl Canon {
    fn new(tag: &str) -> Self {
        let mut c = Self(Vec::new());
        c.text(tag);
        c
    }
    fn text(&mut self, s: &str) {
        self.0.extend_from_slice(&(s.len() as u64).to_be_bytes());
        self.0.extend_from_slice(s.as_bytes());
    }
    fn raw(&mut self, b: &[u8]) {
        self.0.extend_from_slice(b);
    }
    fn id(self) -> ContentId {
        Sha256Digest::of(&self.0)
    }
}

/// Compute a concept id from its content fields.
#[must_use]
pub fn concept_id(name: &str, category: &str, question: &str, kind: &ConceptKind) -> ContentId {
    let mut c = Canon::new("holden.lens/1 concept");
    c.text(name);
    c.text(category);
    c.text(question);
    match kind {
        ConceptKind::Original => c.text("original"),
        ConceptKind::Meta {
            generation,
            constituents,
        } => {
            c.text("meta");
            c.raw(&generation.to_be_bytes());
            c.raw(&(constituents.len() as u64).to_be_bytes());
            for k in constituents {
                c.raw(k.concept.as_bytes());
                c.text(&k.contribution);
            }
        }
    }
    c.id()
}

/// Compute a situated cognition id from its content fields.
#[must_use]
pub fn cognition_id(role: &str, level: &str, circumstance: &str) -> ContentId {
    let mut c = Canon::new("holden.lens/1 situated_cognition");
    c.text(role);
    c.text(level);
    c.text(circumstance);
    c.id()
}

impl Concept {
    /// Build a concept with its content-hash id.
    #[must_use]
    pub fn new(name: String, category: String, question: String, kind: ConceptKind) -> Self {
        let id = concept_id(&name, &category, &question, &kind);
        Self {
            id,
            name,
            category,
            question,
            kind,
        }
    }

    /// The id this concept's content hashes to.
    #[must_use]
    pub fn computed_id(&self) -> ContentId {
        concept_id(&self.name, &self.category, &self.question, &self.kind)
    }
}

impl SituatedCognition {
    /// Build a cognition with its content-hash id.
    #[must_use]
    pub fn new(role: String, level: String, circumstance: String) -> Self {
        let id = cognition_id(&role, &level, &circumstance);
        Self {
            id,
            role,
            level,
            circumstance,
        }
    }

    /// The id this cognition's content hashes to.
    #[must_use]
    pub fn computed_id(&self) -> ContentId {
        cognition_id(&self.role, &self.level, &self.circumstance)
    }
}

/// A lens corpus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LensCorpus {
    /// Concepts, ordered by id.
    pub concepts: Vec<Concept>,
    /// Situated cognitions, ordered by id.
    pub cognitions: Vec<SituatedCognition>,
    /// Retired concept ids, ordered.
    pub retired: Vec<ContentId>,
}

/// Why a corpus is invalid or could not be read.
#[derive(Debug)]
pub enum LensError {
    /// A record's id is not the hash of its content.
    IdNotContentHash {
        /// `concept` or `cognition`.
        record: &'static str,
        /// The id the record carries.
        claimed: ContentId,
        /// The id its content hashes to.
        computed: ContentId,
    },
    /// Two records share an id.
    DuplicateId(ContentId),
    /// A reference points to no concept in the corpus.
    DanglingReference(ContentId),
    /// Records are not ordered by id.
    NotOrdered(&'static str),
    /// A file could not be read or written.
    Io {
        /// The file.
        path: PathBuf,
        /// The error.
        source: std::io::Error,
    },
    /// A file is not valid JSON for the typed model.
    Json {
        /// The file.
        path: PathBuf,
        /// The error.
        source: serde_json::Error,
    },
    /// The import source broke the model.
    Source(String),
}

impl fmt::Display for LensError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdNotContentHash {
                record,
                claimed,
                computed,
            } => write!(
                f,
                "{record} id {claimed} is not the content hash of its record (computed {computed})"
            ),
            Self::DuplicateId(id) => write!(f, "duplicate id {id}"),
            Self::DanglingReference(id) => write!(f, "reference to unknown concept {id}"),
            Self::NotOrdered(what) => write!(f, "{what} are not ordered by id"),
            Self::Io { path, source } => write!(f, "io error at {}: {source}", path.display()),
            Self::Json { path, source } => write!(f, "json error in {}: {source}", path.display()),
            Self::Source(why) => write!(f, "source refused: {why}"),
        }
    }
}

impl std::error::Error for LensError {}

impl LensCorpus {
    /// Validate the corpus against the typed model: every id is the content
    /// hash of its record, ids are unique, references resolve, and records
    /// are ordered by id.
    ///
    /// # Errors
    /// Returns the first violation found.
    pub fn validate(&self) -> Result<(), LensError> {
        let mut ids = BTreeSet::new();
        for c in &self.concepts {
            let computed = c.computed_id();
            if computed != c.id {
                return Err(LensError::IdNotContentHash {
                    record: "concept",
                    claimed: c.id,
                    computed,
                });
            }
            if !ids.insert(c.id) {
                return Err(LensError::DuplicateId(c.id));
            }
        }
        for c in &self.concepts {
            if let ConceptKind::Meta { constituents, .. } = &c.kind {
                for k in constituents {
                    if !ids.contains(&k.concept) {
                        return Err(LensError::DanglingReference(k.concept));
                    }
                }
            }
        }
        let mut cog_ids = BTreeSet::new();
        for s in &self.cognitions {
            let computed = s.computed_id();
            if computed != s.id {
                return Err(LensError::IdNotContentHash {
                    record: "cognition",
                    claimed: s.id,
                    computed,
                });
            }
            if !cog_ids.insert(s.id) {
                return Err(LensError::DuplicateId(s.id));
            }
        }
        for r in &self.retired {
            if !ids.contains(r) {
                return Err(LensError::DanglingReference(*r));
            }
        }
        if !self.concepts.windows(2).all(|w| w[0].id < w[1].id) {
            return Err(LensError::NotOrdered("concepts"));
        }
        if !self.cognitions.windows(2).all(|w| w[0].id < w[1].id) {
            return Err(LensError::NotOrdered("cognitions"));
        }
        if !self.retired.windows(2).all(|w| w[0] < w[1]) {
            return Err(LensError::NotOrdered("retired ids"));
        }
        Ok(())
    }

    /// Load a corpus directory (see [`files`]) and validate it.
    ///
    /// # Errors
    /// Returns an I/O, JSON or validation error.
    pub fn load(dir: &Path) -> Result<Self, LensError> {
        let corpus = Self {
            concepts: read_json(&dir.join(files::CONCEPTS))?,
            cognitions: read_json(&dir.join(files::COGNITIONS))?,
            retired: read_json(&dir.join(files::RETIRED))?,
        };
        corpus.validate()?;
        Ok(corpus)
    }

    /// The JSON bytes of each corpus file, keyed by file name.
    ///
    /// # Errors
    /// Returns a JSON error (not expected for this model).
    pub fn to_files(&self) -> Result<BTreeMap<&'static str, Vec<u8>>, LensError> {
        let mut out = BTreeMap::new();
        out.insert(files::CONCEPTS, json_bytes(&self.concepts)?);
        out.insert(files::COGNITIONS, json_bytes(&self.cognitions)?);
        out.insert(files::RETIRED, json_bytes(&self.retired)?);
        Ok(out)
    }
}

/// Serialize a value as pretty JSON with a trailing newline.
///
/// # Errors
/// Returns a JSON error.
pub fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, LensError> {
    let mut v = serde_json::to_vec_pretty(value).map_err(|source| LensError::Json {
        path: PathBuf::from("<memory>"),
        source,
    })?;
    v.push(b'\n');
    Ok(v)
}

/// Read a typed JSON file.
///
/// # Errors
/// Returns an I/O or JSON error naming the file.
pub fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, LensError> {
    let bytes = fs::read(path).map_err(|source| LensError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| LensError::Json {
        path: path.to_path_buf(),
        source,
    })
}
