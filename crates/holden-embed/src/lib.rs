//! holden-embed: installs a separate holden agent into a host project and
//! runs its just-in-time sealed holdouts on every commit
//! (docs/ARCHITECTURE.md §3.8; slice-7).
//!
//! - [`embed`]: `holden embed <project>` writes exactly the host's public
//!   files (`.holden/config.json`, the agent's constitution, the public
//!   halves of the proposed cognitions, the implementer's MCP client config
//!   and a two-line `post-commit` shim) and keeps everything sealed in the
//!   agent's private state directory ([`state`]). It is idempotent.
//! - [`forge`]: the agent's constitution is forged for the host's domain
//!   through the forge's `export` and files, never as a crate dependency.
//! - [`approve`]: the owner signs the host's promises and cognitions with the
//!   owner key from the owner's key store (`holden approve`).
//! - [`agent`]: the embedded agent, its own process (`holden agent serve`):
//!   it admits only owner-approved cognitions, seals their plans, serves the
//!   MCP oracle with the evaluator role on its own socket and the
//!   implementer role on the host's, and commits a nonce for every plan
//!   before any commit it will evaluate exists.
//! - [`jit`]: `holden jit`, run by the post-commit shim: the whole
//!   just-in-time flow, from the pre-committed nonce to the signed verdict in
//!   the ledger and the fixed diagnostic.
//! - [`trace`]: `holden trace <run>`, the whole chain from the owner's
//!   promise to the verdict, each link re-verified.
//! - [`connect`]: `holden connect`, the stdio bridge a harness runs to reach
//!   the server's socket.
//! - [`fixture`]: the host fixture project, created at run time.

pub mod agent;
pub mod approve;
pub mod connect;
pub mod distribution;
pub mod embed;
pub mod fixture;
pub mod forge;
pub mod host;
pub mod jit;
pub mod state;
pub mod trace;
mod util;

use std::fmt;
use std::io;
use std::path::PathBuf;

pub use util::private_root;

/// Why an embed, approval, agent, just-in-time run or trace was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A check refused, with the reason (never a secret).
    Refused(String),
    /// An I/O error on `path`.
    Io {
        /// The path.
        path: PathBuf,
        /// What failed.
        kind: io::ErrorKind,
    },
    /// A process holden started failed.
    Process {
        /// The program.
        program: String,
        /// Why.
        why: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(why) => write!(f, "refused: {why}"),
            Self::Io { path, kind } => write!(f, "refused: {}: {kind}", path.display()),
            Self::Process { program, why } => write!(f, "refused: {program}: {why}"),
        }
    }
}

impl std::error::Error for Error {}

/// A refusal with `why`.
pub(crate) fn refused(why: impl Into<String>) -> Error {
    Error::Refused(why.into())
}
