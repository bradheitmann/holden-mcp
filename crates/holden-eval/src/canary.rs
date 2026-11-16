//! Canary scan (never-again control (a)).
//!
//! The canaries of a plan are its sealed lens-draw MACs: 64-hex strings
//! computed under the sealed draw key, present only in the sealed plan and
//! never on the public card. If one appears in the code under test, the
//! sealed plan leaked to the builder.
//!
//! The scan reads the evaluated commit's own tree from the git object store
//! (`git ls-tree` and `git cat-file` at that commit). It never takes a path
//! from the caller, so a clean working directory or a chosen subdirectory
//! cannot hide a leak that the evaluated commit carries.

use holden_gen::GenPlan;
use serde::{Deserialize, Serialize};

use crate::EvalError;
use crate::git::Git;

/// One canary found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanaryFinding {
    /// The path inside the evaluated tree.
    pub path: String,
    /// Which canary (its index; the token itself is never recorded).
    pub canary: usize,
}

/// The scan's result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanaryReport {
    /// The commit whose tree was scanned.
    pub commit: String,
    /// How many canaries were searched for.
    pub canaries: usize,
    /// How many blobs were read.
    pub blobs: u64,
    /// How many bytes were read.
    pub bytes: u64,
    /// What was found.
    pub findings: Vec<CanaryFinding>,
}

/// The canaries of `plan`.
#[must_use]
pub fn canaries(plan: &GenPlan) -> Vec<String> {
    plan.provenance.draw.macs.clone()
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && hay
            .windows(needle.len())
            .any(|w| w.eq_ignore_ascii_case(needle))
}

/// Scan the tree of `commit` in `git` for `canaries`.
///
/// # Errors
/// [`EvalError::Git`] when the tree cannot be read.
pub fn scan_commit(
    git: &Git,
    commit: &str,
    canaries: &[String],
) -> Result<CanaryReport, EvalError> {
    let entries: Vec<_> = git
        .ls_tree(commit)?
        .into_iter()
        .filter(|e| e.mode != "160000")
        .collect();
    let oids: Vec<String> = entries.iter().map(|e| e.oid.clone()).collect();
    let blobs = git.cat_blobs(&oids)?;
    let mut findings = Vec::new();
    let mut bytes = 0u64;
    for (entry, blob) in entries.iter().zip(&blobs) {
        bytes += blob.len() as u64;
        for (i, c) in canaries.iter().enumerate() {
            if contains(blob, c.as_bytes()) || contains(entry.path.as_bytes(), c.as_bytes()) {
                findings.push(CanaryFinding {
                    path: entry.path.clone(),
                    canary: i,
                });
            }
        }
    }
    Ok(CanaryReport {
        commit: commit.to_owned(),
        canaries: canaries.len(),
        blobs: blobs.len() as u64,
        bytes,
        findings,
    })
}
