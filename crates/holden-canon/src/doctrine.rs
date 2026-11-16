//! The typed doctrine: what a holdout is, its philosophical roots, and how
//! the roots generate situated cognitions (docs/ARCHITECTURE.md §2).
//!
//! Every claim the doctrine makes about the canon is a [`Citation`]: a file
//! under `canon/holden/`, the exact heading line of a section, and the
//! SHA-256 of that section's bytes. [`Doctrine::verify`] resolves each
//! citation against the committed canon and refuses any that do not match,
//! so the doctrine cannot drift from the text it is grounded in.
//!
//! Section bytes: from the first byte of the heading line up to (not
//! including) the next heading line of the same or a higher level (fewer or
//! equal `#`), or the end of the file. Lines inside fenced code blocks are
//! never headings. The heading line must occur exactly once.

use std::fmt;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::digest::Sha256Digest;

/// The spec file every root cites.
pub const SPEC: &str = "holdout_skill_spec_v2_3_0.md";

/// A citation into a committed canon file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    /// File name relative to the canon root (`canon/holden/`).
    pub file: String,
    /// The exact heading line, including its leading `#`s.
    pub heading: String,
    /// SHA-256 of the cited section's bytes.
    pub sha256: Sha256Digest,
}

/// The five philosophical roots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootId {
    /// Hospitality and the Other: ethics before ontology.
    Hospitality,
    /// Situated cognition: situations, not personas.
    SituatedCognition,
    /// The regenerative loop: the instrument regenerates from what it learns.
    RegenerativeLoop,
    /// Measured reliability: confidence is earned against known controls.
    MeasuredReliability,
    /// Structural independence: the builder never sees the holdout's how.
    StructuralIndependence,
}

impl RootId {
    /// Every root, in doctrine order.
    pub const ALL: [Self; 5] = [
        Self::Hospitality,
        Self::SituatedCognition,
        Self::RegenerativeLoop,
        Self::MeasuredReliability,
        Self::StructuralIndependence,
    ];

    /// The stable text id.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hospitality => "hospitality",
            Self::SituatedCognition => "situated_cognition",
            Self::RegenerativeLoop => "regenerative_loop",
            Self::MeasuredReliability => "measured_reliability",
            Self::StructuralIndependence => "structural_independence",
        }
    }
}

/// Who meets the product.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Constituency {
    /// A person.
    Human,
    /// An AI agent.
    Agent,
    /// The hand-off between a human and an agent, or between agents.
    Seam,
    /// The future participant who inherits the product (the unknown Other).
    Successor,
}

impl Constituency {
    /// Every constituency.
    pub const ALL: [Self; 4] = [Self::Human, Self::Agent, Self::Seam, Self::Successor];

    /// The stable text id.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Agent => "agent",
            Self::Seam => "seam",
            Self::Successor => "successor",
        }
    }
}

/// The circumstance axes that, crossed with constituencies, span the space
/// of situated cognitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircumstanceAxis {
    /// How much attention the participant can give.
    Attention,
    /// How hurried they are.
    TimePressure,
    /// Whether the encounter is broken off.
    Interruption,
    /// What they meet the product through.
    Device,
    /// What side-effect authority they hold.
    Permission,
    /// What they already know.
    PriorKnowledge,
    /// Whether the surrounding content is adversarial.
    Hostility,
    /// Who hands the work to whom.
    Handoff,
}

impl CircumstanceAxis {
    /// Every axis.
    pub const ALL: [Self; 8] = [
        Self::Attention,
        Self::TimePressure,
        Self::Interruption,
        Self::Device,
        Self::Permission,
        Self::PriorKnowledge,
        Self::Hostility,
        Self::Handoff,
    ];

