//! The typed forge-pack loader (docs/ARCHITECTURE.md §3.7).
//!
//! The forge (`modules/holden-forge`) is a file contract only: this crate
//! has no dependency on it. The loader reads what `forge export` writes,
//! purely as files:
//!
//! - `packs/<pack>/PACK.json`: the domain pack, schema `holden-forge.pack/1`,
//!   in the forge's canonical layout (two-space indent, members in schema
//!   order, final newline);
//! - `agents/<pack>/<role>.md`: the four agent constitutions emitted from it.
//!
//! Loading is strict and lossless: an unknown or missing member is refused,
//! re-serialising the typed pack must give back the file's exact bytes, and
//! re-rendering each constitution must give back its exact text. Each
//! constitution must name the pack and its digest, and every entry's text
//! must be the pack value its trace points at.
//!
//! The pack's domain overlay ([`DomainOverlay`]) is what changes candidate
//! cognitions: the domain line, the practitioner worldviews and the
//! signature failures of the pack's items.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use holden_canon::Sha256Digest;
use serde::{Deserialize, Serialize};

/// The pack schema id this loader reads.
pub const PACK_SCHEMA: &str = "holden-forge.pack/1";

/// The constitution text format this loader reads.
pub const CONSTITUTION_FORMAT: &str = "1";

/// The four holden agent roles, in the forge's fixed order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Planner.
    Planner,
    /// Sealer.
    Sealer,
    /// Evaluator.
    Evaluator,
    /// Adversary.
    Adversary,
}

impl Role {
    /// Every role, in the forge's order.
    pub const ALL: [Self; 4] = [
        Self::Planner,
        Self::Sealer,
        Self::Evaluator,
        Self::Adversary,
    ];

    /// The role's name in packs and file names.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planner => "planner",
            Self::Sealer => "sealer",
            Self::Evaluator => "evaluator",
            Self::Adversary => "adversary",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.as_str() == s)
    }
}

/// A practitioner perspective.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Worldview {
    /// Stable id.
    pub id: String,
    /// Name.
    pub name: String,
    /// When it applies.
    pub applies_when: String,
}

/// An evaluation criterion of an item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    /// Id.
    pub criterion_id: String,
    /// Question.
    pub question: String,
    /// Pass condition.
    pub pass_condition: String,
    /// Checklist line.
    pub checklist: String,
}

/// A signature failure and its guard.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    /// The failure.
    pub failure: String,
    /// Its guard.
    pub guard: String,
}

/// One item (the CCII depth contract).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    /// Item id.
    pub item_id: String,
    /// Item code.
    pub item_code: String,
    /// Item name.
    pub item_name: String,
    /// Description.
    pub detailed_description: String,
    /// Synthesis instructions.
    pub synthesis_instructions: Vec<String>,
    /// Criteria.
    pub internal_evaluation_criteria: Vec<Criterion>,
    /// Applicability signals.
    pub applicability_signals: Vec<String>,
    /// Anti-signals.
    pub anti_signals: Vec<String>,
    /// Signature failures.
    pub signature_failures: Vec<Failure>,
}

/// A category.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    /// Code.
    pub code: String,
    /// Name.
    pub name: String,
    /// Seam.
    pub seam: String,
    /// Items.
    pub items: Vec<Item>,
}

/// A super-domain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuperDomain {
    /// Code.
    pub code: String,
    /// Name.
    pub name: String,
    /// Categories.
    pub categories: Vec<Category>,
}

/// A cross-cutting posture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Posture {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
    /// The invariant.
    pub posture: String,
}

/// A module law.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Law {
    /// Id.
    pub id: String,
    /// The law.
    pub law: String,
}

/// A role cut.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleFocus {
    /// The role.
    pub role: Role,
    /// Identity sentence.
    pub identity: String,
    /// Super-domain codes.
    pub super_domains: Vec<String>,
    /// Worldview ids.
    pub worldviews: Vec<String>,
}

