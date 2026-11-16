//! holden-eval: runs a sealed plan against a real product at its boundary,
//! confined without Docker, and records a signed, replayable verdict
//! (docs/ARCHITECTURE.md §3.4–3.5; slice-4).
//!
//! - [`params`]: moment-of-test parameters accepted only as a verified
//!   server reveal; [`plant`]: the values planted from them.
//! - [`binding`]: how each approved promise is observed at the notes
//!   product's boundary; [`drive`]: the boundary observations; [`rules`]:
//!   the decision.
//! - [`confine`] and the `holden-confine` launcher: `sandbox-exec` on macOS,
//!   Landlock on Linux, rlimits on both; [`spawn`]: scrubbed environments and
//!   enforced limits.
//! - Never-again controls: [`canary`] (the evaluated tree is scanned),
//!   [`config`] (public config changes are reported), [`spawn`] (limits are
//!   enforced), [`world`] (world ids are unique per evaluation).
//! - [`record`]: the Ed25519-signed verdict record and [`record::replay`].
//! - [`evaluate`]: one evaluation end to end, over MCP ([`client`]).
//! - [`fixture`]: the fixture repository built from the committed notes
//!   product and its variant overlays.

pub mod binding;
pub mod canary;
pub mod client;
pub mod config;
pub mod confine;
pub mod drive;
pub mod evaluate;
pub mod fixture;
pub mod git;
pub mod params;
pub mod plant;
pub mod product;
pub mod record;
pub mod rules;
pub mod spawn;
pub mod world;

use std::fmt;
use std::io;

pub use binding::{Binding, BoundCheck, Probe, notes_binding};
pub use client::{McpClient, ToolRefusal};
pub use evaluate::{EvalRequest, Evaluation, Evaluator, Toolchain};
pub use params::{ParamRefusal, RevealWire, ServerReveal};
pub use plant::Planted;
pub use product::{
    CargoEntryPoint, CargoManifestBinding, CargoWorkspaceBinding, DeclaredBinding, ProductBinding,
    ProductBindingError,
};
pub use record::{PublicVerdict, ReplayRefusal, VerdictBody, VerdictRecord, parse_record, replay};
pub use rules::{Decision, FailReason, RULES_VERSION};

/// Why an evaluation could not produce a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    /// The OS random source failed.
    NoRandomness,
    /// A world directory with this id already existed.
    WorldCollision,
    /// An I/O error.
    Io(&'static str, io::ErrorKind),
    /// Git refused.
    Git(String),
    /// The fixture could not be built.
    Fixture(String),
    /// Confinement was not established.
    Confinement(String),
    /// The revealed parameters were refused.
    Params(ParamRefusal),
    /// The server refused.
    Server(ToolRefusal),
    /// The MCP client failed.
    Client(String),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRandomness => f.write_str("the OS random source failed"),
            Self::WorldCollision => f.write_str("a world with this id already exists"),
            Self::Io(what, kind) => write!(f, "I/O error on the {what}: {kind:?}"),
            Self::Git(why) => write!(f, "git: {why}"),
            Self::Fixture(why) => write!(f, "fixture: {why}"),
            Self::Confinement(why) => write!(f, "confinement: {why}"),
            Self::Params(r) => write!(f, "parameters refused: {r}"),
            Self::Server(r) => write!(f, "server refused: {r}"),
            Self::Client(why) => write!(f, "MCP client: {why}"),
        }
    }
}

impl std::error::Error for EvalError {}
