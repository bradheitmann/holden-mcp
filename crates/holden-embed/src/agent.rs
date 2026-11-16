//! The embedded holden agent: its own process (`holden agent serve`), with
//! its own constitution and the evaluator role (slice-7 P2, P3, P4).
//!
//! At start it:
//! 1. checks that the host still holds the constitution it was embedded
//!    with (its digest is in the agent's config);
//! 2. admits only the cognitions the owner approved, against the owner's
//!    published key ([`holden_cognition::admit_approved`]); every other one
//!    is refused and hosts no test;
//! 3. generates and seals one plan per admitted cognition into the state;
//! 4. opens the MCP server core on the evaluator's sealed config and serves
//!    two sockets: its own (`se/`, the uid mapped to the evaluator) and the
//!    host's (`si/`, the uid mapped to the implementer), so the role comes
//!    from which socket the server's role map gives the caller;
//! 5. commits a server nonce for every plan and records each commitment
//!    with the host's HEAD in `armed.json`: every run it will open is
//!    opened with a nonce committed before the evaluated commit existed.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use holden_canon::Doctrine;
use holden_cognition::{ApprovalError, Promise, adapters::read_promises};
use holden_gen::{DrawCorpus, DrawInputs, GenPlan, draw_lens, generate, seal_plan, unseal_plan};
use holden_mcp::{Caller, Clock, Core, Log, Role, SealedPlanFile, ServerConfig, Via, serve_socket};
use holden_seal::{DrawKey, PlanCommit, SealKey, plan_commit};

use crate::host::{CONSTITUTION_FILE, read_approvals, read_cognitions};
use crate::state::{Armed, State};
use crate::util::{HostGit, io_at, sha256_hex, write_if_changed};
use crate::{Error, refused};

/// One sealed plan the agent serves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedPlan {
    /// The plan id, hex.
    pub plan_id: String,
    /// Its commit.
    pub plan_commit: PlanCommit,
    /// The sealed plan file.
    pub file: PathBuf,
    /// The promise id, hex.
    pub promise: String,
    /// The cognition id, hex.
    pub cognition: String,
}

/// What admission and sealing produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prepared {
    /// The plans of the admitted cognitions.
    pub plans: Vec<PreparedPlan>,
    /// Each refused cognition (hex id) and why.
    pub refused: Vec<(String, String)>,
}

/// The sealed plan in `file`, when it opens under `key` to the plan `id`.
fn existing_commit(file: &std::path::Path, key: &SealKey, id: &str) -> Option<PlanCommit> {
    let bytes = fs::read(file).ok()?;
    let wire: SealedPlanFile = serde_json::from_slice(&bytes).ok()?;
    let sealed = wire.to_sealed().ok()?;
    let commit = plan_commit(&sealed);
    let plan: GenPlan = unseal_plan(key, &sealed, &commit).ok()?;
    (plan.id.to_hex() == id && plan.computed_id() == plan.id).then_some(commit)
}

/// Admit the owner-approved cognitions of the host and seal one plan for
/// each into the state; remove sealed plans of anything no longer admitted.
///
/// # Errors
/// [`Error`] when the host's files or the state refuse.
pub fn prepare_plans(state: &State) -> Result<Prepared, Error> {
    let agent = state.agent_config()?;
    let client = read_promises(&agent.host).map_err(|e| refused(e.to_string()))?;
    let promises: Vec<Promise> = client.spans().into_iter().map(|(_, p)| p.clone()).collect();
    let cognitions = read_cognitions(&agent.host)?;
    let approvals = read_approvals(&agent.host)?;
    let (admitted, refusals) =
        match holden_cognition::OwnerTrust::read_published(&agent.owner_store) {
            Ok(trust) => client.admit(&cognitions, &approvals, &trust),
            Err(_) => (
                Vec::new(),
                cognitions
                    .iter()
                    .map(|c| (c.id, ApprovalError::Unapproved))
                    .collect(),
            ),
        };
    let corpus =
        DrawCorpus::load(&state.lens_dir()).map_err(|e| refused(format!("lens corpus: {e}")))?;
    let draw_key = DrawKey::new(state.key("draw")?);
    let seal = SealKey::new(state.key("seal")?);
    let doctrine = Doctrine::holden();
    let approved: Vec<&Promise> = promises.iter().collect();
    let mut plans = Vec::new();
    for h in &admitted {
        let inputs = DrawInputs {
            promise: h.promise().id,
            cognition: h.cognition().id,
            epoch: 0,
        };
        let d = draw_lens(&draw_key, &corpus, &inputs).map_err(|e| refused(e.to_string()))?;
        let plan = generate(&doctrine, h, &approved, &corpus, &d)
            .map_err(|e| refused(e.to_string()))?
            .plan;
        let id = plan.id.to_hex();
        let file = state.plans_dir().join(format!("{id}.sealed.json"));
        let commit = if let Some(c) = existing_commit(&file, &seal, &id) {
            c
        } else {
            let (sealed, _card) = seal_plan(&seal, &plan).map_err(|e| refused(e.to_string()))?;
            let bytes = serde_json::to_vec(&SealedPlanFile::from_sealed(&sealed))
                .map_err(|e| refused(e.to_string()))?;
            write_if_changed(&file, &bytes, 0o600)?;
            plan_commit(&sealed)
        };
        plans.push(PreparedPlan {
            plan_id: id,
            plan_commit: commit,
            file,
            promise: h.promise().id.to_hex(),
            cognition: h.cognition().id.to_hex(),
        });
    }
    for f in state.sealed_plan_files()? {
        if !plans.iter().any(|p| p.file == f) {
            fs::remove_file(&f).map_err(io_at(&f))?;
        }
    }
    Ok(Prepared {
        plans,
        refused: refusals
            .into_iter()
            .map(|(id, e)| (id.to_hex(), e.to_string()))
            .collect(),
    })
}

