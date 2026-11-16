//! The MCP surface (`rmcp`) and its three transports.
//!
//! - Unix socket (the default): the caller's uid comes from the kernel
//!   through tokio's `UnixStream::peer_cred()` (`getpeereid` on macOS,
//!   `SO_PEERCRED` on Linux) and is mapped to a role by the sealed config.
//!   The socket is mode 0600 inside a 0700 directory owned by this user.
//! - Stdio: only when the config enables it; the caller is the evaluator.
//! - Loopback TCP (optional): `127.0.0.1` or `::1` only; callers are
//!   unmapped (brief only), since TCP carries no kernel-verified identity.

use std::fs::{self, DirBuilder, Permissions};
use std::io;
use std::net::SocketAddr;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, GetPromptRequestParams, GetPromptResponse, GetPromptResult, Implementation,
    InitializeResult, ListPromptsResult, PaginatedRequestParams, Prompt, PromptArgument,
    PromptMessage, ServerCapabilities,
};
use rmcp::schemars::JsonSchema;
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::net::{TcpListener, UnixListener};
use tokio::task::JoinHandle;

use crate::config::{ConfigError, Role, ServerConfig};
use crate::ledger::{Caller, Core, Refusal, StartError, Via};
use crate::workflow;

/// Mode of the socket directory.
pub const SOCKET_DIR_MODE: u32 = 0o700;

/// Mode of the socket file.
pub const SOCKET_MODE: u32 = 0o600;

/// `commit_nonce` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct CommitNonceArgs {
    /// The plan commit (64 lowercase hex).
    pub plan_commit: String,
}

/// `begin_run` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct BeginRunArgs {
    /// The plan commit (64 lowercase hex).
    pub plan_commit: String,
    /// The evaluated git commit (40 or 64 lowercase hex).
    pub evaluated_commit: String,
    /// The builder's nonce (64 lowercase hex).
    pub builder_nonce: String,
}

/// `reveal_params` and `audit_reveal` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct RunArgs {
    /// The run id returned by `begin_run` (64 lowercase hex).
    pub run_id: String,
}

/// `submit_observation` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct SubmitObservationArgs {
    /// The run id (64 lowercase hex).
    pub run_id: String,
    /// The observation's id within the run (`[a-z0-9-]`, at most 64).
    pub observation_id: String,
    /// SHA-256 of the observation's canonical bytes (64 lowercase hex).
    pub sha256: String,
    /// Length of the observation's canonical bytes.
    pub len: u64,
}

/// `verdict` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct VerdictArgs {
    /// The run id (64 lowercase hex).
    pub run_id: String,
    /// The signed verdict record, as JSON text.
    pub record: String,
}

/// One MCP session: the shared core and this connection's caller.
#[derive(Clone)]
pub struct Session {
    core: Arc<Core>,
    caller: Caller,
    config: Option<Arc<ServerConfig>>,
    tool_router: ToolRouter<Self>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("caller", &self.caller)
            .finish_non_exhaustive()
    }
}

fn answer(out: Result<Value, Refusal>) -> CallToolResult {
    match out {
        Ok(v) => CallToolResult::structured(v),
        Err(r) => CallToolResult::structured_error(r.to_json()),
    }
}

#[tool_router(router = tool_router)]
impl Session {
    /// A session for `caller` on `core`.
    #[must_use]
    pub fn new(core: Arc<Core>, caller: Caller) -> Self {
        Self {
            core,
            caller,
            config: None,
            tool_router: Self::tool_router(),
        }
    }

