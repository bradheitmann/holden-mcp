//! holden-mcp: the MCP server behind G-UNPREDICTABLE at the service boundary
//! (docs/ARCHITECTURE.md §3.2).
//!
//! It uses the official Rust MCP SDK (`rmcp`) and exposes eight audit tools on
//! top of `holden-seal`, plus the slice-8 W1–W4 workflow composition tools:
//!
//! | Tool | Who | When |
//! |---|---|---|
//! | `brief` | anyone | always |
//! | `commit_nonce` | any mapped role | before the builder commits |
//! | `begin_run` | evaluator | needs an unused commitment for the plan |
//! | `reveal_params` | evaluator | while the run window is open |
//! | `submit_observation` | evaluator | while the run window is open (slice-4) |
//! | `verdict` | evaluator | while the run window is open; closes the run (slice-4) |
//! | `audit_reveal` | any mapped role | after the run window has closed |
//! | `telemetry` | evaluator | when a signed ledger is configured (slice-5) |
//!
//! Slice-5: with a `ledger` in the sealed config, every accepted verdict is
//! appended to the signed ledger (`holden-ledger`), and the `verdict`
//! answer carries the verdict's authority: the minimum, across the checks
//! that decided it, of the authority each earned on sealed controls against
//! the configured bars, capped at `recommend` by any confinement
//! limitation.
//!
//! Run close: `begin_run` records a run window from the sealed config. When
//! it expires, or when `verdict` accepts a record, the run is closed:
//! `reveal_params` is refused with `run_closed`, and `audit_reveal` answers.
//!
//! Plans: registration accepts generated plans (`holden_gen::GenPlan`, the
//! format the generator emits and the evaluator runs) as well as slice-1
//! plans.
//!
//! Roles come from the sealed [`ServerConfig`], keyed by the uid that the
//! kernel reports for the socket peer. Every refusal is a typed [`Refusal`]
//! with a fixed message, so no error path carries a key, plan plaintext,
//! nonce or parameter.

pub mod config;
pub mod ledger;
pub mod server;
pub mod workflow;

pub use config::{ConfigError, Role, ServerConfig, loopback_bind};
pub use ledger::{
    Caller, Clock, Core, Log, ManualClock, PlanKind, Refusal, RunRecord, SealedPlanFile,
    StartError, TOOLS, VerdictSummary, Via, open_plan,
};
pub use server::{Listener, Session, serve_socket, serve_stdio, serve_tcp};
