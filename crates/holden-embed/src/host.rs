//! What `holden embed` writes into a host repository, and how it is read
//! back. Every file here is public: the why, never the how.
//!
//! | Path | What |
//! |---|---|
//! | `.holden/config.json` | the public run file ([`HostConfig`]) |
//! | `.holden/agents/holden.md` | the agent's constitution, forged for the host's domain |
//! | `.holden/cognitions/<id>.json` | the public half of each proposed cognition |
//! | `.holden/cognitions/<id>.approval.json` | the owner's approval of it (`holden approve`) |
//! | `.mcp.json` | the implementer's MCP client config (`mcpServers.holden`) |
//! | `<git hooks>/post-commit` | [`HOOK_SHIM`] |

use std::fs;
use std::path::{Path, PathBuf};

use holden_cognition::{Approval, ClientPromises, PlanningDocument, Promise, SituatedCognition};
use holden_mcp::Role;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::state::State;
use crate::util::{HostGit, io_at, read_json};
use crate::{Error, refused};

/// The host's holden directory.
pub const HOLDEN_DIR: &str = ".holden";
/// The public run file.
pub const CONFIG_FILE: &str = ".holden/config.json";
/// Verbatim promise candidates and the planning detection report.
pub const PROMISE_PROPOSAL_FILE: &str = ".holden/promise-proposals.json";
/// Embed completed but the owner has not approved every proposed cognition.
pub const AWAITING_OWNER_APPROVAL: &str = "AWAITING_OWNER_APPROVAL";
/// Every proposed cognition has an owner approval record.
pub const OWNER_APPROVED: &str = "OWNER_APPROVED";
/// The agent's constitution.
pub const CONSTITUTION_FILE: &str = ".holden/agents/holden.md";
/// The public halves of the cognitions.
pub const COGNITIONS_DIR: &str = ".holden/cognitions";
/// The implementer's MCP client config (the harness reads it from the
/// project root).
pub const MCP_CONFIG_FILE: &str = ".mcp.json";
/// The MCP server name in the client config.
pub const MCP_SERVER_NAME: &str = "holden";

/// The post-commit hook: a two-line shim that only runs the `holden`
/// binary. All of its logic lives in Rust (`holden jit`).
pub const HOOK_SHIM: &str = "#!/bin/sh\nexec holden jit \"$@\"\n";

/// The role the host's MCP client config is installed with: the host is
/// the implementer, never the evaluator (slice-7 P2).
pub const HOST_ROLE: Role = Role::Implementer;

/// The public run file format id.
pub const HOST_FORMAT: &str = "holden.embed/1";

/// A promise as the host's public run file shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromiseCard {
    /// The promise id, hex.
    pub id: String,
    /// The owner's words, verbatim.
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposalDocument {
    source: String,
    source_sha256: String,
    promises: Vec<Promise>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposalFile {
    format: String,
    status: String,
    detection: Value,
    discovery: String,
    candidates: Vec<ProposalDocument>,
    owner_command: String,
}

/// Verify that the public proposal shown for approval contains exactly the
/// source spans freshly read through the client adapter. In particular, a
/// paraphrase cannot inherit a verbatim candidate's id, offsets, or approval.
///
/// # Errors
/// [`Error`] when the proposal is absent, malformed, or differs from the
/// detected source-backed candidates.
pub fn verify_promise_proposals(host: &Path, client: &ClientPromises) -> Result<(), Error> {
    let path = host.join(PROMISE_PROPOSAL_FILE);
    let proposal: ProposalFile = read_json(&path)?;
    if proposal.format != "holden.promise-proposals/1"
        || proposal.status != AWAITING_OWNER_APPROVAL
        || proposal.discovery
            != match client.discovery {
                holden_cognition::DiscoveryKind::Explicit => "explicit",
                holden_cognition::DiscoveryKind::Prose => "prose",
            }
        || proposal.detection["planning_process"] != client.adapter.kind.as_str()
        || proposal.owner_command != "holden approve --host ."
    {
        return Err(refused(
            "the promise proposal metadata does not match detection",
        ));
    }
    if proposal.candidates.len() != client.documents.len() {
        return Err(refused(
            "the promise proposal omits or adds a source document",
        ));
    }
    for (shown, actual) in proposal.candidates.iter().zip(&client.documents) {
        if shown.source != actual.document.name
            || shown.source_sha256 != actual.document.sha256().to_hex()
            || shown.promises != actual.promises
        {
            return Err(refused(format!(
                "the promise proposal for {} is not the verbatim source span",
                actual.document.name
            )));
        }
    }
    // Every field is checked above; retain the typed command field in this
    // contract rather than silently accepting an unknown proposal shape.
    Ok(())
}

/// The embedded agent, as the host sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentCard {
    /// Its constitution, relative to the host root.
    pub constitution: String,
    /// SHA-256 of the constitution.
    pub constitution_sha256: String,
    /// Its role at the server.
    pub role: String,
    /// The command that runs it as its own process.
    pub process: String,
}