    fn with_config(core: Arc<Core>, caller: Caller, config: Arc<ServerConfig>) -> Self {
        Self {
            core,
            caller,
            config: Some(config),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Public facts: the epoch public key, the registered plan commits, the run window, and which tools the caller's role may use. Open to every caller."
    )]
    async fn brief(&self) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::structured(self.core.brief(&self.caller)))
    }

    #[tool(
        description = "Draw a fresh server nonce for a plan and publish its commitment, before the builder commits. Any mapped role."
    )]
    async fn commit_nonce(
        &self,
        Parameters(a): Parameters<CommitNonceArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(answer(self.core.commit_nonce(&self.caller, &a.plan_commit)))
    }

    #[tool(
        description = "Open a run for a plan on an evaluated commit with the builder's nonce, consuming the plan's recorded server-nonce commitment. Evaluator only."
    )]
    async fn begin_run(
        &self,
        Parameters(a): Parameters<BeginRunArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(answer(self.core.begin_run(
            &self.caller,
            &a.plan_commit,
            &a.evaluated_commit,
            &a.builder_nonce,
        )))
    }

    #[tool(
        description = "Reveal a run's moment-of-test parameters with the VRF output and proof. Evaluator only, while the run window is open."
    )]
    async fn reveal_params(
        &self,
        Parameters(a): Parameters<RunArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(answer(self.core.reveal_params(&self.caller, &a.run_id)))
    }

    #[tool(
        description = "Record the digest of one boundary observation for an open run. Evaluator only."
    )]
    async fn submit_observation(
        &self,
        Parameters(a): Parameters<SubmitObservationArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(answer(self.core.submit_observation(
            &self.caller,
            &a.run_id,
            &a.observation_id,
            &a.sha256,
            a.len,
        )))
    }

    #[tool(
        description = "Submit the signed verdict record for an open run. It is accepted only if it replays under the configured evaluator key, belongs to this run and lists exactly the submitted observations; the run then closes. Evaluator only."
    )]
    async fn verdict(
        &self,
        Parameters(a): Parameters<VerdictArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(answer(self.core.verdict(
            &self.caller,
            &a.run_id,
            &a.record,
        )))
    }

    #[tool(
        description = "After a run has closed, return its parameters, VRF proof and record to any mapped role, for verification against the epoch public key."
    )]
    async fn audit_reveal(
        &self,
        Parameters(a): Parameters<RunArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(answer(self.core.audit_reveal(&self.caller, &a.run_id)))
    }

    #[tool(
        description = "Measured effectiveness from the signed ledger: every catch and false-fail rate per check, lens concept, cognition, plan and integrity rule, each with its sample size and 95% Wilson bounds, each check's authority under the configured bars, the lens draw weights, holdout versions and re-plan items. Evaluator only."
    )]
    async fn telemetry(&self) -> Result<CallToolResult, ErrorData> {
        Ok(answer(self.core.telemetry(&self.caller)))
    }

    fn require_workflow_caller(&self) -> Result<(), CallToolResult> {
        if self.caller.role.is_none() {
            Err(CallToolResult::structured_error(
                Refusal::UnmappedCaller.to_json(),
            ))
        } else {
            Ok(())
        }
    }

    #[tool(
        description = "W1 ONBOARD: discover the host's verbatim owner promises and proposed cognitions. Returns OWNER_ACTION_REQUIRED and the exact holden approve command whenever approval is pending; an implementer can never approve."
    )]
    async fn holden_onboard(
        &self,
        Parameters(a): Parameters<workflow::HostArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.require_workflow_caller() {
            return Ok(e);
        }
        Ok(match workflow::onboard(&a.host) {
            Ok(v) => CallToolResult::structured(v),
            Err(e) => CallToolResult::structured_error(e.to_json()),
        })
    }

    #[tool(
        description = "W2 CHECK: run every approved just-in-time sealed holdout on the host's current commit. Composes pre-armed begin_run, reveal_params, confined boundary observations, signed verdict, audit_reveal and the next nonce server-side. Returns only fixed diagnostics."
    )]
    async fn holden_check(
        &self,
        Parameters(a): Parameters<workflow::CheckArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.require_workflow_caller() {
            return Ok(e);
        }
        let Some(config) = self.config.clone() else {
            return Ok(CallToolResult::structured_error(json!({
                "refusal": "workflow_unavailable",
                "message": "this session has no sealed server config",
            })));
        };
        let listener = match serve_internal_socket(Arc::clone(&self.core), config.clone()) {
            Ok(l) => l,
            Err(_) => {
                return Ok(CallToolResult::structured_error(json!({
                    "refusal": "workflow_unavailable",
                    "message": "the internal evaluator socket cannot be opened",
                })));
            }
        };
        let (progress, notifications) = workflow::WorkflowProgress::for_request(&context);
        let socket = PathBuf::from(&listener.endpoint);
        let core = Arc::clone(&self.core);
        let host = a.host;
        let evaluated_commit = a.evaluated_commit;
        let task = tokio::task::spawn_blocking(move || {
            workflow::check_blocking(
                core,
                config,
                host,
                evaluated_commit,
                &socket,
                progress.as_ref(),
            )
        });
        let result = task.await.unwrap_or_else(|_| {
            Err(workflow::WorkflowError::new(
                "could_not_run",
                "the evaluator stopped unexpectedly; this is not a pass",
            ))
        });
        if let Some(mut notifications) = notifications {
            match tokio::time::timeout(Duration::from_secs(5), &mut notifications).await {
                Ok(Ok(())) => {}
                _ => {
                    notifications.abort();
                    let _ = notifications.await;
                    return Ok(CallToolResult::structured_error(json!({
                        "refusal": "could_not_run",
                        "message": "progress notifications did not shut down within their bounded wait; no pass is inferred",
                    })));
                }
            }
        }
        listener.stop();
        let _ = fs::remove_file(PathBuf::from(&listener.endpoint));
        Ok(match result {
            Ok(v) => CallToolResult::structured(v),
            Err(e) => CallToolResult::structured_error(e.to_json()),
        })
    }

    #[tool(
        description = "W3 EXPLAIN: turn a closed failed verdict into the owner's promise and fixed consequences to repair. Composes audit_reveal and signed-record replay; never returns a sealed step, value or threshold."
    )]
    async fn holden_explain(
        &self,
        Parameters(a): Parameters<workflow::ExplainArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.require_workflow_caller() {
            return Ok(e);
        }
        let Some(config) = self.config.clone() else {
            return Ok(CallToolResult::structured_error(json!({
                "refusal": "workflow_unavailable",
                "message": "this session has no sealed server config",
            })));
        };
        Ok(
            match workflow::explain(&self.core, &self.caller, &config, &a.host, &a.run_id) {
                Ok(v) => CallToolResult::structured(v),
                Err(e) => CallToolResult::structured_error(e.to_json()),
            },
        )
    }

    #[tool(
        description = "W4 STATUS: report telemetry, measured authority, owner-set authority bars and pending owner actions. Approval and bars remain owner-only; the workflow returns the exact approval command when needed."
    )]
    async fn holden_status(
        &self,
        Parameters(a): Parameters<workflow::HostArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.require_workflow_caller() {
            return Ok(e);
        }
        let Some(config) = self.config.clone() else {
            return Ok(CallToolResult::structured_error(json!({
                "refusal": "workflow_unavailable",
                "message": "this session has no sealed server config",
            })));
        };
        Ok(
            match workflow::status(&self.core, &self.caller, &config, &a.host) {
                Ok(v) => CallToolResult::structured(v),
                Err(e) => CallToolResult::structured_error(e.to_json()),
            },
        )
    }
}