    /// The stable text id.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Attention => "attention",
            Self::TimePressure => "time_pressure",
            Self::Interruption => "interruption",
            Self::Device => "device",
            Self::Permission => "permission",
            Self::PriorKnowledge => "prior_knowledge",
            Self::Hostility => "hostility",
            Self::Handoff => "handoff",
        }
    }

    /// The typed levels of this axis, in order.
    #[must_use]
    pub fn levels(self) -> Vec<AxisLevel> {
        match self {
            Self::Attention => [
                AttentionLevel::Focused,
                AttentionLevel::Divided,
                AttentionLevel::Fragmented,
            ]
            .into_iter()
            .map(AxisLevel::Attention)
            .collect(),
            Self::TimePressure => [
                TimePressureLevel::None,
                TimePressureLevel::Low,
                TimePressureLevel::Moderate,
                TimePressureLevel::High,
                TimePressureLevel::Extreme,
            ]
            .into_iter()
            .map(AxisLevel::TimePressure)
            .collect(),
            Self::Interruption => [
                InterruptionLevel::Uninterrupted,
                InterruptionLevel::InterruptedAndResumed,
                InterruptionLevel::InterruptedAndAbandoned,
            ]
            .into_iter()
            .map(AxisLevel::Interruption)
            .collect(),
            Self::Device => [
                DeviceLevel::Desktop,
                DeviceLevel::Mobile,
                DeviceLevel::AssistiveTechnology,
                DeviceLevel::Headless,
            ]
            .into_iter()
            .map(AxisLevel::Device)
            .collect(),
            Self::Permission => [
                PermissionLevel::ApprovalPolicy,
                PermissionLevel::AutoApproveBoundary,
                PermissionLevel::OperatorDelegation,
                PermissionLevel::NoAuthority,
            ]
            .into_iter()
            .map(AxisLevel::Permission)
            .collect(),
            Self::PriorKnowledge => [
                PriorKnowledgeLevel::None,
                PriorKnowledgeLevel::Basic,
                PriorKnowledgeLevel::Intermediate,
                PriorKnowledgeLevel::Expert,
            ]
            .into_iter()
            .map(AxisLevel::PriorKnowledge)
            .collect(),
            Self::Hostility => [
                HostilityLevel::Benign,
                HostilityLevel::Ambiguous,
                HostilityLevel::Adversarial,
            ]
            .into_iter()
            .map(AxisLevel::Hostility)
            .collect(),
            Self::Handoff => [
                HandoffLevel::None,
                HandoffLevel::HumanToAgent,
                HandoffLevel::AgentToHuman,
                HandoffLevel::AgentToAgent,
            ]
            .into_iter()
            .map(AxisLevel::Handoff)
            .collect(),
        }
    }
}

/// Attention levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionLevel {
    /// Full attention on the product.
    Focused,
    /// Attention shared with another task.
    Divided,
    /// Attention broken into fragments.
    Fragmented,
}

/// Time-pressure levels (spec Layer 4 `time_pressure`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimePressureLevel {
    /// No time pressure.
    None,
    /// Low.
    Low,
    /// Moderate.
    Moderate,
    /// High.
    High,
    /// Extreme.
    Extreme,
}

/// Interruption levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionLevel {
    /// The encounter runs to its end.
    Uninterrupted,
    /// Broken off, then resumed.
    InterruptedAndResumed,
    /// Broken off and never resumed.
    InterruptedAndAbandoned,
}

/// Device levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceLevel {
    /// A desktop screen and keyboard.
    Desktop,
    /// A small touch screen.
    Mobile,
    /// A screen reader or other assistive technology.
    AssistiveTechnology,
    /// No screen: schemas and text only (most agents).
    Headless,
}

/// Permission levels (spec Layer 9, Side-Effect Authority, entries 2 to 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    /// Acting under an approval policy.
    ApprovalPolicy,
    /// Inside a developer-configured auto-approve boundary.
    AutoApproveBoundary,
    /// Inside an operator's delegation scope.
    OperatorDelegation,
    /// No authority for side effects.
    NoAuthority,
}