/// `.holden/config.json`: the public run file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostConfig {
    /// [`HOST_FORMAT`].
    pub format: String,
    /// The host's directory name.
    pub host: String,
    /// The host's domain, from its planning document.
    pub domain: String,
    /// The planning document, relative to the host root.
    pub planning: String,
    /// SHA-256 of the primary planning document when it was embedded.
    pub planning_sha256: String,
    /// The typed product boundary declared at embed time. Cargo workspaces
    /// carry their real entry points; non-Cargo stacks are refused before
    /// this file is written. Optional only to read pre-product-binding hosts.
    #[serde(default)]
    pub product_binding: Option<holden_eval::ProductBinding>,
    /// The agent's private state directory, relative to the user state
    /// directory. Absolute values are accepted only when reading legacy
    /// hosts; embed always writes the relative form.
    pub state: PathBuf,
    /// The embedded agent.
    pub agent: AgentCard,
    /// The promises, in planning-document order.
    pub promises: Vec<PromiseCard>,
    /// The proposed cognitions (their public halves are in
    /// `.holden/cognitions/`), one per promise.
    pub cognitions: Vec<String>,
}

impl HostConfig {
    /// Read the host's run file.
    ///
    /// # Errors
    /// [`Error`] when it is absent or malformed.
    pub fn read(host: &Path) -> Result<Self, Error> {
        let c: Self = read_json(&host.join(CONFIG_FILE))?;
        if c.format != HOST_FORMAT {
            return Err(refused(".holden/config.json has an unknown format"));
        }
        Ok(c)
    }

    /// Resolve the agent state at run time. New host files carry only a
    /// user-state-relative name; legacy absolute values remain readable so
    /// their approvals can be migrated by re-embedding.
    ///
    /// # Errors
    /// [`Error::Refused`] when no user state directory can be resolved or a
    /// new-style value escapes it.
    pub fn state_dir(&self) -> Result<State, Error> {
        if self.state.is_absolute() {
            return Ok(State::new(&self.state));
        }
        if self
            .state
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(refused(".holden/config.json has an invalid state name"));
        }
        let root = crate::private_root()
            .ok_or_else(|| refused("no private root could be resolved for the state"))?;
        let path = root.join(&self.state);
        // Resolve the private alias to its real directory before constructing
        // a Unix socket path; the alias itself can exceed SUN_LEN.
        let resolved = fs::canonicalize(&path).map_err(io_at(&path))?;
        Ok(State::new(&resolved))
    }
}

/// The host's domain: the rest of the first line that starts with
/// `Domain: `, else the first `# ` heading, else the directory name.
#[must_use]
pub fn domain_of(doc: &PlanningDocument, fallback: &str) -> String {
    let text = String::from_utf8_lossy(&doc.bytes);
    text.lines()
        .find_map(|l| l.strip_prefix("Domain: "))
        .or_else(|| text.lines().find_map(|l| l.strip_prefix("# ")))
        .map_or_else(|| fallback.to_owned(), |s| s.trim().to_owned())
}

/// The file of cognition `id`.
#[must_use]
pub fn cognition_file(host: &Path, id: &str) -> PathBuf {
    host.join(COGNITIONS_DIR).join(format!("{id}.json"))
}