fn workflow_prompts() -> Vec<Prompt> {
    let host = || {
        PromptArgument::new("host")
            .with_description("Absolute path of the embedded host repository")
            .with_required(true)
    };
    vec![
        Prompt::new(
            "W1-ONBOARD",
            Some(
                "Discover owner promises and proposed cognitions; return the owner approval command",
            ),
            Some(vec![host()]),
        ),
        Prompt::new(
            "W2-CHECK",
            Some("Run approved sealed holdouts on the current commit and return fixed diagnostics"),
            Some(vec![host()]),
        ),
        Prompt::new(
            "W3-EXPLAIN",
            Some(
                "Explain a closed failed verdict by promise and consequence without revealing the sealed how",
            ),
            Some(vec![
                host(),
                PromptArgument::new("run_id")
                    .with_description("Closed run id returned by W2 CHECK")
                    .with_required(true),
            ]),
        ),
        Prompt::new(
            "W4-STATUS",
            Some("Read telemetry, measured authority and pending owner actions"),
            Some(vec![host()]),
        ),
    ]
}

fn prompt_text(name: &str, args: &rmcp::model::JsonObject) -> Option<String> {
    let host = args.get("host")?.as_str()?;
    match name {
        "W1-ONBOARD" => Some(format!(
            "Run W1 ONBOARD: call holden_onboard with host={host:?}. If it returns OWNER_ACTION_REQUIRED, stop and ask the owner to run its exact owner_command; never self-approve. Then continue to W2 CHECK."
        )),
        "W2-CHECK" => Some(format!(
            "Run W2 CHECK: commit the current product change, then call holden_check with host={host:?}. Keep only its fixed diagnostics. For every failed run continue to W3 EXPLAIN; never infer a pass from an error."
        )),
        "W3-EXPLAIN" => {
            let run = args.get("run_id")?.as_str()?;
            Some(format!(
                "Run W3 EXPLAIN: call holden_explain with host={host:?} and run_id={run:?}. Repair the returned promise consequence. Never ask for or disclose sealed steps, values or thresholds. Continue to W4 STATUS."
            ))
        }
        "W4-STATUS" => Some(format!(
            "Run W4 STATUS: call holden_status with host={host:?}. Report telemetry, measured authority and pending owner actions. Stop for the owner for approval or authority-bar changes."
        )),
        _ => None,
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Session {
    fn get_info(&self) -> InitializeResult {
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
        )
        .with_server_info(Implementation::new("holden-mcp", env!("CARGO_PKG_VERSION")))
        .with_instructions(workflow::INSTRUCTIONS)
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListPromptsResult, ErrorData>> + Send + '_ {
        std::future::ready(Ok(ListPromptsResult::with_all_items(workflow_prompts())))
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<GetPromptResponse, ErrorData>> + Send + '_ {
        let empty = rmcp::model::JsonObject::new();
        let args = request.arguments.as_ref().unwrap_or(&empty);
        let result = prompt_text(&request.name, args).map_or_else(
            || {
                Err(ErrorData::invalid_params(
                    "unknown prompt or missing argument",
                    None,
                ))
            },
            |text| {
                Ok(GetPromptResult::new(vec![PromptMessage::new_text(
                    rmcp::model::Role::User,
                    text,
                )])
                .into())
            },
        );
        std::future::ready(result)
    }
}

/// A running listener.
#[derive(Debug)]
pub struct Listener {
    task: JoinHandle<()>,
    /// The socket path (Unix) or bound address (TCP), for display.
    pub endpoint: String,
    /// The bound TCP address, for a TCP listener.
    pub tcp_addr: Option<SocketAddr>,
}

impl Listener {
    /// Stop accepting connections.
    pub fn stop(&self) {
        self.task.abort();
    }

    /// Wait until the accept loop ends.
    pub async fn wait(self) {
        let _ = self.task.await;
    }
}

fn io_err(path: &Path) -> impl FnOnce(io::Error) -> StartError + '_ {
    move |e| StartError::Io {
        path: path.to_path_buf(),
        kind: e.kind(),
    }
}

