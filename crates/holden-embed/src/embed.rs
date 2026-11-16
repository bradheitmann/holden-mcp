//! `holden embed <project>` (slice-7 P1, P2).
//!
//! It writes exactly these into the host repository, all public:
//!
//! - `.holden/config.json`: the public run file;
//! - `.holden/agents/holden.md`: the agent's constitution, forged for the
//!   host's domain through the forge's export ([`crate::forge`]);
//! - `.holden/cognitions/<id>.json`: the public half of one proposed
//!   cognition per promise, shaped by the host-domain pack;
//! - `.mcp.json`: the `holden` MCP server entry, with the implementer role
//!   and the implementer's socket ([`crate::host::HOST_ROLE`]);
//! - the `post-commit` hook: the two-line [`crate::host::HOOK_SHIM`].
//!
//! Everything sealed (keys, plans, server configs, the ledger, runs) is
//! written only to the agent's state directory ([`crate::state`]). Running
//! embed again writes the same bytes, keeps every existing key and leaves
//! the owner's approvals in place. An existing `post-commit` hook that is
//! not the shim is never replaced.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use holden_canon::Doctrine;
use holden_cognition::adapters::{ClientPromises, read_promises};
use holden_cognition::{DomainOverlay, Promise, SituatedCognition, candidates, load_pack_file};
use holden_eval::{notes_binding, product};
use holden_gen::DrawCorpus;
use holden_mcp::{Role, ServerConfig};
use holden_seal::EvaluatorKey;
use serde_json::{Value, json};

use crate::forge::forge_constitution;
use crate::host::{
    AWAITING_OWNER_APPROVAL, AgentCard, CONFIG_FILE, CONSTITUTION_FILE, HOOK_SHIM, HOST_FORMAT,
    HostConfig, OWNER_APPROVED, PROMISE_PROPOSAL_FILE, PromiseCard, cognition_file,
    cognition_files, domain_of, hook_path, read_approvals,
};
use crate::state::{AGENT_FORMAT, AgentConfig, State};
use crate::util::{HostGit, copy_tree, io_at, json_bytes, sha256_hex, write_if_changed};
use crate::{Error, refused};

/// How long a run stays open by default: long enough for a cold build.
pub const DEFAULT_RUN_WINDOW_MS: u64 = 600_000;

/// What to embed, and with what.
#[derive(Clone, Debug)]
pub struct EmbedOptions {
    /// The host repository's root.
    pub host: PathBuf,
    /// The agent's state directory (default `<private root>/hosts/<name>`).
    pub state: Option<PathBuf>,
    /// The `forge` binary.
    pub forge: PathBuf,
    /// The private-identifier denylist the forge export is scanned with.
    pub denylist: PathBuf,
    /// The lens corpus (copied into the state once). `None` uses the lens
    /// bytes shipped inside the installed executable.
    pub lens: Option<PathBuf>,
    /// Declared host-relative Cargo manifests, in declaration order.
    pub manifests: Vec<PathBuf>,
    /// The `cargo` of the toolchain host builds use.
    pub cargo: PathBuf,
    /// The `holden-confine` launcher.
    pub confine_helper: PathBuf,
    /// The owner's key store directory.
    pub owner_store: PathBuf,
    /// The planning document, relative to the host root.
    pub planning: String,
    /// The run window, in milliseconds.
    pub run_window_ms: u64,
}

/// What an embed did.
#[derive(Clone, Debug)]
pub struct EmbedReport {
    /// The host root (canonical).
    pub host: PathBuf,
    /// The state directory (canonical).
    pub state: PathBuf,
    /// Every host file embed owns.
    pub host_files: Vec<PathBuf>,
    /// The host files whose bytes this run changed.
    pub changed: Vec<PathBuf>,
    /// Stale host cognition files this run removed, host-relative.
    pub removed: Vec<PathBuf>,
    /// The exact owner command published in promise-proposals.json.
    pub owner_command: String,
    /// The agent keys this run created.
    pub keys_created: Vec<&'static str>,
    /// The planning detection report (slice-18a).
    pub detection: DetectionReport,
    /// The public run file.
    pub config: HostConfig,
    /// `AWAITING_OWNER_APPROVAL` until every proposed cognition is approved.
    pub status: &'static str,
}

