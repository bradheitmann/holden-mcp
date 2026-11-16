//! Slice-18: declared non-root Cargo manifests and the public embed contract.
//! Every host is a real git repository containing real buildable crates. W2
//! uses a real owner approval, agent, MCP transport, confined build and signed
//! verdict. Fixture roots and disposable keychains are Drop-guard cleaned.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs::{self, DirBuilder};
    #[cfg(target_os = "linux")]
    use std::io::Write;
    use std::io::{BufRead, BufReader};
    use std::os::unix::fs::DirBuilderExt;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Output, Stdio};
    use std::sync::OnceLock;
    use std::time::Duration;

    use holden_eval::git::Git;
    use holden_seal::FileKeyStore;
    use rmcp::model::CallToolRequestParams;
    use rmcp::{RoleClient, ServiceExt};
    use serde_json::{Value, json};

    const REFUSAL: &str = "refused: unsupported_stack: no root Cargo.toml; declare a crate with --manifest <relative-path> (repeatable)";

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
            let forge_target = target.join("declared-forge-build");
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
            let b = Bins {
                holden: profile.join("holden"),
                confine: profile.join("holden-confine"),
                forge: forge_target.join("debug/forge"),
            };
            for path in [&b.holden, &b.confine, &b.forge] {
                assert!(path.is_file(), "{}", path.display());
            }
            b
        })
    }

    fn text(out: &Output) -> String {
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    }

    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let dest = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &dest);
            } else {
                fs::copy(entry.path(), dest).unwrap();
            }
        }
    }

    struct Fixture {
        root: PathBuf,
        host: PathBuf,
        state: PathBuf,
        owner_store: PathBuf,
        agent: Option<Child>,
    }

    impl Fixture {
        fn new(fixture: &str, tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!("hm18-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)
                .unwrap();
            let root = root.canonicalize().unwrap();
            let host = root.join("host");
            copy_tree(&workspace().join("fixtures/clients").join(fixture), &host);
            let home = root.join("git-home");
            fs::create_dir_all(&home).unwrap();
            let git = Git::new(&host, &home);
            git.init().unwrap();
            git.commit_all("real declared fixture").unwrap();
            let fx = Self {
                state: root.join("s"),
                owner_store: root.join("owner"),
                host,
                root,
                agent: None,
            };
            fs::write(
                fx.root.join("denylist.txt"),
                "zz-slice-eighteen-private-token\n",
            )
            .unwrap();
            fx
        }

        fn embed(&self, manifests: &[&str], explicit_lens: bool) -> Output {
            let b = bins();
            let mut args = vec![
                "embed".to_owned(),
                self.host.to_str().unwrap().to_owned(),
                "--state".to_owned(),
                self.state.to_str().unwrap().to_owned(),
                "--forge".to_owned(),
                b.forge.to_str().unwrap().to_owned(),
                "--denylist".to_owned(),
                self.root.join("denylist.txt").to_str().unwrap().to_owned(),
                "--cargo".to_owned(),
                env!("CARGO").to_owned(),
                "--confine-helper".to_owned(),
                b.confine.to_str().unwrap().to_owned(),
                "--owner-store".to_owned(),
                self.owner_store.to_str().unwrap().to_owned(),
            ];
            for manifest in manifests {
                args.extend(["--manifest".to_owned(), (*manifest).to_owned()]);
            }
            if explicit_lens {
                args.extend([
                    "--lens".to_owned(),
                    workspace()
                        .join("crates/holden-cognition/fixtures/project-synthetic/lens")
                        .to_str()
                        .unwrap()
                        .to_owned(),
                ]);
            }
            let mut command = Command::new(&b.holden);
            command
                .args(args)
                .current_dir(&self.root)
                .env("AGENT_PRIVATE_ROOT", self.root.join("agent-private"));
            command.output().unwrap()
        }

        fn approve(&self) {
            #[cfg(target_os = "macos")]
            {
                FileKeyStore::open(&self.owner_store).unwrap();
                let path = self.owner_store.join("owner-signing.keychain");
                holden_seal::KeychainKeyStore::create(
                    &path,
                    "throwaway-slice-eighteen-keychain-password",
                )
                .unwrap();
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
                        "throwaway-slice-eighteen-keychain-password",
                    )
                    .output()
                    .unwrap();
                assert!(out.status.success(), "approve: {}", text(&out));
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
                    .write_all(b"throwaway-slice-eighteen-owner-passphrase\nthrowaway-slice-eighteen-owner-passphrase\n")
                    .unwrap();
                let out = child.wait_with_output().unwrap();
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

        fn commit_change(&self) -> String {
            fs::write(
                self.host.join("CHANGELOG.md"),
                "Exercise the real declared W2 build.\n",
            )
            .unwrap();
            Git::new(&self.host, &self.root.join("git-home"))
                .commit_all("exercise declared W2")
                .unwrap()
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

    async fn w2(fx: &mut Fixture) -> (Value, Vec<holden_eval::VerdictRecord>) {
        fx.approve();
        fx.start_agent();
        let commit = fx.commit_change();
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
        assert_eq!(check["status"], "CHECK_COMPLETE", "{check}");
        let runs = check["runs"].as_array().unwrap();
        assert_eq!(runs.len(), 1, "{check}");
        assert_eq!(runs[0]["decision"], "pass", "{check}");
        assert_eq!(check["evaluated_commit"], commit, "{check}");
        let run = runs[0]["run_id"].as_str().unwrap();
        let record: holden_eval::VerdictRecord = serde_json::from_slice(
            &fs::read(fx.state.join("runs").join(run).join("record.json")).unwrap(),
        )
        .unwrap();
        tokio::time::timeout(Duration::from_secs(30), client.cancel())
            .await
            .unwrap()
            .unwrap();
        (check, vec![record])
    }

    /// P1. Fail-without: ignore --manifest; embed goes red UnsupportedStack.
    #[tokio::test]
    async fn declared_app_crate_binds_and_completes() {
        let mut fx = Fixture::new("declared-app", "p1");
        let out = fx.embed(&["app/Cargo.toml"], true);
        assert!(out.status.success(), "embed: {}", text(&out));
        let config: Value =
            serde_json::from_slice(&fs::read(fx.host.join(".holden/config.json")).unwrap())
                .unwrap();
        assert_eq!(
            config["product_binding"]["manifests"][0]["manifest"],
            "app/Cargo.toml"
        );
        assert_eq!(
            config["product_binding"]["manifests"][0]["entry_points"][0]["name"],
            "declared-app"
        );
        let (_, records) = w2(&mut fx).await;
        assert_eq!(
            records[0].body.decided.decision,
            holden_eval::Decision::Pass
        );
        assert!(
            records[0]
                .body
                .observations
                .iter()
                .any(|o| o.label == "workspace-tests")
        );
        assert!(
            records[0]
                .body
                .observations
                .iter()
                .any(|o| o.label == "cli-0")
        );
    }

    /// P2. Fail-without: bind only the first manifest; b's entry points and
    /// observations disappear and this test goes red.
    #[tokio::test]
    async fn two_declared_crates_bind_separately() {
        let mut fx = Fixture::new("declared-tools", "p2");
        let out = fx.embed(&["tools/a/Cargo.toml", "tools/b/Cargo.toml"], true);
        assert!(out.status.success(), "embed: {}", text(&out));
        let config: Value =
            serde_json::from_slice(&fs::read(fx.host.join(".holden/config.json")).unwrap())
                .unwrap();
        let manifests = config["product_binding"]["manifests"].as_array().unwrap();
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0]["entry_points"][0]["name"], "declared-tool-a");
        assert_eq!(manifests[1]["entry_points"][0]["name"], "declared-tool-b");
        let (_, records) = w2(&mut fx).await;
        let observations = &records[0].body.observations;
        for label in [
            "workspace-tests",
            "workspace-tests:tools/b/Cargo.toml",
            "cli-0",
            "cli-1",
        ] {
            assert!(
                observations.iter().any(|o| o.label == label),
                "missing {label}"
            );
        }
        assert_eq!(
            records[0].body.decided.decision,
            holden_eval::Decision::Pass
        );
    }

    /// P3. Fail-without: change one byte of the exact refusal.
    #[test]
    fn undeclared_non_root_refusal_names_manifest() {
        let fx = Fixture::new("prose-markdown", "p3");
        let out = fx.embed(&[], true);
        assert_eq!(out.status.code(), Some(1));
        assert!(
            text(&out).lines().any(|line| line == REFUSAL),
            "{}",
            text(&out)
        );
        assert!(text(&out).contains("detection: candidate promises 2"));
    }

    /// P4. Fail-without: accept an absolute or escaping declaration.
    #[test]
    fn declared_paths_are_validated() {
        let fx = Fixture::new("declared-app", "p4");
        let absolute = fx.host.join("app/Cargo.toml");
        for value in [absolute.to_str().unwrap(), "../host/app/Cargo.toml"] {
            let out = fx.embed(&[value], true);
            assert_eq!(out.status.code(), Some(1), "{}", text(&out));
            assert!(text(&out).contains("product_binding:"), "{}", text(&out));
        }

        let tools = Fixture::new("declared-tools", "p4-nested");
        let out = tools.embed(&["tools/a/Cargo.toml", "tools/b/Cargo.toml"], true);
        assert!(out.status.success(), "embed: {}", text(&out));
        let config: Value =
            serde_json::from_slice(&fs::read(tools.host.join(".holden/config.json")).unwrap())
                .unwrap();
        assert_eq!(
            config["product_binding"]["manifests"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(!config.to_string().contains("nested/wasm"));

        fs::write(
            tools.host.join("tools/a/Cargo.toml"),
            "[workspace]\nmembers = [\".\", \"../b\"]\n\n[package]\nname = \"declared-tool-a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        let out = tools.embed(&["tools/a/Cargo.toml"], true);
        assert_eq!(out.status.code(), Some(1));
        assert!(
            text(&out).contains("outside the declared")
                || text(&out).contains("hierarchically below"),
            "{}",
            text(&out)
        );
    }

    /// P6. Fail-without: print help to stderr or return nonzero.
    #[test]
    fn help_exit_codes_are_pinned() {
        for args in [vec!["--help"], vec!["help"], vec!["embed", "--help"]] {
            let out = Command::new(&bins().holden).args(args).output().unwrap();
            assert_eq!(out.status.code(), Some(0));
            assert!(String::from_utf8_lossy(&out.stdout).starts_with("usage:"));
            assert!(out.stderr.is_empty());
        }
        let out = Command::new(&bins().holden)
            .arg("not-a-command")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).contains("usage:"));
    }

    /// P7. Fail-without: restore the cwd-relative lens path; the child run
    /// from the fixture root fails with entity not found.
    #[test]
    fn lens_default_is_cwd_independent() {
        let fx = Fixture::new("declared-app", "p7");
        assert_ne!(fx.root, fx.host);
        let out = fx.embed(&["app/Cargo.toml"], false);
        assert!(out.status.success(), "default-lens embed: {}", text(&out));
        assert!(fx.state.join("lens/draw.json").is_file());
        assert!(fx.state.join("lens/corpus/concepts.json").is_file());
    }

    fn public_files(root: &Path, at: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        if !at.exists() {
            return;
        }
        if at.is_file() {
            files.insert(
                at.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                fs::read(at).unwrap(),
            );
            return;
        }
        for entry in fs::read_dir(at).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                public_files(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }

    /// P8. Fail-without: create a run directory or hosts entry before
    /// binding refusal; the pre-existing state snapshots go red. The
    /// refuse/fix/re-run public result still matches a clean first embed.
    #[test]
    fn refused_embed_leaves_no_host_state() {
        let fx = Fixture::new("declared-app", "p8");
        fs::create_dir_all(&fx.state).unwrap();
        fs::write(fx.state.join("unrelated-sentinel"), b"keep this state\n").unwrap();
        let private = fx.root.join("agent-private");
        fs::create_dir_all(&private).unwrap();
        fs::write(private.join("root-sentinel"), b"keep this root\n").unwrap();
        let mut expected_state = BTreeMap::new();
        public_files(&fx.state, &fx.state, &mut expected_state);
        let mut expected_private = BTreeMap::new();
        public_files(&private, &private, &mut expected_private);

        let refused = fx.embed(&[], false);
        assert_eq!(refused.status.code(), Some(1));
        assert!(
            !fx.state.join("runs").exists(),
            "refusal created an empty runs directory in existing state"
        );
        assert!(
            !private.join("holden-mcp/hosts").exists(),
            "refusal created a hosts entry in the existing private root"
        );
        let mut actual_state = BTreeMap::new();
        public_files(&fx.state, &fx.state, &mut actual_state);
        assert_eq!(
            actual_state, expected_state,
            "refusal changed existing state"
        );
        let mut actual_private = BTreeMap::new();
        public_files(&private, &private, &mut actual_private);
        assert_eq!(
            actual_private, expected_private,
            "refusal changed the existing private root"
        );
        assert!(!fx.host.join(".holden").exists(), "refusal left host state");
        let fixed = fx.embed(&["app/Cargo.toml"], false);
        assert!(fixed.status.success(), "fixed embed: {}", text(&fixed));
        let mut actual = BTreeMap::new();
        public_files(&fx.host, &fx.host.join(".holden"), &mut actual);
        public_files(&fx.host, &fx.host.join(".mcp.json"), &mut actual);

        let clean = Fixture::new("declared-app", "p8-clean");
        let first = clean.embed(&["app/Cargo.toml"], false);
        assert!(first.status.success(), "clean embed: {}", text(&first));
        let mut expected = BTreeMap::new();
        public_files(&clean.host, &clean.host.join(".holden"), &mut expected);
        public_files(&clean.host, &clean.host.join(".mcp.json"), &mut expected);
        assert_eq!(actual, expected);
        assert!(!fx.state.join("lens/corpus").join("orphan").exists());
    }

    /// P10. Fail-without: remove any one pinned contract statement.
    #[test]
    fn embed_usage_documents_the_contract() {
        let help = Command::new(&bins().holden).arg("--help").output().unwrap();
        let usage = String::from_utf8_lossy(&help.stdout);
        let readme = fs::read_to_string(workspace().join("README.md")).unwrap();
        for statement in [
            "shared\n      by every worktree",
            "charter/ even when no\n      PLANNING.md exists",
            "Embed only runs cargo metadata",
            "post-commit jit",
            "--state defaults to <private root>/hosts/<name>",
            "--manifest <relative-path>",
        ] {
            assert!(usage.contains(statement), "usage missing {statement}");
        }
        for statement in [
            "shared by every worktree",
            "charter/` even when the host has no `PLANNING.md",
            "Embed runs only `cargo metadata`",
            "post-commit JIT and W2 run `cargo build` and `cargo test`",
            "`--state` defaults to `<private root>/hosts/<name>`",
            "--manifest app/Cargo.toml",
        ] {
            assert!(readme.contains(statement), "README missing {statement}");
        }
    }
}