/// A domain pack (`holden-forge.pack/1`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    /// Always [`PACK_SCHEMA`].
    pub schema: String,
    /// Pack name.
    pub pack_name: String,
    /// The domain line.
    pub domain: String,
    /// Ratification status.
    pub ratification_status: String,
    /// Priority order.
    pub priority_order: Vec<String>,
    /// Worldviews.
    pub worldviews: Vec<Worldview>,
    /// Super-domains.
    pub super_domains: Vec<SuperDomain>,
    /// Postures.
    pub cross_cutting_postures: Vec<Posture>,
    /// Module laws.
    pub module_laws: Vec<Law>,
    /// Role cuts.
    pub role_focus: Vec<RoleFocus>,
}

/// One constitution entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Tag.
    pub tag: String,
    /// Text.
    pub text: String,
    /// JSON pointer into the pack.
    pub trace: String,
}

/// One constitution section.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    /// Title.
    pub title: String,
    /// The opening sentence.
    pub intro: String,
    /// Entries.
    pub entries: Vec<Entry>,
}

/// An agent constitution emitted by the forge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constitution {
    /// The role.
    pub role: Role,
    /// The pack name.
    pub pack_name: String,
    /// `sha256:<hex>` of the pack file.
    pub pack_digest: String,
    /// Ratification status.
    pub ratification: String,
    /// The banner lines (each starting `> `).
    pub banner: Vec<String>,
    /// Sections, in order.
    pub sections: Vec<Section>,
}

/// A pack read from its file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedPack {
    /// The typed pack.
    pub pack: Pack,
    /// SHA-256 of the file bytes.
    pub digest: Sha256Digest,
}

/// A forge export: the pack and its constitutions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedForge {
    /// The pack.
    pub pack: LoadedPack,
    /// One constitution per role, in [`Role::ALL`] order.
    pub constitutions: Vec<Constitution>,
}

/// Why a pack or constitution was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackError {
    /// The file.
    pub file: PathBuf,
    /// What is wrong.
    pub why: String,
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "refused: {}: {}", self.file.display(), self.why)
    }
}

impl std::error::Error for PackError {}

fn refuse<T>(file: &Path, why: impl Into<String>) -> Result<T, PackError> {
    Err(PackError {
        file: file.to_path_buf(),
        why: why.into(),
    })
}

impl Pack {
    /// The pack in the forge's canonical layout.
    ///
    /// # Errors
    /// A JSON error (not expected for this model).
    pub fn to_canonical(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut v = serde_json::to_vec_pretty(self)?;
        v.push(b'\n');
        Ok(v)
    }
}

/// Read a pack file with the typed loader.
///
/// # Errors
/// [`PackError`] for unreadable, malformed, non-canonical or wrong-schema files.
pub fn load_pack_file(path: &Path) -> Result<LoadedPack, PackError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => return refuse(path, format!("cannot be read: {e}")),
    };
    let pack: Pack = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => return refuse(path, format!("not a typed pack: {e}")),
    };
    if pack.schema != PACK_SCHEMA {
        return refuse(
            path,
            format!("schema '{}' is not {PACK_SCHEMA}", pack.schema),
        );
    }
    match pack.to_canonical() {
        Ok(back) if back == bytes => {}
        Ok(_) => {
            return refuse(
                path,
                "re-serialising the typed pack does not reproduce the file (not lossless)",
            );
        }
        Err(e) => return refuse(path, format!("cannot be re-serialised: {e}")),
    }
    Ok(LoadedPack {
        digest: Sha256Digest::of(&bytes),
        pack,
    })
}