/// Create the socket directory 0700 if absent; refuse an existing one that is
/// not a directory owned by this user with no group or world bits.
fn prepare_socket_dir(dir: &Path) -> Result<(), StartError> {
    match fs::symlink_metadata(dir) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            DirBuilder::new()
                .recursive(true)
                .mode(SOCKET_DIR_MODE)
                .create(dir)
                .map_err(io_err(dir))?;
            fs::set_permissions(dir, Permissions::from_mode(SOCKET_DIR_MODE))
                .map_err(io_err(dir))?;
        }
        Err(e) => return Err(io_err(dir)(e)),
    }
    let meta = fs::symlink_metadata(dir).map_err(io_err(dir))?;
    let refuse = |why| StartError::InsecureSocketPath {
        path: dir.to_path_buf(),
        why,
    };
    if !meta.file_type().is_dir() {
        return Err(refuse("not a directory"));
    }
    if meta.uid() != holden_seal::keystore::current_uid() {
        return Err(refuse("owned by another user"));
    }
    if meta.mode() & 0o077 != 0 {
        return Err(refuse("group- or world-accessible"));
    }
    Ok(())
}

fn remove_stale_socket(path: &Path) -> Result<(), StartError> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_socket() => fs::remove_file(path).map_err(io_err(path)),
        Ok(_) => Err(StartError::InsecureSocketPath {
            path: path.to_path_buf(),
            why: "exists and is not a socket",
        }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_err(path)(e)),
    }
}

