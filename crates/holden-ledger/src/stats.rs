//! Rates with Wilson score intervals (slice-5 P2).
//!
//! For `k` successes in `n` trials, `p = k / n`, and `z` the two-sided normal
//! quantile (1.959963984540054 for 95%):
//!
//! ```text
//! centre = (p + z²/2n) / (1 + z²/n)
//! half   = z / (1 + z²/n) · sqrt(p(1 − p)/n + z²/4n²)
//! bounds = [centre − half, centre + half] clamped to [0, 1]
//! ```
//!
//! With no trials the interval is the whole of `[0, 1]`: nothing is known.

use serde::{Deserialize, Serialize};

/// The two-sided 95% normal quantile.
pub const Z95: f64 = 1.959_963_984_540_054;

/// The Wilson score interval of `k` successes in `n` trials at quantile `z`.
/// `k` is clamped to `n`.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn wilson(k: u64, n: u64, z: f64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let k = k.min(n);
    let nf = n as f64;
    let p = k as f64 / nf;
    let z2 = z * z;
    let denom = 1.0 + z2 / nf;
    let centre = (p + z2 / (2.0 * nf)) / denom;
    let half = z / denom * (p * (1.0 - p) / nf + z2 / (4.0 * nf * nf)).sqrt();
    let lower = if k == 0 {
        0.0
    } else {
        (centre - half).clamp(0.0, 1.0)
    };
    let upper = if k == n {
        1.0
    } else {
        (centre + half).clamp(0.0, 1.0)
    };
    (lower, upper)
}

/// A measured rate: `k` of `n` trials, with its point estimate and 95%
/// Wilson bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rate {
    /// Successes (catches, or false fails).
    pub k: u64,
    /// Trials (the sample size).
    pub n: u64,
}

impl Rate {
    /// Record one trial.
    pub fn add(&mut self, success: bool) {
        self.n += 1;
        if success {
            self.k += 1;
        }
    }

    /// Add another rate's trials.
    pub fn merge(&mut self, other: &Self) {
        self.k += other.k;
        self.n += other.n;
    }

    /// `k / n`, or `None` with no trials.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn point(&self) -> Option<f64> {
        (self.n > 0).then(|| self.k as f64 / self.n as f64)
    }

    /// The 95% Wilson lower bound.
    #[must_use]
    pub fn lower(&self) -> f64 {
        wilson(self.k, self.n, Z95).0
    }

    /// The 95% Wilson upper bound.
    #[must_use]
    pub fn upper(&self) -> f64 {
        wilson(self.k, self.n, Z95).1
    }
}

/// Catch and false-fail rates of one check, lens, cognition or plan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CheckStats {
    /// Known-broken controls caught (failed), of those run.
    pub catch: Rate,
    /// Known-good controls failed, of those run.
    pub false_fail: Rate,
}

impl CheckStats {
    /// Add another's trials.
    pub fn merge(&mut self, other: &Self) {
        self.catch.merge(&other.catch);
        self.false_fail.merge(&other.false_fail);
    }
}