/// One candidate in the planning detection report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateReport {
    /// Repository-relative source document.
    pub source: String,
    /// One-based source line.
    pub line: u64,
    /// Verbatim candidate text.
    pub text: String,
}

/// The planning detection report (slice-18a, shortened in slice-18b): the
/// planning process found, its source documents, and every candidate with
/// its source line. Callers print only the first 20 candidates; the full
/// list is written into the agent state. It never carries a machine path
/// or a sealed value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectionReport {
    /// The planning process the adapter detected (for example `markdown`).
    pub process: String,
    /// The source documents, repository-relative and sorted.
    pub documents: Vec<String>,
    /// Every candidate promise, in document order.
    pub candidates: Vec<CandidateReport>,
    /// Full-list path relative to the private root.
    pub full_list: String,
}

/// An embed that refused, with the detection report the caller still gets
/// when planning detection found candidate promises before the refusal
/// (slice-18a).
#[derive(Debug)]
pub struct RefusedEmbed {
    /// The detection report; absent only when detection itself found no
    /// promises or never ran.
    pub detection: Option<Box<DetectionReport>>,
    /// Why the embed was refused.
    pub error: Error,
}

impl fmt::Display for RefusedEmbed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}

impl std::error::Error for RefusedEmbed {}

/// The report of the planning detection that already ran.
fn detection_report(client: &ClientPromises, host_name: &str) -> DetectionReport {
    let mut candidates = Vec::new();
    for document in &client.documents {
        let mut scanned = 0usize;
        let mut line = 1u64;
        for promise in &document.promises {
            let start = usize::try_from(promise.start).unwrap_or_default();
            line += document.document.bytes[scanned..start]
                .iter()
                .filter(|byte| **byte == b'\n')
                .count() as u64;
            scanned = start;
            candidates.push(CandidateReport {
                source: document.document.name.clone(),
                line,
                text: promise.text.clone(),
            });
        }
    }
    DetectionReport {
        process: client.adapter.kind.as_str().to_owned(),
        documents: client
            .adapter
            .documents
            .iter()
            .map(|d| d.to_string_lossy().into_owned())
            .collect(),
        candidates,
        full_list: format!("hosts/{host_name}/candidate-promises.txt"),
    }
}

fn canonical(p: &Path) -> Result<PathBuf, Error> {
    fs::canonicalize(p).map_err(io_at(p))
}

/// Publish a custom state's stable user-state-relative location. The host
/// records only `hosts/<name>`; this private symlink is what lets an
/// explicitly located state remain discoverable without leaking its path.
fn publish_state_location(host: &Path, state: &State) -> Result<PathBuf, Error> {
    let default = State::default_for(host)
        .ok_or_else(|| refused("no private root could be resolved for the state"))?;
    if default.dir() == state.dir() {
        return Ok(PathBuf::from("hosts").join(host.file_name().unwrap_or_default()));
    }
    let parent = default
        .dir()
        .parent()
        .ok_or_else(|| refused("the user state directory has no hosts directory"))?;
    crate::util::private_dir(parent)?;
    match fs::symlink_metadata(default.dir()) {
        Ok(meta) if meta.file_type().is_symlink() => {
            if fs::read_link(default.dir()).map_err(io_at(default.dir()))? != state.dir() {
                return Err(refused("the host already names a different agent state"));
            }
        }
        Ok(_) => {
            return Err(refused(
                "the default agent state exists; use it instead of a custom state",
            ));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::os::unix::fs::symlink(state.dir(), default.dir()).map_err(io_at(default.dir()))?;
        }
        Err(e) => {
            return Err(Error::Io {
                path: default.dir().to_path_buf(),
                kind: e.kind(),
            });
        }
    }
    Ok(PathBuf::from("hosts").join(host.file_name().unwrap_or_default()))
}