/// What the serving agent reports once it is ready.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Serving {
    /// The plans served.
    pub plans: Vec<PreparedPlan>,
    /// The cognitions refused (no owner approval, or not the owner's).
    pub refused: Vec<(String, String)>,
    /// The agent's own (evaluator) socket.
    pub evaluator_socket: PathBuf,
    /// The host's (implementer) socket.
    pub implementer_socket: PathBuf,
    /// The SHA-256 of the agent's constitution.
    pub constitution_sha256: String,
    /// The commitments made before any commit they will open.
    pub armed: Vec<Armed>,
}

/// Run the agent until its listener ends (see the module docs). `ready` is
/// called once both sockets are served and every plan is armed.
///
/// # Errors
/// [`Error`] when the constitution, the configs, the plans or the server
/// refuse.
pub fn serve(state: &State, ready: impl FnOnce(&Serving)) -> Result<(), Error> {
    let agent = state.agent_config()?;
    let constitution_path = agent.host.join(CONSTITUTION_FILE);
    let constitution = fs::read(&constitution_path).map_err(io_at(&constitution_path))?;
    if sha256_hex(&constitution) != agent.constitution_sha256 {
        return Err(refused(
            "the host's agent constitution is not the one this agent was embedded with",
        ));
    }
    let prepared = prepare_plans(state)?;
    let start = |e: holden_mcp::StartError| refused(e.to_string());
    let evaluator_cfg = Arc::new(
        ServerConfig::load(&state.server_config(Role::Evaluator))
            .map_err(|e| refused(e.to_string()))?,
    );
    let implementer_cfg = Arc::new(
        ServerConfig::load(&state.server_config(Role::Implementer))
            .map_err(|e| refused(e.to_string()))?,
    );
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| refused(format!("runtime: {e}")))?;
    rt.block_on(async {
        let core =
            Arc::new(Core::open(&evaluator_cfg, Clock::System, Log::stderr()).map_err(start)?);
        let own = serve_socket(Arc::clone(&core), Arc::clone(&evaluator_cfg)).map_err(start)?;
        let host = serve_socket(Arc::clone(&core), Arc::clone(&implementer_cfg)).map_err(start)?;
        let head = HostGit::new(&agent.host, &state.git_home())
            .run(&["rev-parse", "--verify", "--quiet", "HEAD"])
            .unwrap_or_else(|_| "none".to_owned());
        let caller = Caller {
            role: Some(Role::Evaluator),
            uid: Some(holden_seal::keystore::current_uid()),
            via: Via::Socket,
        };
        let mut armed = Vec::new();
        for p in &prepared.plans {
            let v = core
                .commit_nonce(&caller, &hex::encode(p.plan_commit.0))
                .map_err(|r| refused(r.to_string()))?;
            armed.push(Armed {
                plan_commit: hex::encode(p.plan_commit.0),
                commitment: v["commitment"].as_str().unwrap_or_default().to_owned(),
                committed_at_ms: v["committed_at_ms"].as_u64().unwrap_or_default(),
                head: head.clone(),
                consumed_by: None,
            });
        }
        // A fresh server holds only these commitments.
        state.write_armed(&armed)?;
        ready(&Serving {
            plans: prepared.plans.clone(),
            refused: prepared.refused.clone(),
            evaluator_socket: state.socket(Role::Evaluator),
            implementer_socket: state.socket(Role::Implementer),
            constitution_sha256: agent.constitution_sha256.clone(),
            armed,
        });
        own.wait().await;
        host.stop();
        Ok(())
    })
}