/// Prior-knowledge levels (spec Layer 4 `prior_knowledge`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorKnowledgeLevel {
    /// None.
    None,
    /// Basic.
    Basic,
    /// Intermediate.
    Intermediate,
    /// Expert.
    Expert,
}

/// Hostility levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostilityLevel {
    /// Nothing around the encounter is adversarial.
    Benign,
    /// Content of unknown intent is present.
    Ambiguous,
    /// Content targets the participant's trust boundaries.
    Adversarial,
}

/// Handoff levels (spec Layer 4 `handoff_pattern`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffLevel {
    /// No hand-off.
    None,
    /// A human hands work to an agent.
    HumanToAgent,
    /// An agent hands work to a human.
    AgentToHuman,
    /// An agent hands work to another agent.
    AgentToAgent,
}

/// One axis at one typed level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxisLevel {
    /// Attention.
    Attention(AttentionLevel),
    /// Time pressure.
    TimePressure(TimePressureLevel),
    /// Interruption.
    Interruption(InterruptionLevel),
    /// Device.
    Device(DeviceLevel),
    /// Permission.
    Permission(PermissionLevel),
    /// Prior knowledge.
    PriorKnowledge(PriorKnowledgeLevel),
    /// Hostility.
    Hostility(HostilityLevel),
    /// Handoff.
    Handoff(HandoffLevel),
}

impl AxisLevel {
    /// The axis this level belongs to.
    #[must_use]
    pub const fn axis(&self) -> CircumstanceAxis {
        match self {
            Self::Attention(_) => CircumstanceAxis::Attention,
            Self::TimePressure(_) => CircumstanceAxis::TimePressure,
            Self::Interruption(_) => CircumstanceAxis::Interruption,
            Self::Device(_) => CircumstanceAxis::Device,
            Self::Permission(_) => CircumstanceAxis::Permission,
            Self::PriorKnowledge(_) => CircumstanceAxis::PriorKnowledge,
            Self::Hostility(_) => CircumstanceAxis::Hostility,
            Self::Handoff(_) => CircumstanceAxis::Handoff,
        }
    }

    /// The level's stable text id (snake case, without the axis).
    #[must_use]
    pub fn level_str(&self) -> String {
        let v = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);
        v.as_object()
            .and_then(|m| m.values().next())
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned()
    }
}

impl fmt::Display for AxisLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.axis().as_str(), self.level_str())
    }
}

/// A philosophical root, grounded in the canon.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Root {
    /// The root's id.
    pub id: RootId,
    /// Its name.
    pub name: String,
    /// What it says, in one sentence.
    pub summary: String,
    /// What it rules out for a participant: each candidate cognition this
    /// root generates carries these as `cannot_assume`.
    pub cannot_assume: Vec<String>,
    /// The canon sections it is grounded in (at least one).
    pub citations: Vec<Citation>,
}

/// A constituency with its canon grounding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstituencyDef {
    /// The constituency.
    pub id: Constituency,
    /// Who it is.
    pub description: String,
    /// The canon section it is grounded in.
    pub citation: Citation,
}

/// A circumstance axis with its typed levels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AxisDef {
    /// The axis.
    pub id: CircumstanceAxis,
    /// Its typed levels, in order.
    pub levels: Vec<AxisLevel>,
}

/// What a generator rule yields: the constituencies and circumstance axes a
/// root opens for situated cognitions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorRule {
    /// The generating root.
    pub root: RootId,
    /// Constituencies it yields.
    pub constituencies: Vec<Constituency>,
    /// Axes it yields.
    pub axes: Vec<CircumstanceAxis>,
}

/// A part of every plan that a canon doctrine generates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanPart {
    /// An absence probe (the Nothing Doctrine).
    AbsenceProbe,
    /// A permission pair (the trust axes).
    PermissionPair,
    /// A targeted broken variant (mutation controls).
    BrokenVariant,
}