/// The approval file of cognition `id`.
#[must_use]
pub fn approval_file(host: &Path, id: &str) -> PathBuf {
    host.join(COGNITIONS_DIR)
        .join(format!("{id}.approval.json"))
}

fn json_files(dir: &Path, approvals: bool) -> Result<Vec<PathBuf>, Error> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(io_at(dir))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            n.ends_with(".json") && n.ends_with(".approval.json") == approvals
        })
        .collect();
    out.sort();
    Ok(out)
}

/// The public halves of the host's cognitions, as the files hold them.
///
/// # Errors
/// [`Error`] when a file is malformed.
pub fn read_cognitions(host: &Path) -> Result<Vec<SituatedCognition>, Error> {
    json_files(&host.join(COGNITIONS_DIR), false)?
        .iter()
        .map(|p| read_json(p))
        .collect()
}

/// The approval records in the host.
///
/// # Errors
/// [`Error`] when a file is malformed.
pub fn read_approvals(host: &Path) -> Result<Vec<Approval>, Error> {
    json_files(&host.join(COGNITIONS_DIR), true)?
        .iter()
        .map(|p| read_json(p))
        .collect()
}

/// The cognition files (not approvals) in the host, by path.
///
/// # Errors
/// [`Error::Io`].
pub fn cognition_files(host: &Path) -> Result<Vec<PathBuf>, Error> {
    json_files(&host.join(COGNITIONS_DIR), false)
}

/// The host's planning document.
///
/// # Errors
/// [`Error::Io`].
pub fn planning(host: &Path, name: &str) -> Result<PlanningDocument, Error> {
    let path = host.join(name);
    PlanningDocument::read(&path, name).map_err(io_at(&path))
}

/// The implementer's MCP client entry: the harness runs `holden connect`,
/// which resolves the server socket from the user state directory at run
/// time. No machine value is written into the host.
#[must_use]
pub fn mcp_entry(role: Role) -> Value {
    json!({
        "command": "holden",
        "args": ["connect", "--role", role.as_str()],
    })
}

/// The `holden` entry of a host's MCP client config: (role, socket).
///
/// # Errors
/// [`Error::Refused`] when the config has no well-formed `holden` entry.
pub fn installed_mcp_entry(host: &Path) -> Result<(String, PathBuf), Error> {
    let v: Value = read_json(&host.join(MCP_CONFIG_FILE))?;
    let args: Vec<&str> = v["mcpServers"][MCP_SERVER_NAME]["args"]
        .as_array()
        .ok_or_else(|| refused(".mcp.json has no holden server"))?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let after = |flag: &str| {
        args.iter()
            .position(|a| *a == flag)
            .and_then(|i| args.get(i + 1))
            .map(|s| (*s).to_owned())
    };
    if args.first() == Some(&"mcp") {
        let state = match after("--root") {
            Some(root) => State::new(Path::new(&root)),
            None => HostConfig::read(host)?.state_dir()?,
        };
        return Ok((HOST_ROLE.as_str().to_owned(), state.socket(HOST_ROLE)));
    }
    match (after("--role"), after("--socket")) {
        (Some(role), Some(socket)) => Ok((role, PathBuf::from(socket))),
        (Some(role), None) => Ok((
            role.clone(),
            HostConfig::read(host)?
                .state_dir()?
                .socket(if role == Role::Evaluator.as_str() {
                    Role::Evaluator
                } else {
                    Role::Implementer
                }),
        )),
        _ => Err(refused(".mcp.json's holden server names no role")),
    }
}

/// The host's post-commit hook path (honouring the repository's own
/// `core.hooksPath`).
///
/// # Errors
/// [`Error::Process`] when git cannot answer.
pub fn hook_path(host: &Path, git_home: &Path) -> Result<PathBuf, Error> {
    let rel =
        HostGit::new(host, git_home).run(&["rev-parse", "--git-path", "hooks/post-commit"])?;
    let p = PathBuf::from(rel);
    Ok(if p.is_absolute() { p } else { host.join(p) })
}
