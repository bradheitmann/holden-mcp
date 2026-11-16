//! slice-8: discovery, composition, prompts, owner boundaries and real-client
//! W1→W2→W3→W4 runs over Unix sockets and stdio against the notes host.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs::{self, DirBuilder};
    use std::future::Future;
    #[cfg(target_os = "linux")]
    use std::io::Write;
    use std::io::{BufRead, BufReader};
    use std::os::unix::fs::DirBuilderExt;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Stdio};
    use std::sync::{Arc, Mutex, OnceLock};
    use std::time::Duration;

    use holden_mcp::workflow::WORKFLOW_TOOLS;
    use holden_mcp::{Clock, Core, Log, Role, ServerConfig, TOOLS, serve_socket};
    use holden_seal::{FileKeyStore, KeyMaterial, KeyName, KeyStore};
    use rmcp::handler::client::ClientHandler;
    use rmcp::model::{
        CallToolRequestParams, GetPromptRequestParams, JsonObject, NumberOrString,
        ProgressNotificationParam, ProgressToken, RequestMetaObject,
    };
    use rmcp::service::{NotificationContext, RunningService};
    use rmcp::transport::TokioChildProcess;
    use rmcp::{RoleClient, ServiceExt};
    use serde_json::{Value, json};

    #[derive(Clone, Default)]
    struct ProgressLog {
        observed: Arc<Mutex<Vec<ProgressNotificationParam>>>,
    }

    impl ClientHandler for ProgressLog {
        fn on_progress(
            &self,
            params: ProgressNotificationParam,
            _context: NotificationContext<RoleClient>,
        ) -> impl Future<Output = ()> + Send + '_ {
            self.observed.lock().unwrap().push(params);
            std::future::ready(())
        }
    }

    type Client = RunningService<RoleClient, ProgressLog>;

    const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(30);

    async fn exchange<T>(transport: &str, step: &str, future: impl Future<Output = T>) -> T {
        tokio::time::timeout(EXCHANGE_TIMEOUT, future)
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "MCP exchange deadline exceeded after {}s: transport={transport} step={step}",
                    EXCHANGE_TIMEOUT.as_secs()
                )
            })
    }

    async fn cancel(client: Client, transport: &str) {
        exchange(transport, "cancel", client.cancel())
            .await
            .unwrap_or_else(|error| {
                panic!("MCP cancel failed: transport={transport} error={error}")
            });
    }

    fn workspace() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    struct Bins {
        holden: PathBuf,
        confine: PathBuf,
        forge: PathBuf,
    }

    fn build(args: &[&str], target: &Path) {
        let status = std::process::Command::new(env!("CARGO"))
            .args(args)
            .env("CARGO_TARGET_DIR", target)
            .status()
            .unwrap();
        assert!(status.success(), "cargo {args:?} failed");
    }

    fn bins() -> &'static Bins {
        static BINS: OnceLock<Bins> = OnceLock::new();
        BINS.get_or_init(|| {
            let exe = std::env::current_exe().unwrap();
            let profile = exe.parent().and_then(Path::parent).unwrap();
            let target = profile.parent().unwrap();
            let manifest = workspace().join("Cargo.toml");
            for (package, binary) in [("holden-cli", "holden"), ("holden-eval", "holden-confine")] {
                build(
                    &[
                        "build",
                        "--locked",
                        "--quiet",
                        "-p",
                        package,
                        "--bin",
                        binary,
                        "--manifest-path",
                        manifest.to_str().unwrap(),
                    ],
                    target,
                );
            }
            let forge_target = target.join("workflow-forge-build");
            build(
                &[
                    "build",
                    "--offline",
                    "--locked",
                    "--quiet",
                    "--manifest-path",
                    workspace()
                        .join("modules/holden-forge/Cargo.toml")
                        .to_str()
                        .unwrap(),
                ],
                &forge_target,
            );
            let bins = Bins {
                holden: profile.join("holden"),
                confine: profile.join("holden-confine"),
                forge: forge_target.join("debug/forge"),
            };
            for path in [&bins.holden, &bins.confine, &bins.forge] {
                assert!(path.is_file(), "{}", path.display());
            }
            bins
        })
    }

    fn text(output: &std::process::Output) -> String {
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    struct Fixture {
        root: PathBuf,
        host: PathBuf,
        state: PathBuf,
        owner_store: PathBuf,
        denylist: PathBuf,
        agent: Option<Child>,
    }

    impl Fixture {
        fn new(tag: &str) -> Self {
            let root = PathBuf::from("/tmp").join(format!("hm8-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)
                .unwrap();
            let root = root.canonicalize().unwrap();
            let host = root.join("host");
            let out = std::process::Command::new(&bins().holden)
                .args(["demo-host", "--workspace"])
                .arg(workspace())
                .arg(&host)
                .output()
                .unwrap();
            assert!(out.status.success(), "demo-host: {}", text(&out));
            // One real promise makes each full workflow proof bounded. The
            // delete-noop source is a real committed fixture broken variant.
            fs::write(
                host.join("PLANNING.md"),
                "# Pocket Notes\n\nDomain: Pocket Notes workflow fixture\n\n## Promises\n\n- PROMISE: Deleting a note asks for confirmation before anything is removed.\n",
            )
            .unwrap();
            let denylist = root.join("denylist.txt");
            fs::write(&denylist, "synthetic-workflow-private-token\n").unwrap();
            let fx = Self {
                state: root.join("state"),
                owner_store: root.join("owner"),
                root,
                host,
                denylist,
                agent: None,
            };
            let b = bins();
            let out = fx.holden(&[
                "embed",
                fx.host.to_str().unwrap(),
                "--state",
                fx.state.to_str().unwrap(),
                "--forge",
                b.forge.to_str().unwrap(),
                "--denylist",
                fx.denylist.to_str().unwrap(),
                "--lens",
                workspace()
                    .join("crates/holden-cognition/fixtures/project-synthetic/lens")
                    .to_str()
                    .unwrap(),
                "--cargo",
                env!("CARGO"),
                "--confine-helper",
                b.confine.to_str().unwrap(),
                "--owner-store",
                fx.owner_store.to_str().unwrap(),
            ]);
            assert!(out.status.success(), "embed: {}", text(&out));
            fx
        }

        fn holden(&self, args: &[&str]) -> std::process::Output {
            let mut command = std::process::Command::new(&bins().holden);
            command
                .args(args)
                .current_dir(&self.host)
                .env("AGENT_PRIVATE_ROOT", self.root.join("agent-private"));
            #[cfg(target_os = "macos")]
            command.env(
                "HOLDEN_TEST_OWNER_KEYCHAIN_PASSWORD",
                "throwaway-test-keychain-password",
            );
            command.output().unwrap()
        }

        fn approve(&self) {
            #[cfg(target_os = "macos")]
            {
                FileKeyStore::open(&self.owner_store).unwrap();
                let path = self.owner_store.join("owner-signing.keychain");
                holden_seal::KeychainKeyStore::create(&path, "throwaway-test-keychain-password")
                    .unwrap();
            }
            #[cfg(target_os = "linux")]
            {
                FileKeyStore::open(&self.owner_store).unwrap();
                let invocation = format!(
                    "{} approve --host {} --owner-store {} --create-owner-key",
                    bins().holden.display(),
                    self.host.display(),
                    self.owner_store.display(),
                );
                let mut child = std::process::Command::new("/usr/bin/script")
                    .args(["-q", "-e", "-c", &invocation, "/dev/null"])
                    .current_dir(&self.host)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap();
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(b"throwaway-workflow-owner-passphrase\nthrowaway-workflow-owner-passphrase\n")
                    .unwrap();
                let out = child.wait_with_output().unwrap();
                assert!(out.status.success(), "approve: {}", text(&out));
            }
            #[cfg(target_os = "macos")]
            let mut command = {
                let mut command = std::process::Command::new("/usr/bin/script");
                command.args(["-q", "/dev/null"]).arg(&bins().holden);
                command
            };
            #[cfg(target_os = "macos")]
            {
                let out = command
                    .args([
                        "approve",
                        "--host",
                        self.host.to_str().unwrap(),
                        "--owner-store",
                        self.owner_store.to_str().unwrap(),
                        "--create-owner-key",
                    ])
                    .current_dir(&self.host)
                    .env(
                        "HOLDEN_TEST_OWNER_KEYCHAIN_PASSWORD",
                        "throwaway-test-keychain-password",
                    )
                    .output()
                    .unwrap();
                assert!(out.status.success(), "approve: {}", text(&out));
            }
        }

        fn start_agent(&mut self) {
            let mut child = std::process::Command::new(&bins().holden)
                .args(["agent", "serve", "--state", self.state.to_str().unwrap()])
                .current_dir(&self.host)
                .env("AGENT_PRIVATE_ROOT", self.root.join("agent-private"))
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
            let stdout = child.stdout.take().unwrap();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    if line.is_ok_and(|l| l.starts_with("holden agent: ready")) {
                        let _ = tx.send(());
                        break;
                    }
                }
            });
            self.agent = Some(child);
            rx.recv_timeout(Duration::from_secs(300))
                .expect("agent ready");
        }

        fn stop_agent(&mut self) {
            if let Some(mut child) = self.agent.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }

        fn socket(&self, role: Role) -> PathBuf {
            self.state
                .join(match role {
                    Role::Evaluator => "se",
                    Role::Implementer => "si",
                })
                .join(holden_mcp::config::SOCKET_FILE)
        }

        fn break_and_commit(&self) -> String {
            fs::copy(
                workspace().join("fixtures/products/notes/variants/delete-noop/src/main.rs"),
                self.host.join("src/main.rs"),
            )
            .unwrap();
            let home = self.root.join("commit-home");
            fs::create_dir_all(&home).unwrap();
            let out = std::process::Command::new("git")
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", home)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .args(["-C", self.host.to_str().unwrap()])
                .args([
                    "-c",
                    "core.hooksPath=/dev/null",
                    "-c",
                    "core.fsmonitor=false",
                ])
                .args([
                    "-c",
                    "user.name=workflow-test",
                    "-c",
                    "user.email=workflow@example.invalid",
                ])
                .args(["add", "--all"])
                .output()
                .unwrap();
            assert!(out.status.success(), "git add: {}", text(&out));
            let out = std::process::Command::new("git")
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.root.join("commit-home"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .args(["-C", self.host.to_str().unwrap()])
                .args([
                    "-c",
                    "core.hooksPath=/dev/null",
                    "-c",
                    "core.fsmonitor=false",
                ])
                .args([
                    "-c",
                    "user.name=workflow-test",
                    "-c",
                    "user.email=workflow@example.invalid",
                ])
                .args(["commit", "--quiet", "-m", "broken deletion"])
                .output()
                .unwrap();
            assert!(out.status.success(), "git commit: {}", text(&out));
            let out = std::process::Command::new("git")
                .args(["-C", self.host.to_str().unwrap(), "rev-parse", "HEAD"])
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_owned()
        }

        fn git(&self, args: &[&str]) -> std::process::Output {
            std::process::Command::new("git")
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.root.join("commit-home"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .args(["-C", self.host.to_str().unwrap()])
                .args([
                    "-c",
                    "core.hooksPath=/dev/null",
                    "-c",
                    "core.fsmonitor=false",
                    "-c",
                    "user.name=workflow-test",
                    "-c",
                    "user.email=workflow@example.invalid",
                ])
                .args(args)
                .output()
                .unwrap()
        }

        fn head(&self) -> String {
            let out = self.git(&["rev-parse", "HEAD"]);
            assert!(out.status.success(), "HEAD: {}", text(&out));
            String::from_utf8_lossy(&out.stdout).trim().to_owned()
        }

        fn history(&self) -> String {
            let out = self.git(&["rev-list", "--all"]);
            assert!(out.status.success(), "history: {}", text(&out));
            String::from_utf8_lossy(&out.stdout).into_owned()
        }

        fn empty_commit(&self, message: &str) -> String {
            let out = self.git(&["commit", "--quiet", "--allow-empty", "-m", message]);
            assert!(out.status.success(), "empty commit: {}", text(&out));
            self.head()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            self.stop_agent();
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    async fn socket_client(fx: &Fixture) -> Client {
        let stream = exchange(
            "socket",
            "connect",
            tokio::net::UnixStream::connect(fx.socket(Role::Evaluator)),
        )
        .await
        .unwrap();
        exchange("socket", "initialize", ProgressLog::default().serve(stream))
            .await
            .unwrap()
    }

    async fn stdio_client(fx: &Fixture) -> Client {
        let mut command = tokio::process::Command::new(&bins().holden);
        command
            .args(["connect", "--role", "evaluator", "--socket"])
            .arg(fx.socket(Role::Evaluator))
            .current_dir(&fx.host)
            .kill_on_drop(true);
        let (transport, _) = TokioChildProcess::builder(command)
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        exchange(
            "stdio",
            "initialize",
            ProgressLog::default().serve(transport),
        )
        .await
        .unwrap()
    }

    async fn call(client: &Client, transport: &str, tool: &str, args: Value) -> Value {
        let mut params = CallToolRequestParams::new(tool.to_owned());
        if let Value::Object(map) = args {
            params = params.with_arguments(map);
        }
        let step = format!("call_tool:{tool}");
        let result = exchange(transport, &step, client.call_tool(params))
            .await
            .unwrap_or_else(|error| {
                panic!("MCP tool call failed: transport={transport} step={step} error={error}")
            });
        let body = result.structured_content.clone().unwrap_or(Value::Null);
        assert!(
            !result.is_error.unwrap_or(false),
            "{tool} refused: {}",
            serde_json::to_string(&body).unwrap()
        );
        body
    }

    async fn drive_workflows(
        client: &Client,
        transport: &str,
        fx: &Fixture,
        commit: &str,
    ) -> (Value, Value, Value) {
        let host = fx.host.to_str().unwrap();
        let onboard = call(client, transport, "holden_onboard", json!({"host": host})).await;
        assert_eq!(onboard["status"], "OWNER_APPROVED");
        assert_eq!(onboard["proposals"].as_array().unwrap().len(), 1);
        assert_eq!(onboard["proposals"][0]["approved"], true);

        let check = call(client, transport, "holden_check", json!({"host": host})).await;
        assert_eq!(check["status"], "CHECK_COMPLETE");
        assert_eq!(check["evaluated_commit"], commit);
        assert_eq!(check["runs"].as_array().unwrap().len(), 1);
        assert_eq!(check["runs"][0]["decision"], "fail");
        let run_id = check["runs"][0]["run_id"].as_str().unwrap();

        let explain = call(
            client,
            transport,
            "holden_explain",
            json!({"host": host, "run_id": run_id}),
        )
        .await;
        assert_eq!(explain["status"], "FIX_REQUIRED");
        assert_eq!(explain["run_id"], run_id);
        assert_eq!(explain["decision"], "fail");
        assert_eq!(
            explain["promise"],
            "Deleting a note asks for confirmation before anything is removed."
        );
        assert!(!explain["fixes"].as_array().unwrap().is_empty());
        let keys: Vec<&String> = explain.as_object().unwrap().keys().collect();
        assert_eq!(
            keys,
            [
                "decision", "fixes", "next", "promise", "run_id", "status", "workflow"
            ]
        );
        let leaked = serde_json::to_string(&explain).unwrap();
        for forbidden in [
            "params",
            "proof",
            "seed",
            "server_nonce",
            "threshold",
            "step_order",
        ] {
            assert!(!leaked.contains(forbidden), "W3 leaked {forbidden}");
        }

        let status = call(client, transport, "holden_status", json!({"host": host})).await;
        assert_eq!(status["status"], "OWNER_ACTIONS_CLEAR");
        assert!(status["authority_bars"]["block"].is_object());
        assert!(status["telemetry"].is_object());
        assert_eq!(status["pending_owner_actions"], json!([]));
        (check, explain, status)
    }

    async fn e2e(stdio: bool) {
        let mut fx = Fixture::new(if stdio { "stdio" } else { "socket" });
        fx.start_agent();
        let client = if stdio {
            stdio_client(&fx).await
        } else {
            socket_client(&fx).await
        };
        let transport = if stdio { "stdio" } else { "socket" };
        let before = call(
            &client,
            transport,
            "holden_onboard",
            json!({"host": fx.host}),
        )
        .await;
        assert_eq!(before["status"], "OWNER_ACTION_REQUIRED");
        cancel(client, transport).await;
        fx.stop_agent();
        fx.approve();
        fx.start_agent();
        let commit = fx.break_and_commit();
        let client = if stdio {
            stdio_client(&fx).await
        } else {
            socket_client(&fx).await
        };
        let (check, _, _) = drive_workflows(&client, transport, &fx, &commit).await;
        let run_id = check["runs"][0]["run_id"].as_str().unwrap();
        let audit = call(
            &client,
            transport,
            "audit_reveal",
            json!({"run_id": run_id}),
        )
        .await;
        assert_eq!(audit["verdict"]["decision"], "fail");
        cancel(client, transport).await;
    }

    // ------------------------------------------------------------- P1 / P3 / P4

    struct Mini {
        root: PathBuf,
        host: PathBuf,
        listener: Option<holden_mcp::Listener>,
    }

    impl Mini {
        fn new(tag: &str) -> Self {
            let root = PathBuf::from("/tmp").join(format!("hm8-mini-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)
                .unwrap();
            let root = root.canonicalize().unwrap();
            let host = root.join("host");
            fs::create_dir_all(host.join(".holden/cognitions")).unwrap();
            fs::write(
                host.join(".holden/config.json"),
                json!({
                    "format": "holden.embed/1", "host": "host", "domain": "synthetic",
                    "planning": "PLANNING.md", "planning_sha256": "00", "state": root.join("state"),
                    "implementer_socket": root.join("none"),
                    "agent": {"constitution": ".holden/agents/holden.md", "constitution_sha256": "00", "role": "evaluator", "process": "holden agent serve"},
                    "promises": [{"id": "01", "text": "The owner keeps the notes."}],
                    "cognitions": ["02"],
                }).to_string(),
            )
            .unwrap();
            let store = FileKeyStore::open(root.join("keys")).unwrap();
            for name in ["epoch", "seal"] {
                store
                    .put(
                        &KeyName::new(name).unwrap(),
                        &KeyMaterial::from_bytes([41; 32]),
                    )
                    .unwrap();
            }
            fs::create_dir_all(root.join("plans")).unwrap();
            Self {
                root,
                host,
                listener: None,
            }
        }

        async fn client(&mut self) -> Client {
            let config = Arc::new(ServerConfig::from_json(&json!({
                "socket_dir": self.root.join("socket"), "key_dir": self.root.join("keys"),
                "epoch_key": "epoch", "seal_key": "seal", "plans_dir": self.root.join("plans"),
                "run_window_ms": 1000, "roles": [{"uid": holden_seal::keystore::current_uid(), "role": "implementer"}],
            }).to_string()).unwrap());
            let path = config.socket_path();
            self.listener = Some(
                serve_socket(
                    Arc::new(Core::open(&config, Clock::System, Log::capture()).unwrap()),
                    config,
                )
                .unwrap(),
            );
            let stream = exchange("socket", "connect", tokio::net::UnixStream::connect(path))
                .await
                .unwrap();
            exchange("socket", "initialize", ProgressLog::default().serve(stream))
                .await
                .unwrap()
        }
    }

    impl Drop for Mini {
        fn drop(&mut self) {
            if let Some(listener) = self.listener.take() {
                listener.stop();
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// P1. Fail-without: remove W2 CHECK from INSTRUCTIONS.
    #[tokio::test]
    async fn instructions_name_workflows() {
        let mut mini = Mini::new("instructions");
        let client = mini.client().await;
        let info = client.peer_info().unwrap();
        let instructions = info.instructions.as_deref().unwrap_or_default();
        assert!(!instructions.is_empty());
        for needle in [
            "W1 ONBOARD",
            "W2 CHECK",
            "W3 EXPLAIN",
            "W4 STATUS",
            "holden_onboard",
            "holden_check",
            "holden_explain",
            "holden_status",
            "stop and ask the owner",
            "begin_run",
            "reveal_params",
            "audit_reveal",
            "SHOULD pass a progress token",
        ] {
            assert!(instructions.contains(needle), "instructions omit {needle}");
        }
        let w1 = instructions.find("W1 ONBOARD").unwrap();
        let w2 = instructions.find("W2 CHECK").unwrap();
        let w3 = instructions.find("W3 EXPLAIN").unwrap();
        let w4 = instructions.find("W4 STATUS").unwrap();
        assert!(
            w1 < w2 && w2 < w3 && w3 < w4,
            "workflow order is not explicit"
        );
        // Fail-without: put an executable owner command such as `holden
        // approve` in INSTRUCTIONS; one of these assertions then fails.
        for forbidden in [
            "holden approve",
            "holden migrate-owner-key",
            "call approve",
            "change bars to",
        ] {
            assert!(
                !instructions.contains(forbidden),
                "instructions enable owner execution: {forbidden}"
            );
        }
        cancel(client, "socket").await;
    }

    /// P1: each workflow has a real object schema and each low-level audit tool remains listed.
    #[tokio::test]
    async fn workflow_tools_declared_with_schemas() {
        let mut mini = Mini::new("schemas");
        let client = mini.client().await;
        let tools = exchange("socket", "list_all_tools", client.list_all_tools())
            .await
            .unwrap();
        for name in WORKFLOW_TOOLS {
            let tool = tools.iter().find(|t| t.name == name).unwrap();
            assert_eq!(tool.input_schema["type"], "object", "{name}");
            assert!(tool.input_schema["properties"].is_object(), "{name}");
            assert!(
                tool.description
                    .as_deref()
                    .unwrap_or_default()
                    .contains(name.trim_start_matches("holden_").to_uppercase().as_str())
            );
        }
        for name in TOOLS {
            assert!(tools.iter().any(|t| t.name == name), "missing {name}");
        }
        cancel(client, "socket").await;
    }

    /// P3. Fail-without: drop W3 EXPLAIN from workflow_prompts.
    #[tokio::test]
    async fn prompts_list_all_four_workflows_with_arguments() {
        let mut mini = Mini::new("prompts");
        let client = mini.client().await;
        let prompts = exchange("socket", "list_all_prompts", client.list_all_prompts())
            .await
            .unwrap();
        assert_eq!(prompts.len(), 4);
        for name in ["W1-ONBOARD", "W2-CHECK", "W3-EXPLAIN", "W4-STATUS"] {
            let prompt = prompts.iter().find(|p| p.name == name).unwrap();
            assert!(prompt.arguments.as_ref().is_some_and(|a| {
                a.iter()
                    .any(|v| v.name == "host" && v.required == Some(true))
            }));
            let mut args = JsonObject::new();
            args.insert("host".to_owned(), json!(mini.host));
            if name == "W3-EXPLAIN" {
                args.insert("run_id".to_owned(), json!("00"));
            }
            let step = format!("get_prompt:{name}");
            let got = exchange(
                "socket",
                &step,
                client.get_prompt(GetPromptRequestParams::new(name).with_arguments(args)),
            )
            .await
            .unwrap();
            assert_eq!(got.messages.len(), 1);
            let rmcp::model::ContentBlock::Text(content) = &got.messages[0].content else {
                panic!("workflow prompt was not text")
            };
            let text = &content.text;
            let expected = match name {
                "W1-ONBOARD" => "holden_onboard",
                "W2-CHECK" => "holden_check",
                "W3-EXPLAIN" => "holden_explain",
                "W4-STATUS" => "holden_status",
                _ => unreachable!(),
            };
            assert!(text.contains(expected), "{name}: {text}");
            for forbidden in [
                "holden approve",
                "migrate-owner-key",
                "call approve",
                "change bars to",
            ] {
                assert!(!text.contains(forbidden), "{name} enables {forbidden}");
            }
        }
        cancel(client, "socket").await;
    }

    /// P4. Fail-without: allow an implementer to reach approval in W1.
    #[tokio::test]
    async fn implementer_owner_step_returns_owner_action_required() {
        let mut mini = Mini::new("owner");
        let client = mini.client().await;
        let body = call(
            &client,
            "socket",
            "holden_onboard",
            json!({"host": mini.host}),
        )
        .await;
        assert_eq!(body["status"], "OWNER_ACTION_REQUIRED");
        assert_eq!(body["caller_role"], Value::Null); // no role escalation is part of the answer
        assert_eq!(
            body["owner_command"],
            format!("holden approve --host {}", mini.host.display())
        );
        assert!(!body["proposals"][0]["approved"].as_bool().unwrap());
        // W2, W3 and W4 (including its authority-bars surface) stop before
        // running, auditing or reporting while an owner action is pending.
        for (tool, arguments) in [
            ("holden_check", json!({"host": mini.host})),
            ("holden_explain", json!({"host": mini.host, "run_id": "00"})),
            ("holden_status", json!({"host": mini.host})),
        ] {
            let pending = call(&client, "socket", tool, arguments).await;
            assert_eq!(
                pending["status"], "OWNER_ACTION_REQUIRED",
                "{tool}: {pending}"
            );
            assert!(pending["authority_bars"].is_null(), "{tool}: {pending}");
        }
        let audit = exchange(
            "socket",
            "call_tool:approve",
            client
                .call_tool(CallToolRequestParams::new("approve").with_arguments(JsonObject::new())),
        )
        .await;
        assert!(
            audit.is_err(),
            "no implementer-executable approval tool exists"
        );
        cancel(client, "socket").await;
    }

    /// P2: composition adds no replacement for the low-level audit surface.
    #[tokio::test]
    async fn low_level_tools_remain_for_audit() {
        let mut mini = Mini::new("audit-tools");
        let client = mini.client().await;
        let listed = exchange("socket", "list_all_tools", client.list_all_tools())
            .await
            .unwrap();
        let actual: Vec<&str> = listed.iter().map(|t| t.name.as_ref()).collect();
        for name in TOOLS {
            assert!(
                actual.contains(&name),
                "low-level audit tool {name} disappeared"
            );
        }
        let brief = call(&client, "socket", "brief", json!({})).await;
        assert_eq!(brief["caller_role"], "implementer");
        assert_eq!(brief["tools"].as_array().unwrap().len(), TOOLS.len());
        cancel(client, "socket").await;
    }

    /// P2. Fail-without: return brief instead of composing begin_run/reveal_params.
    #[tokio::test]
    async fn workflow_tools_compose_low_level_calls() {
        let mut fx = Fixture::new("compose");
        fx.start_agent();
        let client = socket_client(&fx).await;
        let onboard = call(
            &client,
            "socket",
            "holden_onboard",
            json!({"host": fx.host}),
        )
        .await;
        assert_eq!(onboard["status"], "OWNER_ACTION_REQUIRED");
        cancel(client, "socket").await;
        fx.stop_agent();
        fx.approve();
        fx.start_agent();
        let commit = fx.break_and_commit();
        let client = socket_client(&fx).await;
        let check = call(&client, "socket", "holden_check", json!({"host": fx.host})).await;
        assert_eq!(check["evaluated_commit"], commit);
        let run = check["runs"][0]["run_id"].as_str().unwrap();
        let record: holden_eval::VerdictRecord = serde_json::from_slice(
            &fs::read(fx.state.join("runs").join(run).join("record.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(record.body.evaluated_commit, commit);
        assert_eq!(record.body.reveal.run_id, run);
        assert_eq!(record.body.plan_commit, check["runs"][0]["plan_commit"]);
        assert!(!record.body.reveal.proof.is_empty());
        let audit = call(&client, "socket", "audit_reveal", json!({"run_id": run})).await;
        assert_eq!(audit["verdict"]["decision"], "fail");
        assert_eq!(check["runs"][0]["decision"], "fail");
        cancel(client, "socket").await;
    }

    /// Slice-14 P1. Fail-without: have W2 create and evaluate its own
    /// commit; both the commit assertions and the unchanged-history assertion
    /// then fail.
    #[tokio::test]
    async fn w2_check_evaluates_the_commit_under_test() {
        let mut fx = Fixture::new("slice-14-commit");
        fx.approve();
        fx.start_agent();
        let original = fx.head();
        let current = fx.break_and_commit();
        let history = fx.history();
        let client = socket_client(&fx).await;

        let named = call(
            &client,
            "socket",
            "holden_check",
            json!({"host": fx.host, "evaluated_commit": original}),
        )
        .await;
        assert_eq!(named["evaluated_commit"], original);
        let named_run = named["runs"][0]["run_id"].as_str().unwrap();
        let named_record: holden_eval::VerdictRecord = serde_json::from_slice(
            &fs::read(fx.state.join("runs").join(named_run).join("record.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(named_record.body.evaluated_commit, original);
        assert_eq!(fx.head(), current);
        assert_eq!(fx.history(), history);

        let at_head = call(&client, "socket", "holden_check", json!({"host": fx.host})).await;
        assert_eq!(at_head["evaluated_commit"], current);
        let head_run = at_head["runs"][0]["run_id"].as_str().unwrap();
        let head_record: holden_eval::VerdictRecord = serde_json::from_slice(
            &fs::read(fx.state.join("runs").join(head_run).join("record.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(head_record.body.evaluated_commit, current);
        assert_eq!(fx.head(), current);
        assert_eq!(fx.history(), history);
        cancel(client, "socket").await;
    }

    /// Slice-14 P3. Fail-without: omit VERDICT from Measurements::from_entries;
    /// the catch samples and signed weight bases then remain empty.
    #[tokio::test]
    async fn w4_counts_server_side_runs() {
        let mut fx = Fixture::new("slice-14-counts");
        fx.approve();
        fx.start_agent();
        let _ = fx.break_and_commit();
        let client = socket_client(&fx).await;
        let before = call(&client, "socket", "holden_status", json!({"host": fx.host})).await;
        assert_eq!(before["telemetry"]["verdicts"], 0);
        assert_eq!(before["telemetry"]["weights"], Value::Null);

        let first = call(&client, "socket", "holden_check", json!({"host": fx.host})).await;
        assert_eq!(first["status"], "CHECK_COMPLETE");
        let after_first = call(&client, "socket", "holden_status", json!({"host": fx.host})).await;
        assert_eq!(after_first["telemetry"]["verdicts"], 1);
        assert!(
            after_first["telemetry"]["checks"].as_array().is_some_and(
                |checks| !checks.is_empty() && checks.iter().all(|c| c["catch"]["n"] == 1)
            )
        );
        assert!(after_first["telemetry"]["weights"].is_object());

        let second_commit = fx.empty_commit("second measured client change");
        let second = call(&client, "socket", "holden_check", json!({"host": fx.host})).await;
        assert_eq!(second["evaluated_commit"], second_commit);
        let after_second = call(&client, "socket", "holden_status", json!({"host": fx.host})).await;
        assert_eq!(after_second["telemetry"]["verdicts"], 2);
        assert!(
            after_second["telemetry"]["checks"].as_array().is_some_and(
                |checks| !checks.is_empty() && checks.iter().all(|c| c["catch"]["n"] == 2)
            )
        );
        let first_at = after_first["telemetry"]["weights"]["at"].clone();
        let second_at = after_second["telemetry"]["weights"]["at"].clone();
        assert_ne!(first_at, second_at);
        assert!(
            after_second["telemetry"]["weights"]["basis"]
                .as_object()
                .is_some_and(|basis| basis.values().any(|rate| rate["n"] == 2))
        );
        cancel(client, "socket").await;
    }

    /// B4: W2 may trigger an evaluation for an implementer, but its response
    /// has only the public workflow, verdict/diagnostic and aggregate
    /// telemetry fields. Fail-without: add a params field to W2's response.
    #[tokio::test]
    async fn w2_response_carries_no_param_fields() {
        let mut fx = Fixture::new("implementer-leak");
        fx.approve();
        fx.start_agent();
        let commit = fx.break_and_commit();
        let stream = exchange(
            "socket",
            "connect implementer",
            tokio::net::UnixStream::connect(fx.socket(Role::Implementer)),
        )
        .await
        .unwrap();
        let client = exchange(
            "socket",
            "initialize implementer",
            ProgressLog::default().serve(stream),
        )
        .await
        .unwrap();
        let check = call(&client, "socket", "holden_check", json!({"host": fx.host})).await;
        assert_eq!(check["evaluated_commit"], commit);
        assert!(check["telemetry"]["checks"].is_array());
        fn keys(value: &Value) -> BTreeSet<String> {
            value.as_object().unwrap().keys().cloned().collect()
        }
        fn inspect(value: &Value) {
            match value {
                Value::Object(map) => {
                    for (key, value) in map {
                        assert!(
                            ![
                                "params",
                                "seed",
                                "output",
                                "proof",
                                "server_nonce",
                                "builder_nonce",
                                "epoch_public_key",
                                "reveal",
                                "ciphertext",
                                "ledger",
                                "key",
                            ]
                            .contains(&key.as_str()),
                            "W2 exposed {key}: {value}"
                        );
                        inspect(value);
                    }
                }
                Value::Array(items) => {
                    for item in items {
                        inspect(item);
                    }
                }
                _ => {}
            }
        }
        inspect(&check);
        assert_eq!(
            keys(&check),
            [
                "evaluated_commit",
                "next",
                "runs",
                "status",
                "telemetry",
                "workflow"
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
        assert_eq!(
            keys(&check["runs"][0]),
            [
                "authority",
                "decision",
                "diagnostics",
                "plan_commit",
                "promise",
                "run_id"
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
        assert_eq!(
            keys(&check["telemetry"]),
            [
                "authority_bars",
                "checks",
                "cognitions",
                "controls",
                "format",
                "integrity",
                "lenses",
                "plans",
                "verdicts",
                "weights"
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
        cancel(client, "socket").await;
    }

    /// W2 progress: a real holdout emits ordered MCP notifications whose text
    /// names only the public workflow phases. Fail-without: remove the
    /// `peer.notify_progress` emission; the observed sequence is then empty.
    #[tokio::test]
    async fn w2_emits_monotonic_non_leaking_progress() {
        let mut fx = Fixture::new("progress");
        fx.approve();
        fx.start_agent();
        let commit = fx.break_and_commit();
        let stream = exchange(
            "socket",
            "connect progress",
            tokio::net::UnixStream::connect(fx.socket(Role::Evaluator)),
        )
        .await
        .unwrap();
        let log = ProgressLog::default();
        let observed = Arc::clone(&log.observed);
        let client = exchange("socket", "initialize progress", log.serve(stream))
            .await
            .unwrap();
        let token = ProgressToken(NumberOrString::String("workflow-progress".into()));
        let mut params = CallToolRequestParams::new("holden_check")
            .with_arguments(json!({"host": fx.host}).as_object().unwrap().clone());
        params.meta = Some(RequestMetaObject::with_progress_token(token.clone()));
        assert_eq!(
            serde_json::to_value(&params).unwrap()["_meta"]["progressToken"],
            "workflow-progress"
        );
        let result = exchange(
            "socket",
            "call_tool:holden_check:progress",
            client.call_tool(params),
        )
        .await
        .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(
            result.structured_content.unwrap()["evaluated_commit"],
            commit
        );
        let notifications = observed.lock().unwrap().clone();
        assert!(!notifications.is_empty(), "W2 emitted no progress");
        assert!(
            notifications
                .windows(2)
                .all(|pair| pair[0].progress < pair[1].progress),
            "progress counters are not strictly increasing"
        );
        assert!(
            notifications
                .windows(2)
                .all(|pair| pair[0].progress_token == pair[1].progress_token),
            "progress notifications changed request tokens"
        );
        let messages: Vec<&str> = notifications
            .iter()
            .map(|item| item.message.as_deref().unwrap_or_default())
            .collect();
        assert_eq!(
            messages,
            [
                "preparing",
                "plan 1/1: building",
                "plan 1/1: evaluating",
                "plan 1/1: verdict recorded",
                "plan 1/1: audit closed",
                "plan 1/1: ready for next run",
            ]
        );
        for message in messages {
            let lower = message.to_ascii_lowercase();
            for forbidden in [
                "param",
                "seed",
                "nonce",
                "proof",
                "cipher",
                "threshold",
                "step",
                "value",
                "promise",
                "observation",
            ] {
                assert!(
                    !lower.contains(forbidden),
                    "progress leaked {forbidden}: {message}"
                );
            }
        }
        cancel(client, "socket").await;
    }

    /// P5: real rmcp over the agent's evaluator Unix socket.
    #[tokio::test]
    async fn e2e_workflows_over_socket() {
        e2e(false).await;
    }

    /// P5: real rmcp stdio through the byte-for-byte holden connect bridge.
    #[tokio::test]
    async fn e2e_workflows_over_stdio() {
        e2e(true).await;
    }
}