/// A plan part and the doctrine that yields it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Derivation {
    /// The plan part.
    pub yields: PlanPart,
    /// Why, in one sentence.
    pub because: String,
    /// The canon section.
    pub citation: Citation,
}

/// The definition of a holdout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    /// The definition.
    pub text: String,
    /// The canon sections it is grounded in.
    pub citations: Vec<Citation>,
}

/// The typed doctrine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Doctrine {
    /// Doctrine format.
    pub format: String,
    /// The canon version the citations resolve against.
    pub canon_version: String,
    /// What a holdout is.
    pub definition: Definition,
    /// The philosophical roots.
    pub roots: Vec<Root>,
    /// The constituencies.
    pub constituencies: Vec<ConstituencyDef>,
    /// The circumstance axes.
    pub axes: Vec<AxisDef>,
    /// One generator rule per root.
    pub generators: Vec<GeneratorRule>,
    /// Plan parts every plan carries, and their doctrine.
    pub derivations: Vec<Derivation>,
}

/// Why a citation does not resolve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CitationError {
    /// The cited file cannot be read.
    FileUnreadable {
        /// The file.
        file: String,
    },
    /// The file name is not a plain name inside the canon root.
    BadFileName {
        /// The file.
        file: String,
    },
    /// The heading line does not occur in the file.
    HeadingAbsent {
        /// The file.
        file: String,
        /// The heading.
        heading: String,
    },
    /// The heading line occurs more than once.
    HeadingAmbiguous {
        /// The file.
        file: String,
        /// The heading.
        heading: String,
    },
    /// The section bytes hash to something else.
    HashDiffers {
        /// The file.
        file: String,
        /// The heading.
        heading: String,
        /// The recorded hash.
        recorded: Sha256Digest,
        /// The hash of the section bytes in the file.
        actual: Sha256Digest,
    },
}

impl fmt::Display for CitationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileUnreadable { file } => write!(f, "citation: {file} cannot be read"),
            Self::BadFileName { file } => {
                write!(
                    f,
                    "citation: {file} is not a plain file name in the canon root"
                )
            }
            Self::HeadingAbsent { file, heading } => {
                write!(f, "citation: {file}: heading '{heading}' not found")
            }
            Self::HeadingAmbiguous { file, heading } => {
                write!(
                    f,
                    "citation: {file}: heading '{heading}' occurs more than once"
                )
            }
            Self::HashDiffers {
                file,
                heading,
                recorded,
                actual,
            } => write!(
                f,
                "citation: {file}: section '{heading}' hashes to {actual}, not the recorded {recorded}"
            ),
        }
    }
}

impl std::error::Error for CitationError {}

fn heading_level(line: &[u8]) -> Option<usize> {
    let n = line.iter().take_while(|b| **b == b'#').count();
    (n > 0 && line.get(n) == Some(&b' ')).then_some(n)
}

/// The bytes of the section that `heading` opens in `text`.
///
/// # Errors
/// [`CitationError::HeadingAbsent`] or [`CitationError::HeadingAmbiguous`]
/// (with an empty file name; the caller names it).
pub fn section_bytes<'a>(text: &'a [u8], heading: &str) -> Result<&'a [u8], CitationError> {
    let absent = || CitationError::HeadingAbsent {
        file: String::new(),
        heading: heading.to_owned(),
    };
    let target_level = heading_level(heading.as_bytes()).ok_or_else(absent)?;
    // (start offset, level) of every heading line outside fenced code.
    let mut headings: Vec<(usize, usize, &[u8])> = Vec::new();
    let mut in_fence = false;
    let mut offset = 0;
    for line in text.split_inclusive(|b| *b == b'\n') {
        let bare = line.strip_suffix(b"\n").unwrap_or(line);
        let bare = bare.strip_suffix(b"\r").unwrap_or(bare);
        if bare.starts_with(b"```") {
            in_fence = !in_fence;
        } else if !in_fence && let Some(level) = heading_level(bare) {
            headings.push((offset, level, bare));
        }
        offset += line.len();
    }
    let mut found = headings
        .iter()
        .enumerate()
        .filter(|(_, (_, _, bare))| *bare == heading.as_bytes());
    let (index, (start, _, _)) = found.next().ok_or_else(absent)?;
    if found.next().is_some() {
        return Err(CitationError::HeadingAmbiguous {
            file: String::new(),
            heading: heading.to_owned(),
        });
    }
    let end = headings[index + 1..]
        .iter()
        .find(|(_, level, _)| *level <= target_level)
        .map_or(text.len(), |(o, _, _)| *o);
    Ok(&text[*start..end])
}