/// Bind the Unix socket from `config` and serve one session per connection,
/// with the role read from the peer's kernel-reported uid.
///
/// # Errors
/// [`StartError`] when the directory or socket is refused or cannot be bound.
pub fn serve_socket(core: Arc<Core>, config: Arc<ServerConfig>) -> Result<Listener, StartError> {
    let dir = config.socket_dir().to_path_buf();
    prepare_socket_dir(&dir)?;
    let path: PathBuf = config.socket_path();
    remove_stale_socket(&path)?;
    let listener = UnixListener::bind(&path).map_err(io_err(&path))?;
    // The directory is 0700, so the window before this chmod admits no one.
    fs::set_permissions(&path, Permissions::from_mode(SOCKET_MODE)).map_err(io_err(&path))?;
    let endpoint = path.display().to_string();
    let task = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let uid = match stream.peer_cred() {
                Ok(cred) => cred.uid(),
                Err(_) => {
                    core.log().line(
                        "holden-mcp: socket: peer credentials unreadable; connection dropped"
                            .to_owned(),
                    );
                    continue;
                }
            };
            let caller = Caller {
                role: config.role_of(uid),
                uid: Some(uid),
                via: Via::Socket,
            };
            let session = Session::with_config(Arc::clone(&core), caller, Arc::clone(&config));
            tokio::spawn(async move {
                if let Ok(running) = session.serve(stream).await {
                    let _ = running.waiting().await;
                }
            });
        }
    });
    Ok(Listener {
        task,
        endpoint,
        tcp_addr: None,
    })
}

/// A short-lived evaluator-only socket used by W2's real internal rmcp client.
/// The containing directory is still 0700 and the socket is 0600; no caller
/// chooses this path or reaches it through the implementer's configuration.
fn serve_internal_socket(
    core: Arc<Core>,
    config: Arc<ServerConfig>,
) -> Result<Listener, StartError> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = config.socket_dir().to_path_buf();
    prepare_socket_dir(&dir)?;
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = dir.join(format!("workflow-{}-{serial}.sock", std::process::id()));
    remove_stale_socket(&path)?;
    let listener = UnixListener::bind(&path).map_err(io_err(&path))?;
    fs::set_permissions(&path, Permissions::from_mode(SOCKET_MODE)).map_err(io_err(&path))?;
    let endpoint = path.display().to_string();
    let task = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let caller = Caller {
                role: Some(Role::Evaluator),
                uid: None,
                via: Via::Socket,
            };
            let session = Session::with_config(Arc::clone(&core), caller, Arc::clone(&config));
            tokio::spawn(async move {
                if let Ok(running) = session.serve(stream).await {
                    let _ = running.waiting().await;
                }
            });
        }
    });
    Ok(Listener {
        task,
        endpoint,
        tcp_addr: None,
    })
}

/// Bind the loopback TCP listener from `config`. Its callers are unmapped.
///
/// # Errors
/// [`StartError`] when TCP is not configured or the bind fails.
pub async fn serve_tcp(core: Arc<Core>, config: Arc<ServerConfig>) -> Result<Listener, StartError> {
    let Some(addr) = config.tcp() else {
        return Err(StartError::Service("tcp is not configured".to_owned()));
    };
    let shown = PathBuf::from(addr.to_string());
    let listener = TcpListener::bind(addr).await.map_err(io_err(&shown))?;
    let bound = listener.local_addr().map_err(io_err(&shown))?;
    let task = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let caller = Caller {
                role: None,
                uid: None,
                via: Via::Tcp,
            };
            let session = Session::with_config(Arc::clone(&core), caller, Arc::clone(&config));
            tokio::spawn(async move {
                if let Ok(running) = session.serve(stream).await {
                    let _ = running.waiting().await;
                }
            });
        }
    });
    Ok(Listener {
        task,
        endpoint: bound.to_string(),
        tcp_addr: Some(bound),
    })
}

/// Serve one evaluator session on this process's stdin and stdout. Refused
/// unless the config enables stdio.
///
/// # Errors
/// [`StartError::Config`] with [`ConfigError::StdioNotEnabled`], or a service failure.
pub async fn serve_stdio(core: Arc<Core>, config: &ServerConfig) -> Result<(), StartError> {
    if !config.stdio_enabled() {
        return Err(ConfigError::StdioNotEnabled.into());
    }
    let caller = Caller {
        role: Some(Role::Evaluator),
        uid: None,
        via: Via::Stdio,
    };
    let session = Session::with_config(core, caller, Arc::new(config.clone()));
    let running = session
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| StartError::Service(e.to_string()))?;
    running
        .waiting()
        .await
        .map_err(|e| StartError::Service(e.to_string()))?;
    Ok(())
}