impl Constitution {
    /// The constitution in the forge's Form A text.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = format!(
            "# Agent constitution: {} / {}\n\n",
            self.pack_name,
            self.role.as_str()
        );
        for b in &self.banner {
            out.push_str(b);
            out.push('\n');
        }
        out.push('\n');
        out.push_str(&format!("- forge-constitution: {CONSTITUTION_FORMAT}\n"));
        out.push_str(&format!("- role: {}\n", self.role.as_str()));
        out.push_str(&format!("- pack: {}\n", self.pack_name));
        out.push_str(&format!("- pack_digest: {}\n", self.pack_digest));
        out.push_str(&format!("- ratification: {}\n", self.ratification));
        for (n, s) in self.sections.iter().enumerate() {
            out.push_str(&format!("\n## {}. {}\n\n{}\n\n", n + 1, s.title, s.intro));
            for e in &s.entries {
                out.push_str(&format!("- [{}] {} [trace: {}]\n", e.tag, e.text, e.trace));
            }
        }
        out
    }

    /// Parse Form A text; the text must re-render to itself exactly.
    ///
    /// # Errors
    /// A description of the first problem.
    pub fn parse(text: &str) -> Result<Self, String> {
        fn header<'a>(line: Option<&'a str>, key: &str) -> Result<&'a str, String> {
            line.and_then(|l| l.strip_prefix("- "))
                .and_then(|l| l.strip_prefix(key))
                .and_then(|l| l.strip_prefix(": "))
                .ok_or_else(|| format!("expected the header line '- {key}: ...'"))
        }
        let mut lines = text.lines().peekable();
        let title = lines
            .next()
            .and_then(|l| l.strip_prefix("# Agent constitution: "))
            .ok_or("missing the title line")?;
        let (title_pack, title_role) = title.rsplit_once(" / ").ok_or("malformed title line")?;
        if lines.next() != Some("") {
            return Err("expected a blank line after the title".into());
        }
        let mut banner = Vec::new();
        while let Some(l) = lines.next_if(|l| l.starts_with("> ")) {
            banner.push(l.to_owned());
        }
        if banner.is_empty() || lines.next() != Some("") {
            return Err("expected a banner and a blank line".into());
        }
        if header(lines.next(), "forge-constitution")? != CONSTITUTION_FORMAT {
            return Err("unsupported constitution format".into());
        }
        let role_name = header(lines.next(), "role")?;
        let role = Role::parse(role_name).ok_or_else(|| format!("unknown role '{role_name}'"))?;
        let pack_name = header(lines.next(), "pack")?.to_owned();
        if role_name != title_role || pack_name != title_pack {
            return Err("the title and the header lines differ".into());
        }
        let pack_digest = header(lines.next(), "pack_digest")?.to_owned();
        let ratification = header(lines.next(), "ratification")?.to_owned();
        let mut sections: Vec<Section> = Vec::new();
        let mut expect_intro = false;
        for line in lines {
            if let Some(h) = line.strip_prefix("## ") {
                let (num, title) = h.split_once(". ").ok_or("malformed section heading")?;
                if num != (sections.len() + 1).to_string() {
                    return Err(format!("section '{title}' is out of order"));
                }
                sections.push(Section {
                    title: title.to_owned(),
                    intro: String::new(),
                    entries: Vec::new(),
                });
                expect_intro = true;
            } else if line.is_empty() {
                continue;
            } else if expect_intro {
                let s = sections.last_mut().ok_or("text before the first section")?;
                line.clone_into(&mut s.intro);
                expect_intro = false;
            } else {
                let s = sections
                    .last_mut()
                    .ok_or("entry before the first section")?;
                let bad = || format!("malformed entry line: {line}");
                let rest = line.strip_prefix("- [").ok_or_else(bad)?;
                let (tag, rest) = rest.split_once("] ").ok_or_else(bad)?;
                let (body, trace) = rest.rsplit_once(" [trace: ").ok_or_else(bad)?;
                let trace = trace.strip_suffix(']').ok_or_else(bad)?;
                s.entries.push(Entry {
                    tag: tag.to_owned(),
                    text: body.to_owned(),
                    trace: trace.to_owned(),
                });
            }
        }
        let c = Self {
            role,
            pack_name,
            pack_digest,
            ratification,
            banner,
            sections,
        };
        if c.render() != text {
            return Err("the text does not re-render to itself (not lossless)".into());
        }
        Ok(c)
    }
}

/// The text a constitution entry must carry for the pack value at `trace`.
fn pack_value_at<'a>(pack: &'a serde_json::Value, trace: &str) -> Option<&'a str> {
    pack.pointer(trace).and_then(serde_json::Value::as_str)
}

/// The pack file of `pack_name` under a forge directory.
#[must_use]
pub fn pack_path(forge_dir: &Path, pack_name: &str) -> PathBuf {
    forge_dir.join("packs").join(pack_name).join("PACK.json")
}

/// The constitution file of `role` under a forge directory.
#[must_use]
pub fn constitution_path(forge_dir: &Path, pack_name: &str, role: Role) -> PathBuf {
    forge_dir
        .join("agents")
        .join(pack_name)
        .join(format!("{}.md", role.as_str()))
}