impl Citation {
    /// The SHA-256 of the cited section in `canon_root`, or why it cannot be
    /// computed.
    ///
    /// # Errors
    /// [`CitationError`] naming the file and heading.
    pub fn actual_hash(&self, canon_root: &Path) -> Result<Sha256Digest, CitationError> {
        let plain = !self.file.is_empty()
            && !self.file.contains('/')
            && !self.file.contains('\\')
            && self.file != "."
            && self.file != "..";
        if !plain {
            return Err(CitationError::BadFileName {
                file: self.file.clone(),
            });
        }
        let bytes =
            fs::read(canon_root.join(&self.file)).map_err(|_| CitationError::FileUnreadable {
                file: self.file.clone(),
            })?;
        let section = section_bytes(&bytes, &self.heading).map_err(|e| match e {
            CitationError::HeadingAbsent { heading, .. } => CitationError::HeadingAbsent {
                file: self.file.clone(),
                heading,
            },
            CitationError::HeadingAmbiguous { heading, .. } => CitationError::HeadingAmbiguous {
                file: self.file.clone(),
                heading,
            },
            other => other,
        })?;
        Ok(Sha256Digest::of(section))
    }

    /// Resolve this citation: the section exists once and hashes to the
    /// recorded digest.
    ///
    /// # Errors
    /// [`CitationError`] naming the file and heading.
    pub fn resolve(&self, canon_root: &Path) -> Result<(), CitationError> {
        let actual = self.actual_hash(canon_root)?;
        if actual == self.sha256 {
            Ok(())
        } else {
            Err(CitationError::HashDiffers {
                file: self.file.clone(),
                heading: self.heading.clone(),
                recorded: self.sha256,
                actual,
            })
        }
    }
}

fn cite(file: &str, heading: &str, sha256: &str) -> Citation {
    Citation {
        file: file.to_owned(),
        heading: heading.to_owned(),
        // A malformed literal becomes the all-zero digest, which no section
        // hashes to, so `verify` refuses it instead of panicking.
        sha256: sha256
            .parse()
            .unwrap_or(Sha256Digest::from_bytes([0u8; 32])),
    }
}

/// Heading lines cited by the doctrine.
mod headings {
    pub const NAME: &str = "### 0.1 The Name";
    pub const CORE_QUESTION: &str = "### 0.3 The Core Question";
    pub const FRIENDSHIP: &str = "### 1.0 The Friendship Principle";
    pub const TESTING_PARADOX: &str = "### 1.1 The Testing Paradox";
    pub const SITUATEDNESS: &str = "### 1.3 Situatedness and Product-Class Configuration";
    pub const SELF_DOUBT: &str = "### 1.5 The Self-Doubt Commitment";
    pub const NOTHING: &str = "### 1.6 The Nothing Doctrine";
    pub const DUAL_CONSTITUENCY: &str = "### 1.7 The Dual-Constituency Commitment";
    pub const ENCOUNTER_PROFILES: &str = "### Layer 4 — Encounter Profile Multiplexing";
    pub const TRUST_MODEL: &str = "#### The three-axis trust model";
    pub const SELF_EVAL_MECHANISMS: &str = "### 5.1 Mechanisms";
    pub const LEARNING_LOOP: &str = "## Section 8: THE LEARNING LOOP";
    pub const AIR_GAP: &str = "### 9.6 Air Gap with Building Agents";
}

