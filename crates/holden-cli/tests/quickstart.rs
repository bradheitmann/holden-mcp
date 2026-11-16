//! Slice-12 P2: execute the public macOS quickstart against a fresh clone,
//! a disposable private root, a real terminal owner action, the fixture host,
//! and the real implementer `/holden check` workflow. Nothing is mocked.

#[cfg(target_os = "macos")]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::fs::{self, File};
    use std::io::{BufRead, BufReader};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Output, Stdio};
    use std::time::{Duration, Instant};

    use holden_seal::{FileKeyStore, KeychainKeyStore};
    use rmcp::model::CallToolRequestParams;
    use rmcp::transport::TokioChildProcess;
    use rmcp::{RoleClient, ServiceExt};
    use serde_json::json;

    fn workspace() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(tag: &str) -> Self {
            let root =
                PathBuf::from("/tmp").join(format!("holden-cli-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn quickstart_block(readme: &str, name: &str) -> String {
        let start = format!("<!-- quickstart-test:{name} -->\n```sh\n");
        let rest = readme
            .split_once(&start)
            .unwrap_or_else(|| panic!("README lacks the {name} quickstart block"))
            .1;
        rest.split_once("```\n<!-- quickstart-test:end -->")
            .unwrap_or_else(|| panic!("README's {name} quickstart block is not closed"))
            .0
            .to_owned()
    }

    fn run_script(clone: &Path, root: &Path, name: &str, script: &str) {
        let log_path = root.join(format!("{name}.log"));
        let log = File::create(&log_path).unwrap();
        let mut child = Command::new("/bin/sh")
            .args(["-eu", "-c", script])
            .current_dir(clone)
            .env("HOLDEN_QUICKSTART_ROOT", root)
            .env("HOLDEN_HOST", root.join("notes-host"))
            .env("HOLDEN_WORKSPACE", clone)
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    root.join("install/bin").display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1_200);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "README {name} failed:\n{}",
                    fs::read_to_string(&log_path).unwrap()
                );
                return;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "README {name} exceeded 1200s:\n{}",
                    fs::read_to_string(&log_path).unwrap()
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn owner_approve(root: &Path, host: &Path) -> Output {
        let owner_store = root.join("agent-private/holden-mcp/owner-keys");
        FileKeyStore::open(&owner_store).unwrap();
        let keychain = holden_embed::approve::owner_keychain_path(&owner_store);
        KeychainKeyStore::create(&keychain, "throwaway-test-keychain-password").unwrap();
        Command::new("/usr/bin/script")
            .args(["-q", "/dev/null"])
            .arg(root.join("install/bin/holden"))
            .args(["approve", "--host"])
            .arg(host)
            .arg("--create-owner-key")
            .env("AGENT_PRIVATE_ROOT", root.join("agent-private"))
            .env(
                "HOLDEN_TEST_OWNER_KEYCHAIN_PASSWORD",
                "throwaway-test-keychain-password",
            )
            .output()
            .unwrap()
    }

    struct Agent(Child);

    impl Drop for Agent {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn start_agent(root: &Path, host: &Path) -> Agent {
        let log = File::create(root.join("agent.log")).unwrap();
        let mut child = Command::new(root.join("install/bin/holden"))
            .args(["agent", "serve", "--host"])
            .arg(host)
            .current_dir(host)
            .env("AGENT_PRIVATE_ROOT", root.join("agent-private"))
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log))
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
        rx.recv_timeout(Duration::from_secs(300))
            .expect("the quickstart agent did not become ready");
        Agent(child)
    }

    /// P2. Fail-without mutant: omit `holden init` from the setup block; the
    /// documented denylist write then has no initialized private directory
    /// and this test exits red before embedding.
    #[tokio::test]
    async fn readme_quickstart_leads_to_green_check() {
        let scratch = TempRoot::new("readme-quickstart");
        let root = scratch.path().to_path_buf();
        let clone = root.join("fresh-clone");
        let cloned = Command::new("git")
            .args(["clone", "--quiet", "--no-hardlinks"])
            .arg(workspace())
            .arg(&clone)
            .output()
            .unwrap();
        assert!(
            cloned.status.success(),
            "fresh clone failed: {}",
            String::from_utf8_lossy(&cloned.stderr)
        );

        // Execute the current working document before its commit; committed
        // gate reopenings execute exactly the clone's tracked README bytes.
        let readme = fs::read_to_string(workspace().join("README.md")).unwrap();
        fs::write(clone.join("README.md"), &readme).unwrap();
        run_script(&clone, &root, "setup", &quickstart_block(&readme, "setup"));
        assert!(readme.contains("holden init"));
        assert!(readme.contains("holden migrate-owner-key"));
        assert!(readme.contains("/holden check"));
        assert!(readme.contains("OWNER ONLY"));
        let host = root.join("notes-host");
        let approved = owner_approve(&root, &host);
        assert!(approved.status.success(), "approval was refused");
        let _agent = start_agent(&root, &host);
        run_script(&host, &root, "commit", &quickstart_block(&readme, "commit"));

        let socket = root
            .join("agent-state/si")
            .join(holden_mcp::config::SOCKET_FILE);
        let stream = tokio::time::timeout(
            Duration::from_secs(30),
            tokio::net::UnixStream::connect(&socket),
        )
        .await
        .unwrap()
        .unwrap();
        let client: rmcp::service::RunningService<RoleClient, ()> =
            tokio::time::timeout(Duration::from_secs(30), ().serve(stream))
                .await
                .unwrap()
                .unwrap();
        let params = CallToolRequestParams::new("holden_check")
            .with_arguments(serde_json::from_value(json!({"host": host})).unwrap());
        let result = tokio::time::timeout(Duration::from_secs(300), client.call_tool(params))
            .await
            .expect("holden_check exceeded its deadline")
            .unwrap();
        let body = result.structured_content.unwrap();
        assert_ne!(result.is_error, Some(true), "{body}");
        assert_eq!(body["status"], "CHECK_COMPLETE", "{body}");
        assert_eq!(body["runs"].as_array().unwrap().len(), 1, "{body}");
        assert_eq!(body["runs"][0]["decision"], "pass", "{body}");
        tokio::time::timeout(Duration::from_secs(30), client.cancel())
            .await
            .unwrap()
            .unwrap();
        drop(_agent);
        fs::remove_dir_all(&root).unwrap();
    }

    /// Slice-17 P5. Fail-without mutant: remove the holden-mcp cargo-install
    /// line from README; the fresh-clone PATH invocation then fails.
    #[test]
    fn readme_install_covers_holden_mcp() {
        let scratch = TempRoot::new("readme-install");
        let root = scratch.path().to_path_buf();
        let clone = root.join("fresh-clone");
        let cloned = Command::new("git")
            .args(["clone", "--quiet", "--no-hardlinks"])
            .arg(workspace())
            .arg(&clone)
            .output()
            .unwrap();
        assert!(cloned.status.success());
        let readme = fs::read_to_string(workspace().join("README.md")).unwrap();
        fs::write(clone.join("README.md"), &readme).unwrap();
        run_script(
            &clone,
            &root,
            "install-setup",
            &quickstart_block(&readme, "setup"),
        );
        assert!(root.join("install/bin/holden").is_file());
        assert!(root.join("install/bin/holden-mcp").is_file());
        let output = Command::new("holden-mcp")
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    root.join("install/bin").display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("holden-mcp serve"));
        fs::remove_dir_all(&root).unwrap();
    }

    /// Slice-17 P3. The client is spawned from the exact `.mcp.json` bytes;
    /// no direct socket or in-process server is substituted.
    #[tokio::test]
    async fn written_mcp_json_drives_w1_through_w4() {
        let scratch = TempRoot::new("written-mcp");
        let root = scratch.path().to_path_buf();
        let clone = root.join("fresh-clone");
        let cloned = Command::new("git")
            .args(["clone", "--quiet", "--no-hardlinks"])
            .arg(workspace())
            .arg(&clone)
            .output()
            .unwrap();
        assert!(cloned.status.success());
        let readme = fs::read_to_string(workspace().join("README.md")).unwrap();
        fs::write(clone.join("README.md"), &readme).unwrap();
        run_script(
            &clone,
            &root,
            "written-setup",
            &quickstart_block(&readme, "setup"),
        );
        let host = root.join("notes-host");
        let approved = owner_approve(&root, &host);
        assert!(approved.status.success(), "approval was refused");
        let agent = start_agent(&root, &host);
        run_script(
            &host,
            &root,
            "written-commit",
            &quickstart_block(&readme, "commit"),
        );

        let config: serde_json::Value =
            serde_json::from_slice(&fs::read(host.join(".mcp.json")).unwrap()).unwrap();
        let args: Vec<String> = config["mcpServers"]["holden"]["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|arg| arg.as_str().unwrap().to_owned())
            .collect();
        let mut command = tokio::process::Command::new("holden");
        command
            .args(args)
            .current_dir(&host)
            .env("AGENT_PRIVATE_ROOT", root.join("agent-private"))
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    root.join("install/bin").display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .kill_on_drop(true);
        let (transport, _) = TokioChildProcess::builder(command).spawn().unwrap();
        let client: rmcp::service::RunningService<RoleClient, ()> =
            tokio::time::timeout(Duration::from_secs(30), ().serve(transport))
                .await
                .unwrap()
                .unwrap();
        let call = async |tool: &str, arguments: serde_json::Value| {
            let params = CallToolRequestParams::new(tool.to_owned())
                .with_arguments(serde_json::from_value(arguments).unwrap());
            let result = tokio::time::timeout(Duration::from_secs(300), client.call_tool(params))
                .await
                .unwrap()
                .unwrap();
            assert_ne!(result.is_error, Some(true), "{result:?}");
            result.structured_content.unwrap()
        };
        let onboard = call("holden_onboard", json!({"host": host})).await;
        assert_eq!(onboard["status"], "OWNER_APPROVED", "{onboard}");
        let checked = call("holden_check", json!({"host": host})).await;
        assert_eq!(checked["status"], "CHECK_COMPLETE", "{checked}");
        let run = checked["runs"][0]["run_id"].as_str().unwrap().to_owned();
        let explained = call("holden_explain", json!({"host": host, "run_id": run})).await;
        assert_eq!(explained["status"], "NO_FAILURE", "{explained}");
        let status = call("holden_status", json!({"host": host})).await;
        assert_eq!(status["status"], "OWNER_ACTIONS_CLEAR", "{status}");
        tokio::time::timeout(Duration::from_secs(30), client.cancel())
            .await
            .unwrap()
            .unwrap();
        drop(agent);
        fs::remove_dir_all(&root).unwrap();
    }
}
