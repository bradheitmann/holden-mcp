//! Slice-11: one real owner's Cargo repository, embedded and driven only
//! through its published MCP instructions, prompts, workflow descriptions
//! and /holden command. The verifier side checks source spans independently,
//! verifies closed audits, and records controls from actual signed records.
//!
//! HOLDEN_E2E_REAL_REPO is mandatory. It names a seed clone under the private
//! test root; every test process clones that seed afresh. No tracked source
//! names the seed or the original repository.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::ffi::OsString;
    use std::fs::{self, DirBuilder};
    use std::io::{BufRead, BufReader};
    use std::os::unix::fs::DirBuilderExt;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Stdio};
    use std::sync::{Arc, Mutex, OnceLock};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use holden_eval::spawn::{self, Exit, Limits, Outcome, Spawn};
    use holden_eval::{ServerReveal, VerdictRecord};
    use holden_ledger::Ledger;
    use holden_ledger::controls::Control;
    use holden_ledger::records::{CheckSeen, ControlKind, ControlRecord, Expect, Target, kind};
    use holden_seal::{EvaluatorKey, PlanCommit};
    use rmcp::handler::client::ClientHandler;
    use rmcp::model::{
        CallToolRequestParams, GetPromptRequestParams, NumberOrString, ProgressNotificationParam,
        ProgressToken, RequestMetaObject,
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
        ) -> impl std::future::Future<Output = ()> + Send + '_ {
            // The notification's own phase text is the entire human trace;
            // never print request arguments, private paths or sealed state.
            eprintln!(
                "progress: step {}: {}",
                params.progress,
                params.message.as_deref().unwrap_or_default()
            );
            self.observed.lock().unwrap().push(params);
            std::future::ready(())
        }
    }

    type Client = RunningService<RoleClient, ProgressLog>;
    type TestResult = Result<(), String>;
    type TextResult = Result<String, String>;

    const READY_DEADLINE: Duration = Duration::from_secs(300);
    const MCP_DEADLINE: Duration = Duration::from_secs(30);
    // Charter hosts now run every numbered proof and guarantee. Keep the
    // assertions and per-child limits unchanged while allowing the larger
    // real bounded call and end-to-end exchange to finish.
    const CHECK_DEADLINE: Duration = Duration::from_secs(900);
    const E2E_DEADLINE: Duration = Duration::from_secs(1_800);
    const REAP_DEADLINE: Duration = Duration::from_secs(5);

    fn workspace() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    fn now() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    }

    fn run_bounded(
        program: &Path,
        args: &[String],
        cwd: &Path,
        env: Vec<(String, String)>,
        milliseconds: u64,
    ) -> TextResult {
        let limits = Limits {
            run_ms: milliseconds,
            capture_bytes: 4 * 1024 * 1024,
            cpu_secs: milliseconds.div_ceil(1000) + 1,
            file_bytes: 128 * 1024 * 1024,
            open_files: 1024,
        };
        let spec = Spawn {
            program: program.to_path_buf(),
            args: args.iter().map(OsString::from).collect(),
            cwd: cwd.to_path_buf(),
            env,
            stdin: Vec::new(),
        };
        let out = spawn::run(&spec, &limits)
            .map_err(|e| format!("could_not_run: spawn {}: {e}", program.display()))?;
        if out.exit.success() {
            Ok(out.stdout.text())
        } else {
            Err(format!(
                "could_not_run: {} exited {:?}: {}{}",
                program.display(),
                out.exit,
                out.stdout.text(),
                out.stderr.text()
            ))
        }
    }

    fn command(
        program: &Path,
        args: &[&str],
        cwd: &Path,
        env: Vec<(String, String)>,
    ) -> TextResult {
        run_bounded(
            program,
            &args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
            cwd,
            env,
            900_000,
        )
    }

    async fn command_async(
        program: &Path,
        args: &[&str],
        cwd: &Path,
        env: Vec<(String, String)>,
    ) -> TextResult {
        let program = program.to_path_buf();
        let cwd = cwd.to_path_buf();
        let args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
        // The runtime stays responsive for MCP and the outer 15-minute
        // guard. The OS thread owns the blocking process pipes and wait.
        run_bounded_async(program, args, cwd, env, 900_000).await
    }

    async fn run_bounded_async(
        program: PathBuf,
        args: Vec<String>,
        cwd: PathBuf,
        env: Vec<(String, String)>,
        milliseconds: u64,
    ) -> TextResult {
        let task = tokio::task::spawn_blocking(move || {
            run_bounded(&program, &args, &cwd, env, milliseconds)
        });
        let grace = Duration::from_millis(milliseconds) + REAP_DEADLINE;
        tokio::time::timeout(grace, task)
            .await
            .map_err(|_| "could_not_run: subprocess wait deadline exceeded".to_owned())?
            .map_err(|e| format!("could_not_run: subprocess wait task: {e}"))?
    }

    async fn spawn_async(spec: Spawn, limits: Limits) -> Result<Outcome, String> {
        let duration = Duration::from_millis(limits.run_ms) + REAP_DEADLINE;
        tokio::time::timeout(
            duration,
            tokio::task::spawn_blocking(move || spawn::run(&spec, &limits)),
        )
        .await
        .map_err(|_| "could_not_run: child kill/reap deadline".to_owned())?
        .map_err(|e| format!("could_not_run: child deadline task: {e}"))?
        .map_err(|e| format!("could_not_run: child spawn: {e}"))
    }

    fn host_env(extra: Vec<(String, String)>) -> Vec<(String, String)> {
        let mut env = vec![
            (
                "PATH".to_owned(),
                std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".to_owned()),
            ),
            ("GIT_CONFIG_NOSYSTEM".to_owned(), "1".to_owned()),
            ("GIT_CONFIG_GLOBAL".to_owned(), "/dev/null".to_owned()),
            ("GIT_TERMINAL_PROMPT".to_owned(), "0".to_owned()),
            ("LC_ALL".to_owned(), "C".to_owned()),
            ("RUSTC_WRAPPER".to_owned(), String::new()),
            ("RUSTC_WORKSPACE_WRAPPER".to_owned(), String::new()),
            (
                "RUSTC".to_owned(),
                Path::new(env!("CARGO"))
                    .parent()
                    .unwrap_or(Path::new("/usr/bin"))
                    .join("rustc")
                    .display()
                    .to_string(),
            ),
            (
                "HOLDEN_TEST_OWNER_KEYCHAIN_PASSWORD".to_owned(),
                "throwaway-real-e2e-keychain-password".to_owned(),
            ),
        ];
        env.extend(extra);
        env
    }

    #[derive(Debug)]
    struct Bins {
        holden: PathBuf,
        confine: PathBuf,
        forge: PathBuf,
    }

    fn bins() -> Result<&'static Bins, String> {
        static BINS: OnceLock<Result<Bins, String>> = OnceLock::new();
        BINS.get_or_init(|| {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let profile = exe
                .parent()
                .and_then(Path::parent)
                .ok_or("could_not_run: test binary has no target directory")?;
            let target = profile
                .parent()
                .ok_or("could_not_run: test binary has no target root")?;
            let manifest = workspace().join("Cargo.toml");
            let root = workspace();
            let mut args = vec![
                "build".to_owned(),
                "--locked".to_owned(),
                "--quiet".to_owned(),
                "--manifest-path".to_owned(),
                manifest.display().to_string(),
                "-p".to_owned(),
                "holden-cli".to_owned(),
                "--bin".to_owned(),
                "holden".to_owned(),
                "-p".to_owned(),
                "holden-eval".to_owned(),
                "--bin".to_owned(),
                "holden-confine".to_owned(),
            ];
            let cargo = PathBuf::from(env!("CARGO"));
            command(
                &cargo,
                &args.iter().map(String::as_str).collect::<Vec<_>>(),
                &root,
                host_env(vec![(
                    "CARGO_TARGET_DIR".to_owned(),
                    target.display().to_string(),
                )]),
            )?;
            let forge_target = target.join("real-repo-forge");
            args = vec![
                "build".to_owned(),
                "--offline".to_owned(),
                "--locked".to_owned(),
                "--quiet".to_owned(),
                "--manifest-path".to_owned(),
                root.join("modules/holden-forge/Cargo.toml")
                    .display()
                    .to_string(),
            ];
            command(
                &cargo,
                &args.iter().map(String::as_str).collect::<Vec<_>>(),
                &root,
                host_env(vec![(
                    "CARGO_TARGET_DIR".to_owned(),
                    forge_target.display().to_string(),
                )]),
            )?;
            let bins = Bins {
                holden: profile.join("holden"),
                confine: profile.join("holden-confine"),
                forge: forge_target.join("debug/forge"),
            };
            for path in [&bins.holden, &bins.confine, &bins.forge] {
                if !path.is_file() {
                    return Err(format!(
                        "could_not_run: built binary is absent: {}",
                        path.display()
                    ));
                }
            }
            Ok(bins)
        })
        .as_ref()
        .map_err(Clone::clone)
    }

    struct RealHost {
        root: PathBuf,
        host: PathBuf,
        state: PathBuf,
        owner_store: PathBuf,
        cargo: PathBuf,
        agent: Option<Child>,
        agent_stderr: Option<PathBuf>,
    }

    impl RealHost {
        async fn new() -> Result<Self, String> {
            let seed = std::env::var_os("HOLDEN_E2E_REAL_REPO")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .or_else(|| {
                    holden_embed::private_root()
                        .map(|root| root.join("e2e/real-repo"))
                        .filter(|path| path.is_dir())
                })
                .ok_or_else(|| {
                    "could_not_run: HOLDEN_E2E_REAL_REPO is unset and no private-root real seed exists"
                        .to_owned()
                })?;
            if !seed.is_absolute() || !seed.join(".git").exists() {
                return Err(
                    "could_not_run: HOLDEN_E2E_REAL_REPO is not an absolute git clone".to_owned(),
                );
            }
            let root = PathBuf::from("/tmp").join(format!("hm11-{}-{}", std::process::id(), now()));
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)
                .map_err(|e| format!("could_not_run: create temp root: {e}"))?;
            let root = root.canonicalize().map_err(|e| e.to_string())?;
            let host = root.join("host");
            let seed_text = seed.display().to_string();
            let host_text = host.display().to_string();
            command_async(
                Path::new("git"),
                &["clone", "--quiet", "--no-local", &seed_text, &host_text],
                &root,
                host_env(Vec::new()),
            )
            .await?;
            let promises = real_owner_outcome(&host);
            let planning = promises
                .iter()
                .map(|promise| format!("PROMISE: {promise}\n"))
                .collect::<String>();
            fs::write(host.join("PLANNING.md"), planning).map_err(|e| e.to_string())?;
            command_async(
                Path::new("git"),
                &[
                    "-c",
                    "user.name=holden-real-e2e",
                    "-c",
                    "user.email=holden-real-e2e@example.invalid",
                    "add",
                    "PLANNING.md",
                ],
                &host,
                host_env(Vec::new()),
            )
            .await?;
            command_async(
                Path::new("git"),
                &[
                    "-c",
                    "user.name=holden-real-e2e",
                    "-c",
                    "user.email=holden-real-e2e@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                    "commit",
                    "--quiet",
                    "-m",
                    "test: promote real owner outcomes to planning promises",
                ],
                &host,
                host_env(Vec::new()),
            )
            .await?;
            let state = root.join("state");
            let owner_store = root.join("test-owner-store");
            for directory in [&state, &owner_store] {
                DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(directory)
                    .map_err(|e| format!("could_not_run: private test directory: {e}"))?;
            }
            let channel = fs::read_to_string(host.join("rust-toolchain.toml"))
                .ok()
                .and_then(|text| {
                    text.lines().find_map(|line| {
                        line.trim()
                            .strip_prefix("channel = \"")
                            .and_then(|v| v.strip_suffix('"'))
                            .map(str::to_owned)
                    })
                })
                .ok_or("could_not_run: the Cargo host declares no rust-toolchain channel")?;
            let rustup = run_bounded_async(
                Path::new("rustup").to_path_buf(),
                vec![
                    "which".to_owned(),
                    "--toolchain".to_owned(),
                    channel,
                    "cargo".to_owned(),
                ],
                host.clone(),
                host_env(Vec::new()),
                30_000,
            )
            .await?;
            let cargo = PathBuf::from(rustup.trim());
            if !cargo.is_file() {
                return Err("could_not_run: the host's declared cargo is absent".to_owned());
            }
            Ok(Self {
                root,
                host,
                state,
                owner_store,
                cargo,
                agent: None,
                agent_stderr: None,
            })
        }

        async fn holden(&self, args: &[String]) -> TextResult {
            let bins = bins()?;
            let text = command_async(
                &bins.holden,
                &args.iter().map(String::as_str).collect::<Vec<_>>(),
                &self.host,
                host_env(vec![(
                    "AGENT_PRIVATE_ROOT".to_owned(),
                    self.root.join("agent-private").display().to_string(),
                )]),
            )
            .await?;
            Ok(text)
        }

        async fn embed(&self) -> TestResult {
            let denylist = self.root.join("denylist.txt");
            fs::write(&denylist, "synthetic-real-e2e-private-token\n")
                .map_err(|e| e.to_string())?;
            let lens = workspace().join("crates/holden-cognition/fixtures/project-synthetic/lens");
            self.holden(&[
                "embed".to_owned(),
                self.host.display().to_string(),
                "--state".to_owned(),
                self.state.display().to_string(),
                "--forge".to_owned(),
                bins()?.forge.display().to_string(),
                "--denylist".to_owned(),
                denylist.display().to_string(),
                "--lens".to_owned(),
                lens.display().to_string(),
                "--cargo".to_owned(),
                self.cargo.display().to_string(),
                "--confine-helper".to_owned(),
                bins()?.confine.display().to_string(),
                "--owner-store".to_owned(),
                self.owner_store.display().to_string(),
            ])
            .await
            .map(|_| ())
        }

        async fn approve(&self) -> TestResult {
            #[cfg(target_os = "macos")]
            {
                holden_seal::KeychainKeyStore::create(
                    self.owner_store.join("owner-signing.keychain"),
                    "throwaway-real-e2e-keychain-password",
                )
                .map_err(|e| format!("could_not_run: isolated owner keychain: {e}"))?;
                // `/usr/bin/script` supplies a real PTY to the unchanged owner
                // command, exactly like the proven README quickstart. The
                // production presence refusal is deliberately kept intact.
                command_async(
                    Path::new("/usr/bin/script"),
                    &[
                        "-q",
                        "/dev/null",
                        &bins()?.holden.display().to_string(),
                        "approve",
                        "--host",
                        &self.host.display().to_string(),
                        "--owner-store",
                        &self.owner_store.display().to_string(),
                        "--create-owner-key",
                    ],
                    &self.host,
                    host_env(Vec::new()),
                )
                .await
                .map(|_| ())
            }
            #[cfg(target_os = "linux")]
            {
                // util-linux `script` allocates the real PTY required by the
                // unchanged owner command. Creation seals and then reopens the
                // disposable key, so answer both interactive passphrase reads.
                let command = format!(
                    "{} approve --host {} --owner-store {} --create-owner-key",
                    bins()?.holden.display(),
                    self.host.display(),
                    self.owner_store.display(),
                );
                let spec = Spawn {
                    program: PathBuf::from("/usr/bin/script"),
                    args: ["-q", "-e", "-c", &command, "/dev/null"]
                        .iter()
                        .map(OsString::from)
                        .collect(),
                    cwd: self.host.clone(),
                    env: host_env(Vec::new()),
                    stdin: b"throwaway-real-e2e-owner-passphrase\nthrowaway-real-e2e-owner-passphrase\n"
                        .to_vec(),
                };
                let limits = Limits {
                    run_ms: 900_000,
                    capture_bytes: 4 * 1024 * 1024,
                    cpu_secs: 901,
                    file_bytes: 128 * 1024 * 1024,
                    open_files: 1024,
                };
                let out = spawn_async(spec, limits).await?;
                if out.exit.success() {
                    Ok(())
                } else {
                    Err(format!(
                        "could_not_run: Linux PTY approval exited {:?}: {}{}",
                        out.exit,
                        out.stdout.text(),
                        out.stderr.text()
                    ))
                }
            }
        }

        async fn start_agent(&mut self) -> TestResult {
            self.stop_agent().await?;
            let stderr_path = self.root.join(format!("agent-{}.stderr", now()));
            let stderr = fs::File::create(&stderr_path).map_err(|e| e.to_string())?;
            let mut child = std::process::Command::new(&bins()?.holden)
                .args(["agent", "serve", "--state"])
                .arg(&self.state)
                .current_dir(&self.host)
                .env("AGENT_PRIVATE_ROOT", self.root.join("agent-private"))
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::from(stderr))
                .spawn()
                .map_err(|e| format!("could_not_run: agent spawn: {e}"))?;
            let stdout = child
                .stdout
                .take()
                .ok_or("could_not_run: agent stdout is unavailable")?;
            let (ready, done) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    if line.is_ok_and(|line| line.starts_with("holden agent: ready")) {
                        let _ = ready.send(());
                        break;
                    }
                }
            });
            self.agent = Some(child);
            self.agent_stderr = Some(stderr_path);
            let waiting = tokio::task::spawn_blocking(move || done.recv_timeout(READY_DEADLINE));
            let result = tokio::time::timeout(READY_DEADLINE + REAP_DEADLINE, waiting)
                .await
                .map_err(|_| {
                    format!(
                        "could_not_run: agent ready deadline {}s",
                        READY_DEADLINE.as_secs()
                    )
                })?
                .map_err(|e| format!("could_not_run: agent ready task: {e}"))?;
            match result {
                Ok(()) => Ok(()),
                Err(error) => {
                    self.stop_agent().await?;
                    Err(format!(
                        "could_not_run: agent ready deadline {}s: {error}",
                        READY_DEADLINE.as_secs()
                    ))
                }
            }
        }

        async fn stop_agent(&mut self) -> TestResult {
            let Some(child) = self.agent.take() else {
                return Ok(());
            };
            let reaper = tokio::task::spawn_blocking(move || {
                let mut child = child;
                let deliberately_killed = child.try_wait().map_err(|e| e.to_string())?.is_none();
                if deliberately_killed {
                    child
                        .kill()
                        .map_err(|e| format!("could_not_run: kill agent: {e}"))?;
                }
                let status = child
                    .wait()
                    .map_err(|e| format!("could_not_run: agent reap: {e}"))?;
                if deliberately_killed || status.success() {
                    Ok(())
                } else {
                    Err(format!("could_not_run: agent exited before stop: {status}"))
                }
            });
            tokio::time::timeout(REAP_DEADLINE, reaper)
                .await
                .map_err(|_| "could_not_run: agent kill/reap deadline".to_owned())?
                .map_err(|e| format!("could_not_run: agent reap task: {e}"))??;
            Ok(())
        }

        async fn git(&self, args: &[&str]) -> TextResult {
            let mut all = vec!["-C", self.host.to_str().unwrap()];
            all.extend(args);
            command_async(
                Path::new("git"),
                &all,
                &self.host,
                host_env(vec![(
                    "HOME".to_owned(),
                    self.root.join("git-home").display().to_string(),
                )]),
            )
            .await
        }

        async fn commit(&self, label: &str) -> TextResult {
            let readme = self.host.join("README.md");
            let mut text = fs::read_to_string(&readme).map_err(|e| e.to_string())?;
            text.push_str(&format!("\n{label}\n"));
            fs::write(readme, text).map_err(|e| e.to_string())?;
            // Workflows instruct a post-approval product commit followed by
            // W2. The hook's separate JIT path remains tested by slice-7;
            // running it here would consume W2's pre-armed nonce first.
            self.git(&[
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "user.name=real-e2e",
                "-c",
                "user.email=real-e2e@example.invalid",
                "add",
                "--all",
            ])
            .await?;
            self.git(&[
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "user.name=real-e2e",
                "-c",
                "user.email=real-e2e@example.invalid",
                "commit",
                "--quiet",
                "--no-gpg-sign",
                "-m",
                label,
            ])
            .await?;
            self.git(&["rev-parse", "HEAD"])
                .await
                .map(|text| text.trim().to_owned())
        }

        fn record(&self, run_id: &str) -> Result<VerdictRecord, String> {
            serde_json::from_slice(
                &fs::read(self.state.join("runs").join(run_id).join("record.json"))
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("could_not_run: signed run record did not decode: {e}"))
        }
    }

    impl Drop for RealHost {
        fn drop(&mut self) {
            if let Some(mut child) = self.agent.take() {
                let _ = child.kill();
                let (sender, receiver) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let _ = sender.send(child.wait());
                });
                if receiver.recv_timeout(REAP_DEADLINE).is_err() {
                    eprintln!("could_not_run: Drop agent kill/reap deadline");
                }
            }
            if std::env::var_os("HOLDEN_E2E_TRANSPORT_DIAGNOSTIC").is_some() {
                eprintln!("diagnostic scratch retained: {}", self.root.display());
            } else {
                let _ = fs::remove_dir_all(&self.root);
            }
        }
    }

    fn diagnostic() -> bool {
        std::env::var_os("HOLDEN_E2E_TRANSPORT_DIAGNOSTIC").is_some()
    }

    async fn deadline<T>(label: &str, duration: Duration, future: impl Future<Output = T>) -> T {
        tokio::time::timeout(duration, future)
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "could_not_run: MCP deadline {label} exceeded after {}s",
                    duration.as_secs()
                )
            })
    }

    async fn mcp_client(fx: &RealHost) -> Result<Client, String> {
        // This is the installed machine/harness MCP surface, not a direct
        // socket connection or an in-process Core.
        let config: Value = serde_json::from_slice(
            &fs::read(fx.host.join(".mcp.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let args: Vec<String> = config["mcpServers"]["holden"]["args"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect();
        let mut command = tokio::process::Command::new(&bins()?.holden);
        command
            .args(args)
            .current_dir(&fx.host)
            .env("AGENT_PRIVATE_ROOT", fx.root.join("agent-private"))
            .kill_on_drop(true);
        let relay_stderr = fs::File::create(fx.root.join(format!("relay-{}.stderr", now())))
            .map_err(|e| format!("could_not_run: relay log: {e}"))?;
        let (transport, _) = TokioChildProcess::builder(command)
            .stderr(Stdio::from(relay_stderr))
            .spawn()
            .map_err(|e| format!("could_not_run: installed MCP stdio spawn: {e}"))?;
        deadline(
            "initialize",
            MCP_DEADLINE,
            ProgressLog::default().serve(transport),
        )
        .await
        .map_err(|e| format!("could_not_run: initialize: {e}"))
    }

    /// Closed-run audits belong to a separately authorized evaluator, never
    /// the outside implementer that drives /holden. This client exists only
    /// in the verifier-side proof and does not influence workflow results.
    async fn audit_client(fx: &RealHost) -> Result<Client, String> {
        let stream = deadline(
            "auditor connect",
            MCP_DEADLINE,
            tokio::net::UnixStream::connect(fx.state.join("se/holden-mcp.sock")),
        )
        .await
        .map_err(|e| format!("could_not_run: evaluator auditor socket: {e}"))?;
        deadline(
            "auditor initialize",
            MCP_DEADLINE,
            ProgressLog::default().serve(stream),
        )
        .await
        .map_err(|e| format!("could_not_run: evaluator auditor initialize: {e}"))
    }

    async fn cancel(client: Client) -> TestResult {
        deadline("cancel", MCP_DEADLINE, client.cancel())
            .await
            .map(|_| ())
            .map_err(|e| format!("could_not_run: MCP cancel: {e}"))
    }

    async fn call(client: &Client, tool: &str, args: Value) -> Result<Value, String> {
        let slash = match tool {
            "holden_onboard" => "onboard",
            "holden_check" => "check",
            "holden_explain" => "explain",
            "holden_status" => "status",
            "audit_reveal" => "audit",
            _ => return Err(format!("could_not_run: no /holden mapping for {tool}")),
        };
        if slash != "audit" {
            // The prompt is retrieved from the server, never reconstructed
            // from internal code or a private state API.
            let prompt_name = match slash {
                "onboard" => "W1-ONBOARD",
                "check" => "W2-CHECK",
                "explain" => "W3-EXPLAIN",
                "status" => "W4-STATUS",
                _ => unreachable!(),
            };
            let mut arguments = rmcp::model::JsonObject::new();
            arguments.insert("host".to_owned(), args["host"].clone());
            if slash == "explain" {
                arguments.insert("run_id".to_owned(), args["run_id"].clone());
            }
            let prompt = deadline(
                "get_prompt",
                MCP_DEADLINE,
                client
                    .get_prompt(GetPromptRequestParams::new(prompt_name).with_arguments(arguments)),
            )
            .await
            .map_err(|e| format!("could_not_run: /holden {slash} prompt: {e}"))?;
            let published = prompt.messages[0]
                .content
                .as_text()
                .map_or("", |t| t.text.as_str());
            if !published.contains(tool) || !published.contains("host=") {
                return Err(format!(
                    "could_not_run: /holden {slash} prompt does not direct {tool}"
                ));
            }
        }
        let mut params = CallToolRequestParams::new(tool.to_owned());
        if let Value::Object(map) = args {
            params = params.with_arguments(map);
        }
        if matches!(tool, "holden_check" | "holden_status") {
            params.meta = Some(RequestMetaObject::with_progress_token(ProgressToken(
                NumberOrString::String(format!("holden-{tool}").into()),
            )));
        }
        let duration = if tool == "holden_check" {
            CHECK_DEADLINE
        } else {
            MCP_DEADLINE
        };
        eprintln!("exchange: {tool} started");
        let started = Instant::now();
        let result = deadline("call_tool", duration, client.call_tool(params))
            .await
            .map_err(|e| {
                format!(
                    "could_not_run: MCP {tool} after {}ms: {e}",
                    started.elapsed().as_millis()
                )
            })?;
        eprintln!(
            "exchange: {tool} completed in {}ms",
            started.elapsed().as_millis()
        );
        let body = result.structured_content.clone().unwrap_or(Value::Null);
        if result.is_error.unwrap_or(false) {
            Err(format!("could_not_run: MCP {tool} refused: {body}"))
        } else {
            Ok(body)
        }
    }

    /// Independently read the real charter's owner Outcome bullets. The
    /// regression clone promotes these exact words to explicit PLANNING.md
    /// promises so its original product-bound checks remain the regression
    /// under test rather than running historical DONE proofs as products.
    fn real_owner_outcome(host: &Path) -> Vec<String> {
        let bytes = fs::read(host.join("charter/INTENT.md")).unwrap();
        let mut active = false;
        let mut out = Vec::new();
        for line in bytes.split_inclusive(|b| *b == b'\n') {
            let line = line.strip_suffix(b"\n").unwrap_or(line);
            let text = String::from_utf8(line.to_vec()).unwrap();
            if text.starts_with("- Outcome (observable):") {
                active = true;
            } else if active && text.starts_with("- ") {
                active = false;
            }
            if active {
                let trimmed = text.trim_start();
                if let Some((number, body)) = trimmed.split_once(". ")
                    && !number.is_empty()
                    && number.bytes().all(|b| b.is_ascii_digit())
                {
                    out.push(body.to_owned());
                }
            }
        }
        out
    }

    /// Independently read the explicit planning markers. This is deliberately
    /// not the adapter under test: a paraphrase leaves these bytes unchanged.
    fn real_promises(host: &Path) -> Vec<String> {
        fs::read_to_string(host.join("PLANNING.md"))
            .unwrap()
            .lines()
            .filter_map(|line| {
                line.trim_start()
                    .strip_prefix("- PROMISE: ")
                    .or_else(|| line.strip_prefix("PROMISE: "))
                    .map(str::to_owned)
            })
            .collect()
    }

    async fn plant_regression(fx: &RealHost) -> Result<(), String> {
        let out = command_async(
            &fx.cargo,
            &[
                "metadata",
                "--offline",
                "--locked",
                "--no-deps",
                "--format-version",
                "1",
            ],
            &fx.host,
            host_env(Vec::new()),
        )
        .await?;
        let metadata: Value = serde_json::from_str(&out).map_err(|e| e.to_string())?;
        let mut libraries: Vec<PathBuf> = metadata["packages"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .flat_map(|package| package["targets"].as_array().into_iter().flatten())
            .filter(|target| {
                target["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "lib"))
            })
            .filter_map(|target| target["src_path"].as_str().map(PathBuf::from))
            .collect();
        libraries.sort();
        let library = libraries
            .first()
            .ok_or("could_not_run: no real library target for the planted regression")?;
        let mut bytes = fs::read(library).map_err(|e| e.to_string())?;
        bytes.extend_from_slice(b"\ncompile_error!(\"planted real-repository regression\");\n");
        fs::write(library, bytes).map_err(|e| e.to_string())
    }

    async fn audit(
        fx: &RealHost,
        _implementer: &Client,
        check: &Value,
    ) -> Result<Vec<(String, Value)>, String> {
        let auditor = audit_client(fx).await?;
        let mut out = Vec::new();
        for run in check["runs"].as_array().unwrap_or(&Vec::new()) {
            let run_id = run["run_id"].as_str().ok_or("W2 returned no run id")?;
            let audit = deadline(
                "audit_reveal",
                MCP_DEADLINE,
                // Audit is explicitly published by initialize INSTRUCTIONS;
                // it is not a workflow or an owner operation.
                auditor.call_tool(
                    CallToolRequestParams::new("audit_reveal".to_owned())
                        .with_arguments(json!({"run_id": run_id}).as_object().unwrap().clone()),
                ),
            )
            .await
            .map_err(|e| format!("could_not_run: audit_reveal: {e}"))?;
            let audit = audit.structured_content.unwrap_or(Value::Null);
            if audit["verdict"]["decision"] != run["decision"] {
                return Err("audit decision differs from W2".to_owned());
            }
            let plan = run["plan_commit"]
                .as_str()
                .ok_or("W2 returned no plan commit")?;
            let commit = hex::decode(
                check["evaluated_commit"]
                    .as_str()
                    .ok_or("W2 returned no evaluated commit")?,
            )
            .map_err(|e| e.to_string())?;
            let plan_commit = PlanCommit(
                hex::decode(plan)
                    .map_err(|e| e.to_string())?
                    .try_into()
                    .map_err(|_| "plan commit had the wrong byte length")?,
            );
            let verified = ServerReveal::from_reveal_body(
                &holden_embed::jit::reveal_part(&audit),
                audit["epoch_public_key"].as_str().unwrap_or_default(),
                &plan_commit,
                &hex::encode(commit),
            )
            .map_err(|e| format!("audit proof refused: {e:?}"))?;
            if verified.wire().params.bytes.is_empty() {
                return Err("audit returned empty parameters".to_owned());
            }
            // Reading the real signed record is a verifier operation; it does
            // not drive or influence the agent's workflow.
            let record = fx.record(run_id)?;
            if record.body.evaluated_commit != check["evaluated_commit"]
                || record.body.plan_commit != plan
                || record.body.reveal != *verified.wire()
            {
                return Err("audit does not reopen the signed run record".to_owned());
            }
            out.push((plan.to_owned(), audit));
        }
        cancel(auditor).await?;
        Ok(out)
    }

    fn record_controls(
        fx: &RealHost,
        check: &Value,
        proposals: &BTreeMap<String, (String, String)>,
    ) -> Result<(), String> {
        let mut ledger =
            Ledger::open(&fx.state.join("db/ledger.sqlite")).map_err(|e| e.to_string())?;
        let key = EvaluatorKey::load(
            &holden_seal::FileKeyStore::open(fx.state.join("keys")).map_err(|e| e.to_string())?,
            &holden_seal::KeyName::new("ledger").map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("could_not_run: test ledger signer: {e}"))?;
        for run in check["runs"].as_array().unwrap_or(&Vec::new()) {
            let run_id = run["run_id"].as_str().ok_or("control has no run")?;
            let record = fx.record(run_id)?;
            let promise = run["promise"].as_str().unwrap_or_default().to_owned();
            let (promise_id, cognition) = proposals
                .get(&promise)
                .ok_or("a W2 run did not match W1's real promise")?;
            let body = &record.body;
            let control = Control {
                name: format!("real-{run_id}"),
                variant: "evaluated-commit".to_owned(),
                commit: body.evaluated_commit.clone(),
                // The current real workspace and the deliberately planted
                // compiler revision are known-broken controls. Their observed
                // outcomes, not this expectation, produce catch counts.
                kind: ControlKind::FixtureMutant,
                expect: Expect::Broken {
                    target: Target::Promise { text: promise },
                },
                must_fail: None,
            };
            let checks = body
                .checks
                .iter()
                .zip(&body.decided.results)
                .map(|(check, result)| CheckSeen {
                    id: check.id.clone(),
                    key: format!("{}/{}", body.plan_id, check.id),
                    promise_text: check.promise_text.clone(),
                    lens: None,
                    probe: check.probe,
                    outcome: result.outcome,
                    consequence: result.consequence.clone(),
                })
                .collect();
            let control_record = ControlRecord {
                format: "holden.control/1".to_owned(),
                holdout: format!("{}:{}", promise_id, cognition),
                version: 1,
                plan_id: body.plan_id.clone(),
                plan_commit: body.plan_commit.clone(),
                promise: promise_id.clone(),
                cognition: cognition.clone(),
                control: control.name.clone(),
                variant: control.variant.clone(),
                kind: control.kind,
                expect: control.expect.clone(),
                must_fail: None,
                evaluated_commit: body.evaluated_commit.clone(),
                verdict_sha256: record.sha256(),
                world_id: body.world_id.clone(),
                rules_version: body.rules_version.clone(),
                decision: body.decided.decision,
                reasons: body.decided.reasons.clone(),
                checks,
                limitations: body.confinement.limitations.clone(),
            };
            ledger
                .append_json(&key, kind::CONTROL, &control_record)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    #[derive(Debug)]
    struct Evidence {
        before: Value,
        onboard: Value,
        promises: Vec<String>,
        probes: Vec<Value>,
        commits: Vec<String>,
        checks: Vec<Value>,
        audits: Vec<Vec<(String, Value)>>,
        explains: Vec<Value>,
        telemetry: Value,
        expected_check_count: usize,
        stall: Exit,
        stall_ms: u64,
    }

    async fn run() -> Result<Evidence, String> {
        let mut fx = RealHost::new().await?;
        fx.embed().await?;
        let config: Value = serde_json::from_slice(
            &fs::read(fx.host.join(".holden/config.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let binding = &config["product_binding"];
        if binding["stack"] != "cargo_workspace"
            || binding["entry_points"]
                .as_array()
                .unwrap_or(&Vec::new())
                .len()
                < 2
        {
            return Err(
                "could_not_run: real Cargo binding did not declare its real CLIs".to_owned(),
            );
        }
        fx.start_agent().await?;
        let client = mcp_client(&fx).await?;
        let info = client
            .peer_info()
            .ok_or("could_not_run: initialize returned no instructions")?;
        let instructions = info.instructions.as_deref().unwrap_or_default();
        for step in ["W1 ONBOARD", "W2 CHECK", "W3 EXPLAIN", "W4 STATUS"] {
            if !instructions.contains(step) {
                return Err(format!("initialize omitted {step}"));
            }
        }
        let tools = deadline("list_all_tools", MCP_DEADLINE, client.list_all_tools())
            .await
            .map_err(|e| format!("could_not_run: list tools: {e}"))?;
        for tool in [
            "holden_onboard",
            "holden_check",
            "holden_explain",
            "holden_status",
        ] {
            let description = tools
                .iter()
                .find(|candidate| candidate.name == tool)
                .and_then(|candidate| candidate.description.as_deref())
                .unwrap_or_default();
            if !description.contains("W") {
                return Err(format!("{tool} has no public workflow description"));
            }
        }
        let slash_command = fs::read_to_string(fx.host.join(".factory/commands/holden.md"))
            .map_err(|e| e.to_string())?;
        for (subcommand, tool) in [
            ("onboard", "holden_onboard"),
            ("check", "holden_check"),
            ("explain", "holden_explain"),
            ("status", "holden_status"),
        ] {
            if !slash_command.contains(&format!("- {subcommand}: call {tool}")) {
                return Err(format!("/holden has no {subcommand} -> {tool} mapping"));
            }
        }
        let before = call(&client, "holden_onboard", json!({"host": fx.host})).await?;
        if before["status"] != "OWNER_ACTION_REQUIRED" {
            return Err("initial W1 did not stop for owner approval".to_owned());
        }
        let promises = real_promises(&fx.host);
        if promises.len() < 2 {
            return Err("could_not_run: real intent yielded too few promises".to_owned());
        }
        cancel(client).await?;
        fx.stop_agent().await?;
        fx.approve().await?; // owner-side action, explicitly not the agent driver.
        fx.start_agent().await?;
        let client = mcp_client(&fx).await?;
        let onboard = call(&client, "holden_onboard", json!({"host": fx.host})).await?;
        if onboard["status"] != "OWNER_APPROVED" {
            return Err("approved W1 did not reopen".to_owned());
        }
        let proposals: BTreeMap<String, (String, String)> = onboard["proposals"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|proposal| {
                (
                    proposal["promise"].as_str().unwrap().to_owned(),
                    (
                        proposal["promise_id"].as_str().unwrap().to_owned(),
                        proposal["cognition_id"].as_str().unwrap().to_owned(),
                    ),
                )
            })
            .collect();

        let mut commits = Vec::new();
        let mut checks = Vec::new();
        let mut audits = Vec::new();
        for label in ["first real product commit", "second real product commit"] {
            let commit = fx.commit(label).await?;
            let check = call(&client, "holden_check", json!({"host": fx.host})).await?;
            if diagnostic() {
                fs::write(
                    fx.root.join("diagnostic-w2.json"),
                    serde_json::to_vec_pretty(&check).map_err(|e| e.to_string())?,
                )
                .map_err(|e| format!("could_not_run: write W2 diagnostic: {e}"))?;
                return Err("diagnostic_round: real W2 returned and was recorded".to_owned());
            }
            if check["status"] != "CHECK_COMPLETE"
                || check["evaluated_commit"] != commit
                || check["runs"].as_array().unwrap_or(&Vec::new()).len() != promises.len()
            {
                return Err("W2 did not execute every real approved plan".to_owned());
            }
            audits.push(audit(&fx, &client, &check).await?);
            commits.push(commit);
            checks.push(check);
        }

        plant_regression(&fx).await?;
        let broken_commit = fx.commit("planted real product regression").await?;
        let broken = call(&client, "holden_check", json!({"host": fx.host})).await?;
        if broken["evaluated_commit"] != broken_commit {
            return Err("W2 did not evaluate the planted commit".to_owned());
        }
        let mut explains = Vec::new();
        for run in broken["runs"].as_array().unwrap_or(&Vec::new()) {
            if run["decision"] != "fail" {
                return Err("planted regression passed W2".to_owned());
            }
            explains.push(
                call(
                    &client,
                    "holden_explain",
                    json!({"host": fx.host, "run_id": run["run_id"]}),
                )
                .await?,
            );
        }
        commits.push(broken_commit);
        checks.push(broken.clone());
        audits.push(audit(&fx, &client, &broken).await?);
        let expected_check_count: usize = broken["runs"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|run| {
                fx.record(run["run_id"].as_str().unwrap_or_default())
                    .map(|record| record.body.checks.len())
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum();
        for check in [&checks[0], &checks[1], &broken] {
            record_controls(&fx, check, &proposals)?;
        }
        let status = call(&client, "holden_status", json!({"host": fx.host})).await?;
        let telemetry = status["telemetry"].clone();
        cancel(client).await?;

        // Adversarial stalled child in this fresh real scratch clone. The
        // process is group-killed at the deadline and reaped by spawn::run.
        let start = Instant::now();
        let stalled = spawn_async(
            Spawn {
                program: PathBuf::from("/bin/sleep"),
                args: vec!["60".into()],
                cwd: fx.host.clone(),
                env: host_env(Vec::new()),
                stdin: Vec::new(),
            },
            Limits {
                run_ms: 150,
                ..Limits::PRODUCT
            },
        )
        .await?;
        let stall_ms = start.elapsed().as_millis() as u64;
        let stall = stalled.exit;
        if !matches!(
            stall,
            Exit::Killed {
                limit: spawn::LimitHit::RunTime
            }
        ) {
            return Err(format!(
                "stalled child was not killed at its deadline: {stall:?}"
            ));
        }
        fx.stop_agent().await?;
        Ok(Evidence {
            before,
            onboard,
            promises,
            probes: vec![binding.clone()],
            commits,
            checks,
            audits,
            explains,
            telemetry,
            expected_check_count,
            stall,
            stall_ms,
        })
    }

    fn cargo_target_sets() -> BTreeSet<PathBuf> {
        fs::read_dir("/tmp")
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .starts_with("holden-mcp-target-s11-e2e-")
                })
            })
            .collect()
    }

    fn evidence() -> &'static Evidence {
        static EVIDENCE: OnceLock<Result<Evidence, String>> = OnceLock::new();
        EVIDENCE
            .get_or_init(|| {
                let began = Instant::now();
                let targets_before = cargo_target_sets();
                let _ = bins()?;
                let remaining = E2E_DEADLINE.saturating_sub(began.elapsed());
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("could_not_run: runtime: {e}"))?;
                let result = runtime.block_on(async {
                    tokio::time::timeout(remaining, run()).await.map_err(|_| {
                        "could_not_run: e2e exceeded its 900-second self deadline".to_owned()
                    })?
                });
                runtime.shutdown_background();
                let residues: BTreeSet<PathBuf> = cargo_target_sets()
                    .difference(&targets_before)
                    .cloned()
                    .collect();
                assert!(residues.is_empty(), "e2e left target residue: {residues:?}");
                result
            })
            .as_ref()
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// P1. Fail-without: paraphrase one extracted promise before approval.
    #[test]
    fn w1_real_promises_verbatim() {
        let evidence = evidence();
        for response in [&evidence.before, &evidence.onboard] {
            let actual: BTreeSet<String> = response["proposals"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["promise"].as_str().unwrap().to_owned())
                .collect();
            let expected: BTreeSet<String> = evidence.promises.iter().cloned().collect();
            assert_eq!(actual, expected, "W1 changed the owner's byte spans");
        }
        let serialized = evidence.onboard.to_string();
        for promise in &evidence.promises {
            assert!(serialized.contains(promise));
        }
        assert_eq!(evidence.probes[0]["stack"], "cargo_workspace");
    }

    /// P2. Fail-without: reuse commit one's reveal for the second commit.
    #[test]
    fn two_commits_different_params_audit_verified() {
        let evidence = evidence();
        assert_eq!(evidence.commits.len(), 3);
        assert_ne!(evidence.commits[0], evidence.commits[1]);
        assert_eq!(evidence.audits[0].len(), evidence.promises.len());
        let first: BTreeMap<&str, &str> = evidence.audits[0]
            .iter()
            .map(|(plan, audit)| (plan.as_str(), audit["params"]["bytes"].as_str().unwrap()))
            .collect();
        for (plan, audit) in &evidence.audits[1] {
            let params = audit["params"]["bytes"].as_str().unwrap();
            assert_ne!(first[plan.as_str()], params, "reused params for {plan}");
        }
        assert_eq!(evidence.checks[0]["evaluated_commit"], evidence.commits[0]);
        assert_eq!(evidence.checks[1]["evaluated_commit"], evidence.commits[1]);
    }

    /// P3. Fail-without: include a threshold in a W3 diagnostic.
    #[test]
    fn planted_regression_fails_w2_w3_names_consequence_only() {
        let evidence = evidence();
        let broken = &evidence.checks[2];
        assert_eq!(broken["evaluated_commit"], evidence.commits[2]);
        assert_eq!(evidence.explains.len(), evidence.promises.len());
        for explain in &evidence.explains {
            assert_eq!(explain["status"], "FIX_REQUIRED");
            let fixes = explain["fixes"].as_array().unwrap();
            assert!(!fixes.is_empty());
            for fixed in fixes {
                assert!(
                    fixed
                        .as_object()
                        .is_some_and(|fields| fields.contains_key("promise")),
                    "W3 omitted the optional promise field"
                );
                match &fixed["promise"] {
                    Value::String(id) => assert!(!id.is_empty()),
                    Value::Null => {
                        // Integrity/build diagnostics have no specific promise.
                    }
                    other => panic!("W3 promise id is not string or null: {other}"),
                }
                assert!(fixed["consequence"].as_str().is_some_and(|c| !c.is_empty()));
            }
            let leaked = explain.to_string().to_ascii_lowercase();
            for forbidden in [
                "threshold",
                "step_order",
                "server_nonce",
                "builder_nonce",
                "params",
                "proof",
                "seed",
                "value",
            ] {
                assert!(
                    !leaked.contains(forbidden),
                    "W3 leaked {forbidden}: {leaked}"
                );
            }
        }
    }

    fn assert_rate_recomputes(rate: &Value) {
        let n = rate["n"].as_u64().expect("rate must carry sample size n");
        let k = rate["k"]
            .as_u64()
            .expect("rate must carry observation count k");
        assert!(k <= n, "observations exceed the sample: k={k}, n={n}");
        let (expected_lower, expected_upper) =
            holden_ledger::stats::wilson(k, n, holden_ledger::stats::Z95);
        // The report is produced by the separately compiled server binary.
        // Permit only a single-float-bit difference from that codegen unit.
        for (field, expected) in [("lower", expected_lower), ("upper", expected_upper)] {
            let actual = rate[field].as_f64().unwrap();
            assert!(
                actual.to_bits().abs_diff(expected.to_bits()) <= 1,
                "{field} differs beyond one float bit: {actual} != {expected}"
            );
        }
        if n == 0 {
            assert!(rate["point"].is_null());
        } else {
            let expected = k as f64 / n as f64;
            assert_eq!(rate["point"].as_f64(), Some(expected));
        }
    }

    /// P4. Fail-without: omit the sample size n from rate_json.
    #[test]
    fn telemetry_reports_catch_rates_with_samples() {
        let evidence = evidence();
        let checks = evidence.telemetry["checks"].as_array().unwrap();
        assert_eq!(checks.len(), evidence.expected_check_count);
        let total: u64 = checks
            .iter()
            .map(|check| check["catch"]["n"].as_u64().unwrap_or(0))
            .sum();
        assert!(total > 0, "no real catch trials");
        for table in ["checks", "cognitions", "plans", "integrity"] {
            for row in evidence.telemetry[table].as_array().unwrap() {
                for rate_name in ["catch", "false_fail"] {
                    let rate = &row[rate_name];
                    assert_rate_recomputes(rate);
                }
            }
        }
        assert!(matches!(
            evidence.stall,
            Exit::Killed {
                limit: spawn::LimitHit::RunTime
            }
        ));
        assert!(evidence.stall_ms < 10_000, "stalled child was not bounded");
    }
}