/// The sealed server config of `role` for the agent in `state`.
fn server_config(state: &State, role: Role, evaluator_public: &str, window: u64) -> Value {
    let mut v = json!({
        "socket_dir": state.socket_dir(role),
        "key_dir": state.keys_dir(),
        "epoch_key": "epoch",
        "seal_key": "seal",
        "plans_dir": state.plans_dir(),
        "run_window_ms": window,
        "roles": [{ "uid": holden_seal::keystore::current_uid(), "role": role.as_str() }],
        "verdict_key": evaluator_public,
    });
    if role == Role::Evaluator {
        v["ledger"] = json!({ "db": state.ledger_db(), "key": "ledger" });
    }
    v
}

/// The proposed cognition of each promise: the first candidate the
/// doctrine's generators make with the host-domain overlay.
fn proposed(
    promises: &[Promise],
    overlay: &DomainOverlay,
) -> Result<Vec<SituatedCognition>, Error> {
    let doctrine = Doctrine::holden();
    if promises.is_empty() {
        return Err(refused("the planning adapter states no owner promise"));
    }
    promises
        .iter()
        .map(|p| {
            candidates(&doctrine, p, Some(overlay))
                .into_iter()
                .next()
                .ok_or_else(|| refused("the doctrine proposes no cognition for a promise"))
        })
        .collect()
}

fn install_bundled_lens(state: &State) -> Result<(), Error> {
    let files: [(&str, &[u8]); 4] = [
        (
            "draw.json",
            include_bytes!("../../holden-cognition/fixtures/project-synthetic/lens/draw.json"),
        ),
        (
            "corpus/concepts.json",
            include_bytes!(
                "../../holden-cognition/fixtures/project-synthetic/lens/corpus/concepts.json"
            ),
        ),
        (
            "corpus/retired.json",
            include_bytes!(
                "../../holden-cognition/fixtures/project-synthetic/lens/corpus/retired.json"
            ),
        ),
        (
            "corpus/cognitions.json",
            include_bytes!(
                "../../holden-cognition/fixtures/project-synthetic/lens/corpus/cognitions.json"
            ),
        ),
    ];
    for (relative, bytes) in files {
        crate::util::write_if_changed(&state.lens_dir().join(relative), bytes, 0o600)?;
    }
    Ok(())
}

/// Embed a holden agent into the host (see the module docs).
///
/// # Errors
/// [`RefusedEmbed`] naming what was refused, carrying the detection report
/// when planning detection found candidate promises before the refusal.
pub fn embed(opts: &EmbedOptions) -> Result<EmbedReport, RefusedEmbed> {
    let mut detection = None;
    run(opts, &mut detection).map_err(|error| RefusedEmbed { detection, error })
}

