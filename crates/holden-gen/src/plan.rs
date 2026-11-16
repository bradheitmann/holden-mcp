//! The generated plan (docs/ARCHITECTURE.md §3.3; slice-3 P4 and P5).
//!
//! [`generate`] turns an approved promise and cognition (a
//! [`HostableCognition`]) and a keyed lens draw into a typed [`GenPlan`]:
//!
//! - encounter steps written from inside the cognition: one for the
//!   cognition's own goal, one per drawn lens concept;
//! - a deterministic check per step, whose expectation is always an
//!   approved promise quoted verbatim, never lens text;
//! - an absence probe (the Nothing Doctrine);
//! - a permission pair (the trust axes);
//! - a targeted broken variant (mutation control) its checks must catch.
//!
//! A lens concept that implies a property no approved promise covers yields
//! a [`NeedsOwnerWords`] item for the owner and no check. Lens concept data
//! can shape how a step is approached; it can never become an expectation.
//!
//! Every plan carries its provenance: promise id → cognition id → root ids
//! → lens concept ids, with the recorded draw. Generation is deterministic
//! given its inputs and the draw. The plan is sealed with `holden-seal`
//! ([`seal_plan`]); only the [`PublicCard`] (the why) is published.

use std::fmt;

use holden_canon::doctrine::{AxisLevel, PermissionLevel, PlanPart, RootId};
use holden_canon::{Doctrine, Sha256Digest};
use holden_cognition::{ContentId, HostableCognition, Promise};
use holden_seal::{
    PlanCommit, PlanId, SealKey, SealPlanError, SealableDocument, SealedPlan, UnsealError,
    plan_commit, seal::seal_document, seal::unseal_document,
};
use serde::{Deserialize, Serialize};

use crate::draw::{DrawCorpus, LensDraw};

/// A deterministic check at the product boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    /// `check-<n>`, unique within the plan.
    pub id: String,
    /// The approved promise this check enforces.
    pub promise: ContentId,
    /// What the evaluator observes, at the boundary only.
    pub observe: String,
    /// What must hold: the approved promise, quoted verbatim.
    pub expect: String,
}

/// One encounter step, written from inside the cognition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterStep {
    /// The lens concept that shaped the step, if any.
    pub lens: Option<ContentId>,
    /// What the participant does.
    pub action: String,
    /// The step's check.
    pub check: Check,
}

/// The absence probe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbsenceProbe {
    /// What is withheld.
    pub action: String,
    /// Its check.
    pub check: Check,
}

/// The permission pair: the same action with and without authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionPair {
    /// The authority the participant holds in the permitted half.
    pub granted: PermissionLevel,
    /// The permitted half.
    pub permitted: EncounterStep,
    /// The refused half.
    pub refused: EncounterStep,
}

/// The targeted broken variant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrokenVariant {
    /// The mutation applied to a known-good build.
    pub mutation: String,
    /// The check that must fail on it.
    pub must_fail: String,
}

/// The provenance chain of a plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// The approved promise.
    pub promise: ContentId,
    /// SHA-256 of the promise's source file.
    pub source: Sha256Digest,
    /// The approved cognition.
    pub cognition: ContentId,
    /// The root(s) that produced the cognition.
    pub roots: Vec<RootId>,
    /// The lens concepts that shaped the plan.
    pub lens_concepts: Vec<ContentId>,
    /// The keyed draw that chose them.
    pub draw: LensDraw,
    /// The owner key the approval verified under, hex.
    pub approved_by: String,
}

/// A generated holdout plan (the sealed how).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenPlan {
    /// `H(canonical bytes)` of the fields below.
    pub id: ContentId,
    /// Plan format.
    pub format: String,
    /// The provenance chain.
    pub provenance: Provenance,
    /// The situation, from inside the cognition.
    pub situation: String,
    /// Encounter steps, each with its check.
    pub steps: Vec<EncounterStep>,
    /// The absence probe.
    pub absence_probe: AbsenceProbe,
    /// The permission pair.
    pub permission_pair: PermissionPair,
    /// The targeted broken variant.
    pub broken_variant: BrokenVariant,
}

/// A lens-implied property that no approved promise covers: it goes to the
/// owner for words, never to a check.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedsOwnerWords {
    /// The lens concept.
    pub concept: ContentId,
    /// The property it implies.
    pub property: String,
    /// The promise the plan was generated for.
    pub promise: ContentId,
    /// The cognition the plan sits in.
    pub cognition: ContentId,
}

/// A generated plan and its owner-words items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generated {
    /// The plan.
    pub plan: GenPlan,
    /// Properties that need the owner's words.
    pub needs_owner_words: Vec<NeedsOwnerWords>,
}

