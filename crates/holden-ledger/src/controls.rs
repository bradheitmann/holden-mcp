//! Sealed controls, run through the real evaluator (slice-5 P2).
//!
//! For a holdout version (a plan), [`catalog`] lists the controls of the
//! notes fixture product, each a real commit of the fixture repository:
//!
//! | Kind | Controls | Expected |
//! |---|---|---|
//! | known good | `keeps`, `keeps-alt` (another correct implementation), `reconfig` (public config change only) | good |
//! | fixture mutant | `breaks`, `refuses`, `torn-write` | breaks the saving promise |
//! | do nothing | `import-noop`, `list-noop`, `export-noop`, `delete-noop` | breaks its probe's promise |
//! | plan targeted | the do-nothing variant of the plan's `broken_variant.must_fail` check's promise | that check must fail |
//! | never again | `escapes` and `sneaks` (confinement), `hangs` (limits), a commit carrying the plan's canary (canary scan) | fails with the rule's reason |
//!
//! [`run_controls`] evaluates each control end to end (MCP reveal, build,
//! confined probes, rules, signed record, server acceptance) and appends a
//! [`ControlRecord`] read from the signed record to the ledger. Nothing is
//! counted without a run.

use std::fmt;

use holden_eval::binding::{Binding, Probe};
use holden_eval::canary;
use holden_eval::fixture::{BASE, FixtureRepo, do_nothing_variant};
use holden_eval::{EvalError, EvalRequest, Evaluator, FailReason, McpClient, VerdictRecord};
use holden_gen::GenPlan;
use holden_seal::{EvaluatorKey, PlanCommit};
use serde::{Deserialize, Serialize};

use crate::records::{ControlKind, ControlRecord, Expect, Target, kind, promise_of};
use crate::store::{Entry, Ledger, LedgerError};

/// One control: a commit of the fixture repository and what it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Control {
    /// Its name.
    pub name: String,
    /// The fixture variant (or synthesized commit) it builds.
    pub variant: String,
    /// The commit.
    pub commit: String,
    /// Its kind.
    pub kind: ControlKind,
    /// Its expectation.
    pub expect: Expect,
    /// For the plan's targeted variant, the check that must fail.
    pub must_fail: Option<String>,
}

/// Why controls could not run.
#[derive(Debug)]
pub enum ControlError {
    /// The evaluation failed.
    Eval(EvalError),
    /// The server refused the control's verdict.
    Refused(String),
    /// The ledger refused.
    Ledger(LedgerError),
    /// The binding has no promise for a probe.
    Unbound(Probe),
}

impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eval(e) => write!(f, "control evaluation: {e}"),
            Self::Refused(r) => write!(f, "the server refused a control verdict: {r}"),
            Self::Ledger(e) => write!(f, "{e}"),
            Self::Unbound(p) => write!(f, "no promise is bound to probe {}", p.as_str()),
        }
    }
}

impl std::error::Error for ControlError {}

impl From<EvalError> for ControlError {
    fn from(e: EvalError) -> Self {
        Self::Eval(e)
    }
}

impl From<LedgerError> for ControlError {
    fn from(e: LedgerError) -> Self {
        Self::Ledger(e)
    }
}

fn promise_target(probe: Probe) -> Result<Target, ControlError> {
    promise_of(probe)
        .map(|t| Target::Promise { text: t.to_owned() })
        .ok_or(ControlError::Unbound(probe))
}

/// The file the canary control commits on top of the base.
pub const CANARY_FILE: &str = "src/leaked-plan.txt";