/// Load a forge export (or the module itself): the pack `pack_name` and its
/// four constitutions.
///
/// # Errors
/// [`PackError`] naming the first refused file.
pub fn load_forge(forge_dir: &Path, pack_name: &str) -> Result<LoadedForge, PackError> {
    let pack_file = pack_path(forge_dir, pack_name);
    let pack = load_pack_file(&pack_file)?;
    if pack.pack.pack_name != pack_name {
        return refuse(&pack_file, "pack_name differs from its directory");
    }
    let value: serde_json::Value = match serde_json::to_value(&pack.pack) {
        Ok(v) => v,
        Err(e) => return refuse(&pack_file, format!("cannot be re-read: {e}")),
    };
    let digest = format!("sha256:{}", pack.digest.to_hex());
    let mut constitutions = Vec::new();
    for role in Role::ALL {
        let path = constitution_path(forge_dir, pack_name, role);
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => return refuse(&path, format!("cannot be read: {e}")),
        };
        let c = match Constitution::parse(&text) {
            Ok(c) => c,
            Err(why) => return refuse(&path, why),
        };
        if c.role != role || c.pack_name != pack_name {
            return refuse(&path, "names another role or pack");
        }
        if c.pack_digest != digest {
            return refuse(
                &path,
                format!("pack_digest {} is not the pack's {digest}", c.pack_digest),
            );
        }
        if c.ratification != pack.pack.ratification_status {
            return refuse(&path, "ratification differs from the pack");
        }
        for s in &c.sections {
            for e in &s.entries {
                let Some(v) = pack_value_at(&value, &e.trace) else {
                    return refuse(&path, format!("trace {} points at no pack text", e.trace));
                };
                let checklist = format!("[ ] {v}");
                if e.text != v && e.text != checklist {
                    return refuse(
                        &path,
                        format!(
                            "entry [{}] differs from the pack value at {}",
                            e.tag, e.trace
                        ),
                    );
                }
            }
        }
        constitutions.push(c);
    }
    Ok(LoadedForge {
        pack,
        constitutions,
    })
}

/// A worldview as it reaches a candidate cognition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayWorldview {
    /// JSON pointer into the pack.
    pub trace: String,
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
}

/// A signature failure as it reaches a candidate cognition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayFailure {
    /// JSON pointer into the pack.
    pub trace: String,
    /// The failure.
    pub failure: String,
    /// Its guard.
    pub guard: String,
}

/// The domain overlay of a pack: the fields that change candidate cognitions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainOverlay {
    /// The pack name.
    pub pack_name: String,
    /// SHA-256 of the pack file.
    pub pack_digest: Sha256Digest,
    /// The domain line.
    pub domain: String,
    /// Practitioner worldviews, in pack order.
    pub worldviews: Vec<OverlayWorldview>,
    /// Every item's signature failures, in pack order.
    pub failures: Vec<OverlayFailure>,
}

impl DomainOverlay {
    /// The overlay of a loaded pack.
    #[must_use]
    pub fn from_pack(loaded: &LoadedPack) -> Self {
        let p = &loaded.pack;
        let worldviews = p
            .worldviews
            .iter()
            .enumerate()
            .map(|(i, w)| OverlayWorldview {
                trace: format!("/worldviews/{i}/name"),
                id: w.id.clone(),
                name: w.name.clone(),
            })
            .collect();
        let mut failures = Vec::new();
        for (si, sd) in p.super_domains.iter().enumerate() {
            for (ci, c) in sd.categories.iter().enumerate() {
                for (ii, item) in c.items.iter().enumerate() {
                    for (fi, f) in item.signature_failures.iter().enumerate() {
                        failures.push(OverlayFailure {
                            trace: format!(
                                "/super_domains/{si}/categories/{ci}/items/{ii}/signature_failures/{fi}/failure"
                            ),
                            failure: f.failure.clone(),
                            guard: f.guard.clone(),
                        });
                    }
                }
            }
        }
        Self {
            pack_name: p.pack_name.clone(),
            pack_digest: loaded.digest,
            domain: p.domain.clone(),
            worldviews,
            failures,
        }
    }
}
