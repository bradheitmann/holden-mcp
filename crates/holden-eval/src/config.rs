//! Public-config diff (never-again control (b)).
//!
//! Every public config path of the product binding is read at the baseline
//! commit and at the evaluated commit, from git objects. Any change (added,
//! removed or modified) is reported in the verdict with both hashes and a
//! bounded line diff. Nothing is skipped silently: a change too large to
//! diff is still reported, with its hashes and `diff_truncated`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::EvalError;
use crate::git::Git;

/// Most diff lines kept per file.
pub const MAX_DIFF_LINES: usize = 200;

/// Longest diff line kept, in characters.
pub const MAX_LINE_CHARS: usize = 200;

/// Files longer than this many lines are reported without a line diff.
const MAX_DIFF_INPUT_LINES: usize = 2000;

/// One changed public config file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigChange {
    /// The path.
    pub path: String,
    /// SHA-256 at the baseline commit; `None` when absent there.
    pub baseline_sha256: Option<String>,
    /// SHA-256 at the evaluated commit; `None` when absent there.
    pub evaluated_sha256: Option<String>,
    /// `-` and `+` lines.
    pub diff: Vec<String>,
    /// Whether the diff was cut short.
    pub diff_truncated: bool,
}

fn clip(line: &str) -> String {
    line.chars().take(MAX_LINE_CHARS).collect()
}

/// A line diff of `a` and `b` (longest common subsequence).
fn line_diff(a: &str, b: &str) -> (Vec<String>, bool) {
    let a: Vec<&str> = a.lines().collect();
    let b: Vec<&str> = b.lines().collect();
    if a.len() > MAX_DIFF_INPUT_LINES || b.len() > MAX_DIFF_INPUT_LINES {
        return (Vec::new(), true);
    }
    let (n, m) = (a.len(), b.len());
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            i += 1;
            j += 1;
        } else if j < m && (i == n || lcs[i][j + 1] >= lcs[i + 1][j]) {
            out.push(format!("+{}", clip(b[j])));
            j += 1;
        } else {
            out.push(format!("-{}", clip(a[i])));
            i += 1;
        }
    }
    let truncated = out.len() > MAX_DIFF_LINES;
    out.truncate(MAX_DIFF_LINES);
    (out, truncated)
}

fn sha(b: &[u8]) -> String {
    hex::encode(Sha256::digest(b))
}

/// Diff `paths` between `baseline` and `evaluated`.
///
/// # Errors
/// [`EvalError::Git`] when git cannot read a commit.
pub fn diff_public_config(
    git: &Git,
    baseline: &str,
    evaluated: &str,
    paths: &[&str],
) -> Result<Vec<ConfigChange>, EvalError> {
    let mut changes = Vec::new();
    for path in paths {
        let before = git.file_at(baseline, path)?;
        let after = git.file_at(evaluated, path)?;
        if before == after {
            continue;
        }
        let text = |b: &Option<Vec<u8>>| {
            b.as_deref()
                .map(|v| String::from_utf8_lossy(v).into_owned())
                .unwrap_or_default()
        };
        let (diff, diff_truncated) = line_diff(&text(&before), &text(&after));
        changes.push(ConfigChange {
            path: (*path).to_owned(),
            baseline_sha256: before.as_deref().map(sha),
            evaluated_sha256: after.as_deref().map(sha),
            diff,
            diff_truncated,
        });
    }
    Ok(changes)
}