/// SHA-256 of each cited section of the committed canon (`canon/holden/`,
/// canon `skill-v2.3.0 adapter-v0.2.0`).
mod hashes {
    pub const NAME: &str = "26793df9269fc82f1b1ed4e53aaafefb8d2c519b93a4ee7f972c9ee768a403e5";
    pub const CORE_QUESTION: &str =
        "6ba3c7ded3e13f1e297ccfe1852e20939f1e62403b8736bc78f701c3bb0d0dd4";
    pub const FRIENDSHIP: &str = "947f05828a5989db7b765c62ae268917f0cfa0e7f359715a9fed940d7a453b4b";
    pub const TESTING_PARADOX: &str =
        "9afb47db24f97a177837683d60383f1ce875ddd44dabdc87b205615e26368725";
    pub const SITUATEDNESS: &str =
        "ed3b600b497835ab4a20ab669ac4d4102120d73d29685fda32b70d4968c33fff";
    pub const SELF_DOUBT: &str = "2b7b652665941c6ce0e832fe5a5802148e2ecb2a570be688307ec3ed69cac000";
    pub const NOTHING: &str = "be7363606290211575a19b5f43e448bd6fe64cff6722cb9289a717cd539dcc09";
    pub const DUAL_CONSTITUENCY: &str =
        "e745618ddff1ed7d510d28ccd9ccbd794ead3af7199a2bdaeeae579053ed347d";
    pub const ENCOUNTER_PROFILES: &str =
        "387258fda138def0a090f2e9d657dfcceb9f3deb46b92cdc5dbc300054047f69";
    pub const TRUST_MODEL: &str =
        "00df440fe42b08c636fd7228ee0bdc66161e555368b95de16ca481bdbd802624";
    pub const SELF_EVAL_MECHANISMS: &str =
        "3c9eb359089879e6fe1123f032111b920e77fc3a5b44a9ad03c458f89f94ab01";
    pub const LEARNING_LOOP: &str =
        "732889a3edfe6e200c71e46d99f4fad72bb7bae2da0ebdceebf7d083d2de9ec7";
    pub const AIR_GAP: &str = "0152ea16d5c41236273ff0084d602d3061ef8b6567b372b08ed6820e8e753122";
}