fn run(
    opts: &EmbedOptions,
    detection: &mut Option<Box<DetectionReport>>,
) -> Result<EmbedReport, Error> {
    let host = canonical(&opts.host)?;
    let state = match &opts.state {
        Some(s) => State::new(s),
        None => State::default_for(&host)
            .ok_or_else(|| refused("no private root could be resolved for the state"))?,
    };
    let git = HostGit::new(&host, &state.git_home());
    let top = PathBuf::from(git.run(&["rev-parse", "--show-toplevel"])?);
    if canonical(&top)? != host {
        return Err(refused("the project is not the root of a git work tree"));
    }
    let host_name = host
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    // Detection and extraction are always the client adapter: Charter,
    // dispatch graph, then Markdown. The old single-file --planning value
    // remains in the public config for compatibility, never as another path
    // around adapter detection or byte-span verification.
    let client = read_promises(&host).map_err(|e| refused(e.to_string()))?;
    let promises: Vec<Promise> = client.spans().into_iter().map(|(_, p)| p.clone()).collect();
    if promises.is_empty() {
        return Err(refused("the planning adapter states no owner promise"));
    }
    let primary = client
        .documents
        .iter()
        .find(|d| d.document.name == "charter/INTENT.md")
        .or_else(|| client.documents.iter().find(|d| !d.promises.is_empty()))
        .map(|d| &d.document)
        .ok_or_else(|| refused("the planning adapter selected no source document"))?;
    // The report of the detection that already ran (slice-18a). Set only
    // once promises exist, so a no-promise refusal carries none; every
    // later refusal (stack, lens, forge, hook) carries it.
    let report = detection_report(&client, &host_name);
    *detection = Some(Box::new(report.clone()));
    let domain = domain_of(primary, &host_name);
    let established = notes_binding();
    let product_binding = if promises.iter().all(|promise| {
        established
            .promises
            .iter()
            .any(|(text, _)| *text == promise.text)
    }) {
        product::declare(&established)
    } else {
        product::discover_declared(&host, &opts.cargo, &opts.manifests)
            .map_err(|e| refused(e.to_string()))?
    };
    // Binding and declaration validation are preflight: an unsupported host
    // is refused before a lens, key, state directory or public host file is
    // created. The subsequent fix-and-retry is a clean first embed.
    state.create_dirs()?;
    let state = State::new(&canonical(state.dir())?);
    let public_state = publish_state_location(&host, &state)?;
    let full_list = report
        .candidates
        .iter()
        .map(|candidate| {
            format!(
                "{}:{}\t{}\n",
                candidate.source, candidate.line, candidate.text
            )
        })
        .collect::<String>();
    write_if_changed(
        &state.dir().join("candidate-promises.txt"),
        full_list.as_bytes(),
        0o600,
    )?;
    let keys_created = state.ensure_keys()?;
    if !state.lens_dir().exists() {
        match &opts.lens {
            Some(source) => copy_tree(source, &state.lens_dir())?,
            None => install_bundled_lens(&state)?,
        }
    }
    DrawCorpus::load(&state.lens_dir()).map_err(|e| refused(format!("the lens corpus: {e}")))?;
    let constitution = forge_constitution(
        &opts.forge,
        &opts.denylist,
        &state.forge_work(),
        &domain,
        &state.host_pack(),
    )?;
    let pack = load_pack_file(&state.host_pack()).map_err(|e| refused(e.to_string()))?;
    let cognitions = proposed(&promises, &DomainOverlay::from_pack(&pack))?;

    // The host's public files.
    let mut host_files = Vec::new();
    let mut changed = Vec::new();
    let mut removed = Vec::new();
    let mut put = |path: PathBuf, bytes: &[u8], mode: u32| -> Result<(), Error> {
        if write_if_changed(&path, bytes, mode)? {
            changed.push(path.clone());
        }
        host_files.push(path);
        Ok(())
    };
    put(host.join(CONSTITUTION_FILE), constitution.as_bytes(), 0o644)?;
    let ids: Vec<String> = cognitions.iter().map(|c| c.id.to_hex()).collect();
    for c in &cognitions {
        put(
            cognition_file(&host, &c.id.to_hex()),
            &json_bytes(c)?,
            0o644,
        )?;
    }
    for stale in cognition_files(&host)? {
        let keep = ids.iter().any(|id| {
            stale
                .file_name()
                .is_some_and(|n| n == format!("{id}.json").as_str())
        });
        if !keep {
            fs::remove_file(&stale).map_err(io_at(&stale))?;
            removed.push(stale.strip_prefix(&host).unwrap_or(&stale).to_path_buf());
        }
    }
    let approvals = read_approvals(&host)?;
    let status = if cognitions.iter().all(|cognition| {
        approvals.iter().any(|approval| {
            approval.cognition == cognition.id && approval.promise == cognition.promise
        })
    }) {
        OWNER_APPROVED
    } else {
        AWAITING_OWNER_APPROVAL
    };
    let proposal = json!({
        "format": "holden.promise-proposals/1",
        // A proposal is an immutable discovery snapshot. Approval records
        // live beside cognitions; rewriting this file after approval would
        // make an otherwise idempotent re-embed change public bytes.
        "status": AWAITING_OWNER_APPROVAL,
        "detection": {
            "planning_process": client.adapter.kind.as_str(),
            "documents": client.adapter.documents,
            "limitations": client.adapter.limitations.iter().map(|l| l.code()).collect::<Vec<_>>(),
        },
        "discovery": match client.discovery {
            holden_cognition::DiscoveryKind::Explicit => "explicit",
            holden_cognition::DiscoveryKind::Prose => "prose",
        },
        "candidates": client.documents.iter().map(|document| json!({
            "source": document.document.name,
            "source_sha256": document.document.sha256().to_hex(),
            "promises": document.promises,
        })).collect::<Vec<_>>(),
        "owner_command": "holden approve --host .",
    });
    put(
        host.join(PROMISE_PROPOSAL_FILE),
        &json_bytes(&proposal)?,
        0o644,
    )?;
    let config = HostConfig {
        format: HOST_FORMAT.to_owned(),
        host: host_name,
        domain,
        planning: primary.name.clone(),
        planning_sha256: primary.sha256().to_hex(),
        product_binding: Some(product_binding),
        state: public_state,
        agent: AgentCard {
            constitution: CONSTITUTION_FILE.to_owned(),
            constitution_sha256: sha256_hex(constitution.as_bytes()),
            role: Role::Evaluator.as_str().to_owned(),
            process: "holden agent serve --host .".to_owned(),
        },
        promises: promises
            .iter()
            .map(|p| PromiseCard {
                id: p.id.to_hex(),
                text: p.text.clone(),
            })
            .collect(),
        cognitions: ids,
    };
    put(host.join(CONFIG_FILE), &json_bytes(&config)?, 0o644)?;
    for file in crate::distribution::write_harness_configs_at(&host, Some(state.dir()))?.into_iter()
    {
        let bytes = fs::read(&file).map_err(io_at(&file))?;
        put(file, &bytes, 0o644)?;
    }
    let hook = hook_path(&host, &state.git_home())?;
    match fs::read(&hook) {
        Ok(existing) if existing != HOOK_SHIM.as_bytes() => {
            return Err(refused(
                "the host already has a post-commit hook that is not holden's; it is not replaced",
            ));
        }
        _ => put(hook, HOOK_SHIM.as_bytes(), 0o755)?,
    }

    // The agent's sealed side.
    let evaluator_public = hex::encode(
        EvaluatorKey::from_material(&state.key("evaluator")?)
            .public()
            .0,
    );
    for role in [Role::Evaluator, Role::Implementer] {
        let v = server_config(&state, role, &evaluator_public, opts.run_window_ms);
        ServerConfig::from_json(&v.to_string()).map_err(|e| refused(e.to_string()))?;
        write_if_changed(&state.server_config(role), &json_bytes(&v)?, 0o600)?;
    }
    state.write_agent_config(&AgentConfig {
        format: AGENT_FORMAT.to_owned(),
        host: host.clone(),
        planning: primary.name.clone(),
        cargo: opts.cargo.clone(),
        confine_helper: opts.confine_helper.clone(),
        owner_store: opts.owner_store.clone(),
        constitution_sha256: config.agent.constitution_sha256.clone(),
        run_window_ms: opts.run_window_ms,
    })?;
    Ok(EmbedReport {
        host,
        state: state.dir().to_path_buf(),
        host_files,
        changed,
        removed,
        owner_command: proposal["owner_command"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        keys_created,
        detection: report,
        config,
        status,
    })
}
