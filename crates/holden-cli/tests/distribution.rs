//! slice-10 distribution proofs: real files, real child processes and real
//! rmcp stdio clients. Every child exchange has a deadline.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::fs::{self, DirBuilder, Permissions};
    use std::future::Future;
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output, Stdio};
    use std::sync::OnceLock;
    use std::time::Duration;

    use holden_embed::distribution::{SLASH_WORKFLOWS, holden_command, workflow_tool};
    use holden_embed::state::{KEYS, State};
    use holden_mcp::Role;
    use holden_mcp::workflow::WORKFLOW_TOOLS;
    use rmcp::model::CallToolRequestParams;
    use rmcp::service::RunningService;
    use rmcp::transport::TokioChildProcess;
    use rmcp::{RoleClient, ServiceExt};
    use serde_json::{Value, json};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

    const DEADLINE: Duration = Duration::from_secs(30);

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("holden-cli-s10-{name}-{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&dir)
            .unwrap();
        dir.join("private")
    }

    /// A blocking child wait lives on its own OS thread. The deadline path
    /// kills and reaps it, and the test thread only waits on a channel.
    fn wait_bounded(child: std::process::Child, step: &'static str) -> std::process::ExitStatus {
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut child = child;
            let deadline = std::time::Instant::now() + DEADLINE;
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        let _ = sender.send(status);
                        return;
                    }
                    Ok(None) if std::time::Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Ok(None) => {
                        let _ = child.kill();
                        let status = child.wait();
                        if let Ok(status) = status {
                            let _ = sender.send(status);
                        }
                        return;
                    }
                    Err(_) => return,
                }
            }
        });
        receiver
            .recv_timeout(DEADLINE + Duration::from_secs(5))
            .unwrap_or_else(|_| panic!("typed deadline: {step} did not exit and reap"))
    }

    struct ServerGuard(Option<std::process::Child>);

    impl Drop for ServerGuard {
        fn drop(&mut self) {
            let Some(mut child) = self.0.take() else {
                return;
            };
            if child.try_wait().ok().is_some_and(|status| status.is_none()) {
                let _ = child.kill();
            }
            let _ = wait_bounded(child, "server cleanup");
        }
    }

    fn real_server(root: &Path) -> ServerGuard {
        let config = State::new(root)
            .server_config(Role::Implementer)
            .display()
            .to_string();
        let server = Command::new(server_bin())
            .args(["serve", "--config", &config])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let guard = ServerGuard(Some(server));
        let socket = holden_embed::distribution::socket_dir(root).join("holden-mcp.sock");
        let until = std::time::Instant::now() + DEADLINE;
        while !socket.exists() && std::time::Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(socket.exists(), "real server did not become ready");
        guard
    }

    fn holden(args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_holden"))
            .args(args)
            .output()
            .expect("run holden")
    }

    fn init(root: &Path) -> Output {
        holden(&["init", "--root", root.to_str().unwrap()])
    }

    fn key_bytes(root: &Path) -> Vec<Vec<u8>> {
        KEYS.iter()
            .map(|name| fs::read(root.join("keys").join(format!("{name}.key"))).unwrap())
            .collect()
    }

    fn text(output: &Output) -> String {
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    /// P1. Fail-without: replace the existing keys on each init.
    #[test]
    fn init_is_idempotent() {
        let root = scratch("init-idempotent");
        let first = init(&root);
        assert!(first.status.success(), "{}", text(&first));
        let state = State::new(&root);
        assert!(state.server_config(Role::Implementer).is_file());
        let keys = key_bytes(&root);
        assert_eq!(keys.len(), 5);
        let config = fs::read(state.server_config(Role::Implementer)).unwrap();
        let second = init(&root);
        assert!(second.status.success(), "{}", text(&second));
        assert!(!text(&second).contains("keys created"));
        assert_eq!(key_bytes(&root), keys);
        assert_eq!(
            fs::read(state.server_config(Role::Implementer)).unwrap(),
            config
        );
        for key in root.join("keys").read_dir().unwrap() {
            let mode = fs::metadata(key.unwrap().path())
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    /// P1. Fail-without: allow group-readable key files. The same path-mode
    /// guard rejects the private root before any key or config is written.
    #[test]
    fn init_refuses_world_readable_paths() {
        let root = scratch("init-modes");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, Permissions::from_mode(0o750)).unwrap();
        let output = init(&root);
        assert_eq!(output.status.code(), Some(1), "{}", text(&output));
        assert!(text(&output).contains("group- or world-accessible"));
        assert!(!root.join("keys").exists());
        assert!(!root.join("server").exists());

        fs::set_permissions(&root, Permissions::from_mode(0o700)).unwrap();
        assert!(init(&root).status.success());
        let key = root.join("keys/epoch.key");
        fs::set_permissions(&key, Permissions::from_mode(0o640)).unwrap();
        let output = init(&root);
        assert_eq!(output.status.code(), Some(1), "{}", text(&output));
        assert!(text(&output).contains("group- or world-accessible"));
    }

    /// P1. No key material appears in either output stream, including errors.
    #[test]
    fn init_never_prints_secrets() {
        let root = scratch("init-secrets");
        let output = init(&root);
        assert!(output.status.success(), "{}", text(&output));
        let shown = text(&output);
        for key in key_bytes(&root) {
            let hex = hex::encode(&key);
            assert!(!shown.contains(&hex));
            assert!(
                !String::from_utf8_lossy(&key)
                    .chars()
                    .all(|c| c.is_control())
            );
            assert!(!output.stdout.windows(key.len()).any(|w| w == key));
            assert!(!output.stderr.windows(key.len()).any(|w| w == key));
        }
    }

    fn server_bin() -> &'static Path {
        static BIN: OnceLock<PathBuf> = OnceLock::new();
        BIN.get_or_init(|| {
            let exe = std::env::current_exe().unwrap();
            let profile = exe.parent().and_then(Path::parent).unwrap();
            let target = profile.parent().unwrap();
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
            for (package, binary) in [("holden-cli", "holden"), ("holden-mcp", "holden-mcp")] {
                let status = Command::new(env!("CARGO"))
                    .args([
                        "build",
                        "--locked",
                        "--quiet",
                        "-p",
                        package,
                        "--bin",
                        binary,
                        "--manifest-path",
                    ])
                    .arg(&manifest)
                    .env("CARGO_TARGET_DIR", target)
                    .status()
                    .unwrap();
                assert!(status.success());
                assert!(profile.join(binary).is_file());
            }
            let bin = profile.join("holden-mcp");
            assert!(bin.is_file(), "{}", bin.display());
            bin
        })
    }

    async fn bounded<T>(step: &'static str, future: impl Future<Output = T>) -> T {
        tokio::time::timeout(DEADLINE, future)
            .await
            .unwrap_or_else(|_| panic!("typed MCP exchange timeout during {step}"))
    }

    async fn relay_client(root: &Path) -> RunningService<RoleClient, ()> {
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_holden"));
        command
            .arg("mcp")
            .arg("--root")
            .arg(root)
            .env("HOLDEN_MCP_SERVER_BIN", server_bin())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let (transport, _stderr) = TokioChildProcess::builder(command).spawn().unwrap();
        bounded("initialize relay", ().serve(transport))
            .await
            .unwrap()
    }

    /// P2. Fail-without: remove the on-demand spawn. Initialization returns
    /// without a socket and a real stdio initialization creates the server.
    #[tokio::test]
    async fn mcp_stdio_relay_spawns_server_on_demand() {
        let root = scratch("relay-spawn");
        assert!(init(&root).status.success());
        let socket = holden_embed::distribution::socket_dir(&root).join("holden-mcp.sock");
        assert!(!socket.exists());
        let client = relay_client(&root).await;
        assert!(socket.exists(), "the relay started a socket server child");
        let result = bounded(
            "brief through spawned relay",
            client.call_tool(CallToolRequestParams::new("brief").with_arguments(
                serde_json::from_value::<rmcp::model::JsonObject>(json!({})).unwrap(),
            )),
        )
        .await
        .unwrap();
        assert_ne!(result.is_error, Some(true));
        bounded("cancel spawned relay", client.cancel())
            .await
            .unwrap();
    }

    /// P2. Fail-without: serve the evaluator role through the relay.
    #[tokio::test]
    async fn mcp_stdio_relay_carries_implementer_role() {
        let root = scratch("relay-role");
        assert!(init(&root).status.success());
        let client = relay_client(&root).await;
        let result = bounded(
            "role brief",
            client.call_tool(CallToolRequestParams::new("brief").with_arguments(
                serde_json::from_value::<rmcp::model::JsonObject>(json!({})).unwrap(),
            )),
        )
        .await
        .unwrap();
        let brief = result.structured_content.unwrap();
        assert_eq!(brief["caller_role"], "implementer");
        assert!(
            !brief["tools"]
                .as_array()
                .unwrap()
                .contains(&json!("reveal_params"))
        );
        bounded("cancel role relay", client.cancel()).await.unwrap();
    }

    /// EXEC-PM relay intervention: a real client closing stdin must terminate
    /// the relay process. Fail-without: restore the plain Runtime drop in
    /// mcp_cmd::run after block_on(relay), leaving stdin's reader alive.
    #[test]
    fn relay_exits_when_client_closes_stdin() {
        let root = scratch("relay-eof");
        assert!(init(&root).status.success());
        let _server = real_server(&root);
        let mut relay = Command::new(env!("CARGO_BIN_EXE_holden"))
            .args(["mcp", "--root", root.to_str().unwrap()])
            .env("HOLDEN_MCP_SERVER_BIN", server_bin())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        drop(relay.stdin.take());
        let status = wait_bounded(relay, "relay stdin EOF");
        assert!(
            status.success(),
            "relay did not exit cleanly on EOF: {status}"
        );
    }

    async fn write_message(stdin: &mut tokio::process::ChildStdin, message: Value) {
        let mut bytes = serde_json::to_vec(&message).unwrap();
        bytes.push(b'\n');
        bounded("write relay request", stdin.write_all(&bytes))
            .await
            .unwrap();
    }

    async fn read_message(
        lines: &mut tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStdout>>,
    ) -> Value {
        let line = bounded("read relay response", lines.next_line())
            .await
            .unwrap()
            .expect("relay closed stdout during the initial exchange");
        serde_json::from_str(&line).unwrap()
    }

    /// EXEC-PM: the idle path ends a session while the client still holds
    /// stdin open. The initial exchange is real and spawns the socket server
    /// on demand; idle shutdown must kill and reap that child before exit.
    /// Fail-without: restore plain Runtime::drop after block_on(relay). Its
    /// wait on the live stdin reader makes the outer exit deadline fire.
    #[tokio::test]
    async fn relay_idle_deadline_ends_held_stdin_session() {
        let root = scratch("relay-held-stdin-idle");
        assert!(init(&root).status.success());
        let socket = holden_embed::distribution::socket_dir(&root).join("holden-mcp.sock");
        assert!(!socket.exists());
        let mut relay = tokio::process::Command::new(env!("CARGO_BIN_EXE_holden"))
            .args(["mcp", "--root", root.to_str().unwrap()])
            .env("HOLDEN_MCP_SERVER_BIN", server_bin())
            .env("HOLDEN_RELAY_IDLE_DEADLINE_MS", "1000")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut held_stdin = relay.stdin.take().unwrap();
        let stdout = relay.stdout.take().unwrap();
        let mut lines = tokio::io::BufReader::new(stdout).lines();

        write_message(
            &mut held_stdin,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {"name": "held-stdin-test", "version": "0"}
                }
            }),
        )
        .await;
        assert!(read_message(&mut lines).await.get("result").is_some());
        write_message(
            &mut held_stdin,
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        )
        .await;
        write_message(
            &mut held_stdin,
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": "brief", "arguments": {}}
            }),
        )
        .await;
        let brief = read_message(&mut lines).await;
        assert_eq!(brief["id"], 2);
        assert_ne!(brief["result"]["isError"], true);

        // Keep held_stdin alive and write nothing else. The only exit trigger
        // is the relay's idle deadline, not client EOF or cancellation.
        let status = match tokio::time::timeout(DEADLINE, relay.wait()).await {
            Ok(status) => status.unwrap(),
            Err(_) => {
                let _ = relay.kill().await;
                panic!("typed deadline: held-stdin relay did not exit after idle shutdown");
            }
        };
        assert_eq!(status.code(), Some(1), "relay did not report idle expiry");
        let mut stderr = String::new();
        bounded(
            "read held-stdin relay error",
            relay.stderr.take().unwrap().read_to_string(&mut stderr),
        )
        .await
        .unwrap();
        assert!(stderr.contains("relay idle exchange"), "{stderr}");
        assert!(
            tokio::net::UnixStream::connect(socket).await.is_err(),
            "the on-demand server survived relay shutdown"
        );
        drop(held_stdin);
    }

    /// EXEC-PM: idle refresh is shared across both directions. Total session
    /// age deliberately exceeds 120 seconds while no request/response gap
    /// approaches 120s. Fail-without: timeout(60s, exchange) in the relay,
    /// which restores connection-age rather than inactivity semantics.
    #[tokio::test]
    async fn relay_session_survives_long_active_session() {
        let root = scratch("relay-idle-refresh");
        assert!(init(&root).status.success());
        let _server = real_server(&root);
        let client = relay_client(&root).await;
        let started = std::time::Instant::now();
        for _ in 0..70 {
            let result = bounded(
                "active long-session brief",
                client.call_tool(CallToolRequestParams::new("brief").with_arguments(
                    serde_json::from_value::<rmcp::model::JsonObject>(json!({})).unwrap(),
                )),
            )
            .await
            .unwrap();
            assert_ne!(result.is_error, Some(true));
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        let elapsed = started.elapsed();
        assert!(
            elapsed >= Duration::from_secs(130),
            "test did not exceed the old connection age: {elapsed:?}"
        );
        eprintln!("relay active session elapsed: {}ms", elapsed.as_millis());
        bounded("close long active relay", client.cancel())
            .await
            .unwrap();
    }

    /// P3. Fail-without: write a Codex entry (mcp_servers) into Claude's JSON.
    #[test]
    fn embed_configs_validate_per_harness() {
        let host = scratch("harness-formats");
        let files = holden_embed::distribution::write_harness_configs(&host).unwrap();
        assert_eq!(files.len(), 6);
        for (path, prefix) in [
            (host.join(".mcp.json"), "mcpServers"),
            (host.join(".factory/mcp.json"), "mcpServers"),
        ] {
            let value: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
            let entry = &value[prefix]["holden"];
            assert_eq!(entry["command"], "holden");
            assert_eq!(entry["args"], json!(["mcp"]));
            assert!(value.get("mcp_servers").is_none());
        }
        let codex = fs::read_to_string(host.join(".codex/config.toml")).unwrap();
        assert!(codex.contains("[mcp_servers.holden]"));
        assert!(codex.contains("command = \"holden\""));
        assert!(codex.contains("args = [\"mcp\"]"));
        assert!(!codex.contains("mcpServers"));
        for command in [
            host.join(".claude/commands/holden.md"),
            host.join(".codex/commands/holden.md"),
            host.join(".factory/commands/holden.md"),
        ] {
            assert_eq!(fs::read_to_string(command).unwrap(), holden_command());
        }
    }

    /// P3. Fail-without: append another server entry on every embed.
    #[test]
    fn embed_is_idempotent() {
        let host = scratch("harness-idempotent");
        let files = holden_embed::distribution::write_harness_configs(&host).unwrap();
        let first: Vec<Vec<u8>> = files.iter().map(|path| fs::read(path).unwrap()).collect();
        let again = holden_embed::distribution::write_harness_configs(&host).unwrap();
        assert_eq!(files, again);
        let second: Vec<Vec<u8>> = files.iter().map(|path| fs::read(path).unwrap()).collect();
        assert_eq!(first, second);
    }

    /// P4. Fail-without: map /holden approve to the low-level approval tool.
    #[test]
    fn holden_command_maps_to_workflows_only() {
        for (index, subcommand) in SLASH_WORKFLOWS.iter().enumerate() {
            assert_eq!(workflow_tool(subcommand), Some(WORKFLOW_TOOLS[index]));
            assert!(holden_command().contains(WORKFLOW_TOOLS[index]));
        }
        for forbidden in [
            "approve",
            "migrate-owner-key",
            "bars",
            "set-bars",
            "change-bars",
            "owner",
            "reveal_params",
            "begin_run",
            "verdict",
            "audit_reveal",
        ] {
            assert_eq!(workflow_tool(forbidden), None);
        }
        let instructions = holden_command();
        assert!(instructions.contains("print each owner_command verbatim and stop"));
        assert!(instructions.contains("Do not execute it"));
        assert!(!instructions.contains("call approve"));
        assert!(!instructions.contains("call reveal_params"));
    }
}