/// Why generation was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenError {
    /// The draw is for another promise or cognition.
    DrawForOther,
    /// The draw names a concept the corpus does not hold.
    UnknownConcept(ContentId),
    /// The doctrine lacks a plan part's derivation or a root.
    DoctrineIncomplete(&'static str),
}

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DrawForOther => {
                f.write_str("refused: the draw is for another promise or cognition")
            }
            Self::UnknownConcept(id) => write!(f, "refused: the draw names unknown concept {id}"),
            Self::DoctrineIncomplete(what) => write!(f, "refused: the doctrine lacks {what}"),
        }
    }
}

impl std::error::Error for GenError {}

/// Whether `promise` covers a property: it contains every term (ASCII case
/// folded). Only approved promises are ever asked.
fn covers(promise: &Promise, terms: &[String]) -> bool {
    let text = promise.text.to_ascii_lowercase();
    terms.iter().all(|t| text.contains(&t.to_ascii_lowercase()))
}

fn promise_check(n: &mut usize, promise: &Promise, observe: String, prefix: &str) -> Check {
    *n += 1;
    Check {
        id: format!("check-{n}"),
        promise: promise.id,
        observe,
        expect: format!("{prefix}\"{}\"", promise.text),
    }
}

fn level_of(cognition: &holden_cognition::SituatedCognition) -> PermissionLevel {
    cognition
        .circumstance
        .iter()
        .find_map(|l| match l {
            AxisLevel::Permission(p) => Some(*p),
            _ => None,
        })
        .unwrap_or(PermissionLevel::OperatorDelegation)
}

#[derive(Serialize)]
struct Body<'a> {
    format: &'a str,
    provenance: &'a Provenance,
    situation: &'a str,
    steps: &'a [EncounterStep],
    absence_probe: &'a AbsenceProbe,
    permission_pair: &'a PermissionPair,
    broken_variant: &'a BrokenVariant,
}

impl GenPlan {
    /// The id this plan's content hashes to.
    #[must_use]
    pub fn computed_id(&self) -> ContentId {
        let body = Body {
            format: &self.format,
            provenance: &self.provenance,
            situation: &self.situation,
            steps: &self.steps,
            absence_probe: &self.absence_probe,
            permission_pair: &self.permission_pair,
            broken_variant: &self.broken_variant,
        };
        let mut bytes = b"holden.gen-plan/1\n".to_vec();
        bytes.extend(serde_json::to_vec(&body).unwrap_or_default());
        Sha256Digest::of(&bytes)
    }

    /// Every check in the plan.
    #[must_use]
    pub fn checks(&self) -> Vec<&Check> {
        let mut out: Vec<&Check> = self.steps.iter().map(|s| &s.check).collect();
        out.push(&self.absence_probe.check);
        out.push(&self.permission_pair.permitted.check);
        out.push(&self.permission_pair.refused.check);
        out
    }
}

impl SealableDocument for GenPlan {
    fn plan_id(&self) -> PlanId {
        PlanId(*self.id.as_bytes())
    }
}