fn spec(heading: &str, sha256: &str) -> Citation {
    cite(SPEC, heading, sha256)
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

impl Doctrine {
    /// The holden doctrine, grounded in the committed canon.
    #[must_use]
    pub fn holden() -> Self {
        use CircumstanceAxis as A;
        use Constituency as C;
        Self {
            format: "holden.doctrine/1".to_owned(),
            canon_version: "skill-v2.3.0 adapter-v0.2.0".to_owned(),
            definition: Definition {
                text: "A holdout is a test the builder never sees. It holds space open for the \
                       one who was not assumed (human, agent, seam or successor), asks what \
                       happens when that participant meets the product, and compiles the \
                       answer into evidence: hospitality compiled into evidence. The why is \
                       published; the how is sealed."
                    .to_owned(),
                citations: vec![
                    spec(headings::NAME, hashes::NAME),
                    spec(headings::CORE_QUESTION, hashes::CORE_QUESTION),
                    spec(headings::AIR_GAP, hashes::AIR_GAP),
                ],
            },
            roots: vec![
                Root {
                    id: RootId::Hospitality,
                    name: "Hospitality and the Other".to_owned(),
                    summary: "Ethics is prior to ontology: before a system classifies a valid \
                              user it is already in a relationship with whoever meets it, and \
                              owes the unassumed Other accommodation, legibility and \
                              non-humiliation."
                        .to_owned(),
                    cannot_assume: strings(&[
                        "that the participant shares the builder's defaults",
                        "that the participant has the body, language, device or permissions the builder modelled",
                    ]),
                    citations: vec![
                        spec(headings::NAME, hashes::NAME),
                        spec(headings::FRIENDSHIP, hashes::FRIENDSHIP),
                    ],
                },
                Root {
                    id: RootId::SituatedCognition,
                    name: "Situated cognition".to_owned(),
                    summary: "There is no context-free test: a holdout is set inside a concrete \
                              circumstance (attention, time pressure, device, prior knowledge), \
                              never a demographic caricature."
                        .to_owned(),
                    cannot_assume: strings(&[
                        "that the participant meets the product in the builder's own circumstances",
                    ]),
                    citations: vec![
                        spec(headings::SITUATEDNESS, hashes::SITUATEDNESS),
                        spec(headings::ENCOUNTER_PROFILES, hashes::ENCOUNTER_PROFILES),
                    ],
                },
                Root {
                    id: RootId::RegenerativeLoop,
                    name: "The regenerative loop".to_owned(),
                    summary: "The instrument regenerates itself from what it learns: post-fix \
                              verification, schema drift, model changes and missed mutations \
                              feed back into its profiles and canaries, and it detects its own \
                              obsolescence."
                        .to_owned(),
                    cannot_assume: strings(&[
                        "that the participant was present when today's rules were chosen",
                        "that what the participant learned earlier is still current",
                    ]),
                    citations: vec![spec(headings::LEARNING_LOOP, hashes::LEARNING_LOOP)],
                },
                Root {
                    id: RootId::MeasuredReliability,
                    name: "Measured reliability".to_owned(),
                    summary: "The evaluator's own models are fallible: confidence is explicit, \
                              negative controls and seeded mutations exist, and authority is \
                              earned by measured error rates."
                        .to_owned(),
                    cannot_assume: strings(&[
                        "that a passing check proves the promise was kept",
                    ]),
                    citations: vec![
                        spec(headings::SELF_DOUBT, hashes::SELF_DOUBT),
                        spec(headings::SELF_EVAL_MECHANISMS, hashes::SELF_EVAL_MECHANISMS),
                    ],
                },
                Root {
                    id: RootId::StructuralIndependence,
                    name: "Structural independence".to_owned(),
                    summary: "Encounter execution is separated from structural diagnosis, and \
                              only typed artifacts cross the air gap to building agents: the \
                              builder never sees the holdout's reasoning."
                        .to_owned(),
                    cannot_assume: strings(&[
                        "that the party who built the boundary will be the one who crosses it",
                        "that authority granted for one action extends to the next",
                    ]),
                    citations: vec![
                        spec(headings::AIR_GAP, hashes::AIR_GAP),
                        spec(headings::TESTING_PARADOX, hashes::TESTING_PARADOX),
                    ],
                },
            ],
            constituencies: vec![
                ConstituencyDef {
                    id: C::Human,
                    description: "a person whose constraints or values sit outside the builder's defaults".to_owned(),
                    citation: spec(headings::NAME, hashes::NAME),
                },
                ConstituencyDef {
                    id: C::Agent,
                    description: "an AI agent that consumes schemas and text rather than pixels and touch".to_owned(),
                    citation: spec(headings::NAME, hashes::NAME),
                },
                ConstituencyDef {
                    id: C::Seam,
                    description: "the hand-off where humans and agents pass work to one another".to_owned(),
                    citation: spec(headings::DUAL_CONSTITUENCY, hashes::DUAL_CONSTITUENCY),
                },
                ConstituencyDef {
                    id: C::Successor,
                    description: "the future participant who inherits today's interfaces without having been present when their rules were chosen".to_owned(),
                    citation: spec(headings::NAME, hashes::NAME),
                },
            ],
            axes: CircumstanceAxis::ALL
                .into_iter()
                .map(|id| AxisDef {
                    id,
                    levels: id.levels(),
                })
                .collect(),
            generators: vec![
                GeneratorRule {
                    root: RootId::Hospitality,
                    constituencies: vec![C::Human, C::Agent, C::Successor],
                    axes: vec![A::Device, A::PriorKnowledge, A::Attention],
                },
                GeneratorRule {
                    root: RootId::SituatedCognition,
                    constituencies: vec![C::Human, C::Agent],
                    axes: vec![
                        A::Attention,
                        A::TimePressure,
                        A::Interruption,
                        A::Device,
                        A::PriorKnowledge,
                    ],
                },
                GeneratorRule {
                    root: RootId::RegenerativeLoop,
                    constituencies: vec![C::Successor, C::Agent],
                    axes: vec![A::Handoff, A::PriorKnowledge, A::Interruption],
                },
                GeneratorRule {
                    root: RootId::MeasuredReliability,
                    constituencies: vec![C::Agent, C::Seam],
                    axes: vec![A::Hostility, A::TimePressure],
                },
                GeneratorRule {
                    root: RootId::StructuralIndependence,
                    constituencies: vec![C::Seam, C::Agent],
                    axes: vec![A::Permission, A::Handoff, A::Hostility],
                },
            ],
            derivations: vec![
                Derivation {
                    yields: PlanPart::AbsenceProbe,
                    because: "the Nothing Doctrine: \"nothing happened\" is not empty data, so \
                              every plan probes what should exist but does not"
                        .to_owned(),
                    citation: spec(headings::NOTHING, hashes::NOTHING),
                },
                Derivation {
                    yields: PlanPart::PermissionPair,
                    because: "the trust axes are probed independently, so every plan pairs a \
                              permitted action with the same action refused"
                        .to_owned(),
                    citation: spec(headings::TRUST_MODEL, hashes::TRUST_MODEL),
                },
                Derivation {
                    yields: PlanPart::BrokenVariant,
                    because: "false negatives are detected with seeded mutations, so every plan \
                              carries a targeted broken variant its checks must catch"
                        .to_owned(),
                    citation: spec(headings::SELF_EVAL_MECHANISMS, hashes::SELF_EVAL_MECHANISMS),
                },
            ],
        }
    }

    /// Every citation in the doctrine, with where it sits.
    #[must_use]
    pub fn citations(&self) -> Vec<(String, &Citation)> {
        let mut out: Vec<(String, &Citation)> = Vec::new();
        for c in &self.definition.citations {
            out.push(("definition".to_owned(), c));
        }
        for r in &self.roots {
            for c in &r.citations {
                out.push((format!("root {}", r.id.as_str()), c));
            }
        }
        for k in &self.constituencies {
            out.push((format!("constituency {}", k.id.as_str()), &k.citation));
        }
        for d in &self.derivations {
            out.push((format!("derivation {:?}", d.yields), &d.citation));
        }
        out
    }

    /// The root with this id.
    #[must_use]
    pub fn root(&self, id: RootId) -> Option<&Root> {
        self.roots.iter().find(|r| r.id == id)
    }

    /// The generator rule of this root.
    #[must_use]
    pub fn generator(&self, id: RootId) -> Option<&GeneratorRule> {
        self.generators.iter().find(|g| g.root == id)
    }

    /// Resolve every citation against `canon_root`. Returns every failure,
    /// each named by where it sits in the doctrine.
    ///
    /// # Errors
    /// The list of unresolved citations.
    pub fn verify(&self, canon_root: &Path) -> Result<usize, Vec<(String, CitationError)>> {
        let all = self.citations();
        let failures: Vec<(String, CitationError)> = all
            .iter()
            .filter_map(|(at, c)| c.resolve(canon_root).err().map(|e| (at.clone(), e)))
            .collect();
        if failures.is_empty() {
            Ok(all.len())
        } else {
            Err(failures)
        }
    }

    /// The doctrine as pretty JSON with a trailing newline.
    ///
    /// # Errors
    /// A JSON error (not expected for this model).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut s = serde_json::to_string_pretty(self)?;
        s.push('\n');
        Ok(s)
    }
}
