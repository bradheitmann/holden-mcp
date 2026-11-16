//! holden-ledger: the append-only signed ledger, measured effectiveness,
//! authority from the pessimistic bound, and feedback
//! (docs/ARCHITECTURE.md §3.6; slice-5, G-MEASURED).
//!
//! - [`store`]: SQLite (bundled), one file in the private root's `db/`;
//!   records hash-chained with SHA-256 and signed with Ed25519; appends check
//!   only the tip; [`store::Ledger::verify`] checks the whole chain and names
//!   each record that does not hold.
//! - [`controls`]: sealed known-good and known-broken controls of the
//!   fixture product, run through the real evaluator; one
//!   [`records::ControlRecord`] per run, read from the signed verdict.
//! - [`stats`] and [`measure`]: catch and false-fail rates per check, lens
//!   concept, cognition, plan and integrity rule, with 95% Wilson bounds.
//! - [`authority`]: advise, recommend or block from the bounds only,
//!   against owner-set bars; limitations cap at recommend.
//! - [`feedback`]: lens draw weights from yield (never zero) and re-plan
//!   triggers (canary leak, vacuity, proven false) that bump the holdout
//!   version.
//! - [`telemetry`]: the report the MCP `telemetry` tool serves.

pub mod authority;
pub mod controls;
pub mod feedback;
pub mod measure;
pub mod records;
pub mod stats;
pub mod store;
pub mod telemetry;

pub use authority::{Authority, Bar, Bars, VerdictAuthority, authority, capped, verdict_authority};
pub use measure::Measurements;
pub use stats::{CheckStats, Rate, Z95, wilson};
pub use store::{Entry, Finding, Ledger, LedgerError, Problem, TipAnchor, VerifyReport};