/// Every control of the notes fixture for `plan`.
///
/// # Errors
/// [`ControlError`] when a variant is missing from the repository or the
/// canary commit cannot be made.
pub fn catalog(
    repo: &FixtureRepo,
    plan: &GenPlan,
    binding: &Binding,
) -> Result<Vec<Control>, ControlError> {
    let mut out = Vec::new();
    let mut add = |name: &str, variant: &str, commit: String, kind, expect, must_fail| {
        out.push(Control {
            name: name.to_owned(),
            variant: variant.to_owned(),
            commit,
            kind,
            expect,
            must_fail,
        });
    };
    for v in [BASE, "keeps-alt", "reconfig"] {
        add(
            v,
            v,
            repo.commit(v)?,
            ControlKind::KnownGood,
            Expect::Good,
            None,
        );
    }
    let save = promise_target(Probe::InterruptedSave)?;
    for v in ["breaks", "refuses", "torn-write"] {
        add(
            v,
            v,
            repo.commit(v)?,
            ControlKind::FixtureMutant,
            Expect::Broken {
                target: save.clone(),
            },
            None,
        );
    }
    // The plan's targeted broken variant: the check that must fail, and the
    // probe its promise is observed by.
    let checks = binding.bind(plan);
    let targeted = checks
        .iter()
        .find(|c| c.id == plan.broken_variant.must_fail)
        .and_then(|c| c.probe);
    for p in Probe::ALL {
        let v = do_nothing_variant(p);
        let is_target = targeted == Some(p);
        add(
            v,
            v,
            repo.commit(v)?,
            if is_target {
                ControlKind::PlanTargeted
            } else {
                ControlKind::DoNothing
            },
            Expect::Broken {
                target: promise_target(p)?,
            },
            is_target.then(|| plan.broken_variant.must_fail.clone()),
        );
    }
    let confinement = Target::Reason {
        reason: FailReason::ConfinementViolation,
    };
    for v in ["escapes", "sneaks"] {
        add(
            v,
            v,
            repo.commit(v)?,
            ControlKind::NeverAgain,
            Expect::Broken {
                target: confinement.clone(),
            },
            None,
        );
    }
    add(
        "hangs",
        "hangs",
        repo.commit("hangs")?,
        ControlKind::NeverAgain,
        Expect::Broken {
            target: Target::Limit,
        },
        None,
    );
    let token = canary::canaries(plan)
        .into_iter()
        .next()
        .unwrap_or_default();
    let leaked = format!("copied from the sealed plan: {token}\n");
    let commit = repo.commit_on(
        &repo.commit(BASE)?,
        "canary-leak",
        &[(CANARY_FILE, leaked.as_bytes())],
    )?;
    add(
        "canary-leak",
        "canary-leak",
        commit,
        ControlKind::NeverAgain,
        Expect::Broken {
            target: Target::Reason {
                reason: FailReason::CanaryLeak,
            },
        },
        None,
    );
    Ok(out)
}

/// Whether `control` is in scope for `plan`: known good, an integrity or
/// limit target, or a promise one of the plan's checks quotes.
#[must_use]
pub fn in_scope(control: &Control, plan: &GenPlan, binding: &Binding) -> bool {
    match &control.expect {
        Expect::Broken {
            target: Target::Promise { text },
        } => binding.bind(plan).iter().any(|c| &c.promise_text == text),
        _ => true,
    }
}

/// What a control suite runs under.
#[derive(Debug)]
pub struct Suite<'a> {
    /// The sealed evaluator.
    pub evaluator: &'a Evaluator,
    /// An evaluator-role MCP client.
    pub client: &'a McpClient,
    /// The fixture repository.
    pub repo: &'a FixtureRepo,
    /// The plan (the holdout version under test).
    pub plan: &'a GenPlan,
    /// Its commit, registered with the server.
    pub plan_commit: PlanCommit,
    /// The holdout version.
    pub version: u64,
}

/// One control run.
#[derive(Debug)]
pub struct RanControl {
    /// The control.
    pub control: Control,
    /// The signed verdict record the evaluator produced.
    pub record: VerdictRecord,
    /// The ledger record appended for it.
    pub entry: Entry,
    /// Its body.
    pub body: ControlRecord,
}

/// Run each control `reps` times through the real evaluator, each with
/// fresh moment-of-test parameters, and append a control record per run.
///
/// # Errors
/// [`ControlError`] on the first evaluation, server or ledger failure.
pub fn run_controls(
    suite: &Suite<'_>,
    controls: &[Control],
    reps: usize,
    ledger: &mut Ledger,
    key: &EvaluatorKey,
) -> Result<Vec<RanControl>, ControlError> {
    let base = suite.repo.commit(BASE)?;
    let mut out = Vec::new();
    for c in controls {
        for _ in 0..reps {
            let req = EvalRequest {
                repo: &suite.repo.path,
                plan: suite.plan,
                plan_commit: suite.plan_commit,
                evaluated_commit: &c.commit,
                baseline_commit: &base,
            };
            let e = suite.evaluator.evaluate(suite.client, &req)?;
            if let Err(r) = &e.server {
                return Err(ControlError::Refused(r.to_string()));
            }
            let body = ControlRecord::from_run(suite.plan, suite.version, c, &e.record);
            let entry = ledger.append_json(key, kind::CONTROL, &body)?;
            e.world.remove();
            out.push(RanControl {
                control: c.clone(),
                record: e.record,
                entry,
                body,
            });
        }
    }
    Ok(out)
}
