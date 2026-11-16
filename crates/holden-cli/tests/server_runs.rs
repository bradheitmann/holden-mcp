//! Slice-14 P2: a server-composed W2 run is a first-class CLI/audit run.
//! The fixture, owner approval, agent, evaluator and transports are real.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::fs::{self, DirBuilder};
    #[cfg(target_os = "linux")]
    use std::io::Write;
    use std::io::{BufRead, BufReader};
    use std::os::unix::fs::DirBuilderExt;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Output, Stdio};
    use std::sync::OnceLock;
    use std::time::Duration;

    use holden_eval::ServerReveal;
    use holden_seal::{FileKeyStore, PlanCommit};
    use rmcp::model::CallToolRequestParams;
    use rmcp::{RoleClient, ServiceExt};
    use serde_json::{Value, json};

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
        let status = Command::new(env!("CARGO"))
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
            let forge_target = target.join("server-run-forge-build");
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

    fn text(output: &Output) -> String {
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
        agent: Option<Child>,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("hm14-cli-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)
                .unwrap();
            let root = root.canonicalize().unwrap();
            let host = root.join("host");
            let out = Command::new(&bins().holden)
                .args(["demo-host", "--workspace"])
                .arg(workspace())
                .arg(&host)
                .output()
                .unwrap();
            assert!(out.status.success(), "demo-host: {}", text(&out));
            fs::write(
                host.join("PLANNING.md"),
                "# Pocket Notes\n\nDomain: Pocket Notes server-run fixture\n\n## Promises\n\n- PROMISE: Deleting a note asks for confirmation before anything is removed.\n",
            )
            .unwrap();
            let fx = Self {
                state: root.join("state"),
                owner_store: root.join("owner"),
                root,
                host,
                agent: None,
            };
            let denylist = fx.root.join("denylist.txt");
            fs::write(&denylist, "synthetic-server-run-private-token\n").unwrap();
            let b = bins();
            let out = fx.holden(&[
                "embed",
                fx.host.to_str().unwrap(),
                "--state",
                fx.state.to_str().unwrap(),
                "--forge",
                b.forge.to_str().unwrap(),
                "--denylist",
                denylist.to_str().unwrap(),
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

        fn holden(&self, args: &[&str]) -> Output {
            let mut command = Command::new(&bins().holden);
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
                let mut child = Command::new("/usr/bin/script")
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
                    .write_all(b"throwaway-server-run-owner-passphrase\nthrowaway-server-run-owner-passphrase\n")
                    .unwrap();
                let out = child.wait_with_output().unwrap();
                assert!(out.status.success(), "approve: {}", text(&out));
            }
            #[cfg(target_os = "macos")]
            {
                let out = Command::new("/usr/bin/script")
                    .args(["-q", "/dev/null"])
                    .arg(&bins().holden)
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
            let mut child = Command::new(&bins().holden)
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
                    if line.is_ok_and(|line| line.starts_with("holden agent: ready")) {
                        let _ = tx.send(());
                        break;
                    }
                }
            });
            self.agent = Some(child);
            rx.recv_timeout(Duration::from_secs(300))
                .expect("agent ready");
        }

        fn break_and_commit(&self) -> String {
            fs::copy(
                workspace().join("fixtures/products/notes/variants/delete-noop/src/main.rs"),
                self.host.join("src/main.rs"),
            )
            .unwrap();
            let home = self.root.join("commit-home");
            fs::create_dir_all(&home).unwrap();
            for args in [
                vec!["add", "--all"],
                vec!["commit", "--quiet", "-m", "server-run client change"],
            ] {
                let out = Command::new("git")
                    .env_clear()
                    .env("PATH", "/usr/bin:/bin")
                    .env("HOME", &home)
                    .env("GIT_CONFIG_NOSYSTEM", "1")
                    .env("GIT_CONFIG_GLOBAL", "/dev/null")
                    .args(["-C", self.host.to_str().unwrap()])
                    .args([
                        "-c",
                        "core.hooksPath=/dev/null",
                        "-c",
                        "core.fsmonitor=false",
                        "-c",
                        "user.name=server-run-test",
                        "-c",
                        "user.email=server-run@example.invalid",
                    ])
                    .args(args)
                    .output()
                    .unwrap();
                assert!(out.status.success(), "git: {}", text(&out));
            }
            let out = Command::new("git")
                .args(["-C", self.host.to_str().unwrap(), "rev-parse", "HEAD"])
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_owned()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            if let Some(mut child) = self.agent.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    async fn call(
        client: &rmcp::service::RunningService<RoleClient, ()>,
        tool: &str,
        args: Value,
    ) -> Value {
        let params = CallToolRequestParams::new(tool.to_owned())
            .with_arguments(serde_json::from_value(args).unwrap());
        let result = tokio::time::timeout(Duration::from_secs(300), client.call_tool(params))
            .await
            .expect("MCP call deadline")
            .unwrap();
        let body = result.structured_content.clone().unwrap_or(Value::Null);
        assert!(!result.is_error.unwrap_or(false), "{tool}: {body}");
        body
    }

    /// Fail-without: do not append the W2 verdict to the signed ledger; the
    /// real `holden trace` then refuses its missing ledger link.
    #[tokio::test]
    async fn server_w2_run_traces_replays_and_reveals() {
        let mut fx = Fixture::new();
        fx.approve();
        fx.start_agent();
        let commit = fx.break_and_commit();
        let socket = fx.state.join("se").join(holden_mcp::config::SOCKET_FILE);
        let stream = tokio::time::timeout(
            Duration::from_secs(30),
            tokio::net::UnixStream::connect(socket),
        )
        .await
        .unwrap()
        .unwrap();
        let client: rmcp::service::RunningService<RoleClient, ()> =
            tokio::time::timeout(Duration::from_secs(30), ().serve(stream))
                .await
                .unwrap()
                .unwrap();
        let check = call(&client, "holden_check", json!({"host": fx.host})).await;
        let run = check["runs"][0]["run_id"].as_str().unwrap();

        let traced = fx.holden(&["trace", run, "--host", fx.host.to_str().unwrap()]);
        assert!(traced.status.success(), "trace: {}", text(&traced));
        assert!(text(&traced).contains("verified: every link of the chain holds"));
        let replayed = fx.holden(&["replay", run, "--host", fx.host.to_str().unwrap()]);
        assert!(replayed.status.success(), "replay: {}", text(&replayed));
        assert!(text(&replayed).contains(&format!("evaluated_commit: {commit}")));

        let brief = call(&client, "brief", json!({})).await;
        let audit = call(&client, "audit_reveal", json!({"run_id": run})).await;
        let plan_commit = PlanCommit(
            <[u8; 32]>::try_from(
                hex::decode(check["runs"][0]["plan_commit"].as_str().unwrap()).unwrap(),
            )
            .unwrap(),
        );
        let reveal = ServerReveal::from_reveal_body(
            &holden_embed::jit::reveal_part(&audit),
            brief["epoch_public_key"].as_str().unwrap(),
            &plan_commit,
            &commit,
        )
        .unwrap();
        let record: holden_eval::VerdictRecord = serde_json::from_slice(
            &fs::read(fx.state.join("runs").join(run).join("record.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(reveal.wire(), &record.body.reveal);
        tokio::time::timeout(Duration::from_secs(30), client.cancel())
            .await
            .unwrap()
            .unwrap();
    }
}