/// Generate the plan for `hosted` from `draw`. `approved` holds every
/// approved promise that may cover a lens-implied property.
///
/// # Errors
/// [`GenError`] when the draw does not belong to `hosted` or names an
/// unknown concept, or the doctrine is incomplete.
pub fn generate(
    doctrine: &Doctrine,
    hosted: &HostableCognition,
    approved: &[&Promise],
    corpus: &DrawCorpus,
    draw: &LensDraw,
) -> Result<Generated, GenError> {
    let promise = hosted.promise();
    let cognition = hosted.cognition();
    if draw.inputs.promise != promise.id || draw.inputs.cognition != cognition.id {
        return Err(GenError::DrawForOther);
    }
    for part in [
        PlanPart::AbsenceProbe,
        PlanPart::PermissionPair,
        PlanPart::BrokenVariant,
    ] {
        if !doctrine.derivations.iter().any(|d| d.yields == part) {
            return Err(GenError::DoctrineIncomplete("a plan-part derivation"));
        }
    }
    for r in &cognition.roots {
        if doctrine.root(*r).is_none() {
            return Err(GenError::DoctrineIncomplete("a root of the cognition"));
        }
    }

    let situation = format!(
        "You are {}. You are under {}. You cannot assume {}. You are trying {}.",
        cognition.role,
        cognition.pressure,
        cognition.cannot_assume.join("; nor "),
        cognition.goal
    );
    let mut n = 0usize;
    let mut steps = vec![EncounterStep {
        lens: None,
        action: format!(
            "From inside this situation, meet the product at its boundary and try {}",
            cognition.goal
        ),
        check: promise_check(
            &mut n,
            promise,
            "the product's boundary output after the attempt".to_owned(),
            "the owner's promise holds as written: ",
        ),
    }];
    let mut needs_owner_words = Vec::new();
    for concept_id in &draw.picks {
        let concept = corpus
            .concept(concept_id)
            .ok_or(GenError::UnknownConcept(*concept_id))?;
        let entry = corpus
            .entry(concept_id)
            .ok_or(GenError::UnknownConcept(*concept_id))?;
        let covering = std::iter::once(promise)
            .chain(approved.iter().copied())
            .find(|p| covers(p, &entry.implies.terms));
        let check_promise = match covering {
            Some(p) => p,
            None => {
                needs_owner_words.push(NeedsOwnerWords {
                    concept: *concept_id,
                    property: entry.implies.property.clone(),
                    promise: promise.id,
                    cognition: cognition.id,
                });
                promise
            }
        };
        steps.push(EncounterStep {
            lens: Some(*concept_id),
            action: format!(
                "Still inside the situation, approach it through the lens '{}': {}",
                concept.name, concept.question
            ),
            check: promise_check(
                &mut n,
                check_promise,
                "the product's boundary output after this approach".to_owned(),
                "the owner's promise holds as written: ",
            ),
        });
    }

    let absence_probe = AbsenceProbe {
        action: "Withhold what the promise depends on and watch for silence: a void state, an \
                 inert control, a missing explanation or a blocked hand-off"
            .to_owned(),
        check: promise_check(
            &mut n,
            promise,
            "whether the boundary reports the absence or stays silent".to_owned(),
            "the absence is reported, not silent, and the owner's promise still holds: ",
        ),
    };
    let granted = level_of(cognition);
    let permission_pair = PermissionPair {
        granted,
        permitted: EncounterStep {
            lens: None,
            action: format!(
                "Holding {granted:?} authority, take the side effect the promise names"
            ),
            check: promise_check(
                &mut n,
                promise,
                "the boundary output with authority granted".to_owned(),
                "the permitted action succeeds and the owner's promise holds: ",
            ),
        },
        refused: EncounterStep {
            lens: None,
            action: "Holding no authority, attempt the same side effect".to_owned(),
            check: promise_check(
                &mut n,
                promise,
                "the boundary output with no authority".to_owned(),
                "the action is refused visibly, nothing irreversible happens, and the owner's \
                 promise still holds: ",
            ),
        },
    };
    let first_mac = draw.macs.first().and_then(|m| m.get(..2)).unwrap_or("00");
    let target = usize::from(u8::from_str_radix(first_mac, 16).unwrap_or(0)) % steps.len();
    let target_check = steps[target].check.clone();
    let broken_variant = BrokenVariant {
        mutation: format!(
            "A known-good build changed so that, after step {}, this no longer holds: \"{}\"",
            target + 1,
            promise.text
        ),
        must_fail: target_check.id,
    };

    let mut plan = GenPlan {
        id: Sha256Digest::from_bytes([0; 32]),
        format: "holden.gen-plan/1".to_owned(),
        provenance: Provenance {
            promise: promise.id,
            source: promise.source,
            cognition: cognition.id,
            roots: cognition.roots.clone(),
            lens_concepts: draw.picks.clone(),
            draw: draw.clone(),
            approved_by: hex::encode(hosted.owner().0),
        },
        situation,
        steps,
        absence_probe,
        permission_pair,
        broken_variant,
    };
    plan.id = plan.computed_id();
    Ok(Generated {
        plan,
        needs_owner_words,
    })
}

/// What is published about a sealed plan: the why, never the how. It names
/// the promise, the cognition and the roots, and commits to the sealed bytes.
/// It never names the lens concepts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicCard {
    /// The plan id.
    pub plan: ContentId,
    /// `plan_commit` of the sealed bytes, hex.
    pub plan_commit: String,
    /// The promise.
    pub promise: ContentId,
    /// The cognition.
    pub cognition: ContentId,
    /// The roots.
    pub roots: Vec<RootId>,
}

/// Seal `plan` under `key`. Returns the sealed plan and its public card.
///
/// # Errors
/// [`SealPlanError`] when sealing fails.
pub fn seal_plan(key: &SealKey, plan: &GenPlan) -> Result<(SealedPlan, PublicCard), SealPlanError> {
    let sealed = seal_document(key, plan)?;
    let commit = plan_commit(&sealed);
    let card = PublicCard {
        plan: plan.id,
        plan_commit: hex::encode(commit.0),
        promise: plan.provenance.promise,
        cognition: plan.provenance.cognition,
        roots: plan.provenance.roots.clone(),
    };
    Ok((sealed, card))
}

/// Unseal a generated plan against its recorded commit.
///
/// # Errors
/// [`UnsealError`] naming the first failed check.
pub fn unseal_plan(
    key: &SealKey,
    sealed: &SealedPlan,
    commit: &PlanCommit,
) -> Result<GenPlan, UnsealError> {
    unseal_document(key, sealed, commit)
}
