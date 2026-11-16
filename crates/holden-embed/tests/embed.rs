//! slice-7 integration tests: a real host git repository created from the
//! committed host fixture, the real `holden` binary (embed, approve, agent,
//! jit, trace, connect, replay), the real forge binary built from
//! `modules/holden-forge`, a real holden agent process serving real Unix
//! sockets, real `git commit`s that fire the real post-commit hook, real
//! confined builds and product runs, and a real signed ledger. The owner key
//! is a test owner key: real Ed25519 from the OS random source, in its own
//! temporary key store. Nothing under test is mocked.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs::{self, DirBuilder};
    use std::io::{BufRead, BufReader};
    #[cfg(target_os = "linux")]
    use std::io::{Read, Write};
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Output, Stdio};
    use std::sync::{OnceLock, mpsc};
    use std::time::{Duration, Instant};

    use holden_cognition::SituatedCognition;
    use holden_embed::host::{
        AWAITING_OWNER_APPROVAL, CONFIG_FILE, CONSTITUTION_FILE, HOOK_SHIM, HostConfig,
        MCP_CONFIG_FILE, PROMISE_PROPOSAL_FILE, read_approvals,
    };
    use holden_embed::jit::TraceFile;
    use holden_embed::state::{Armed, KEYS, State};
    use holden_eval::{ServerReveal, VerdictRecord, parse_record, replay};
    use holden_gen::{GenPlan, unseal_plan};
    use holden_mcp::{Role, SealedPlanFile};
    use holden_seal::{EvaluatorKey, FileKeyStore, KeyName, KeyStore, SealKey, plan_commit};
    use rmcp::ServiceExt;
    use rmcp::model::CallToolRequestParams;
    use serde_json::{Value, json};

    fn crate_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn workspace() -> PathBuf {
        crate_dir().join("../..").canonicalize().unwrap()
    }

    /// The binaries the tests run, built once per test process into this
    /// test's own target directory with the same cargo.
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
            // <target>/<profile>/deps/<test binary>
            let profile_dir = exe.parent().and_then(Path::parent).unwrap();
            let target = profile_dir.parent().unwrap();
            let manifest = workspace().join("Cargo.toml");
            let manifest = manifest.to_str().unwrap();
            for (pkg, bin) in [("holden-cli", "holden"), ("holden-eval", "holden-confine")] {
                build(
                    &[
                        "build",
                        "--locked",
                        "--quiet",
                        "-p",
                        pkg,
                        "--bin",
                        bin,
                        "--manifest-path",
                        manifest,
                    ],
                    target,
                );
            }
            let forge_target = target.join("forge-build");
            let forge_manifest = workspace().join("modules/holden-forge/Cargo.toml");
            build(
                &[
                    "build",
                    "--offline",
                    "--locked",
                    "--quiet",
                    "--manifest-path",
                    forge_manifest.to_str().unwrap(),
                ],
                &forge_target,
            );
            let b = Bins {
                holden: profile_dir.join("holden"),
                confine: profile_dir.join("holden-confine"),
                forge: forge_target.join("debug").join("forge"),
            };
            for p in [&b.holden, &b.confine, &b.forge] {
                assert!(p.is_file(), "{}", p.display());
            }
            b
        })
    }

    /// One test's host, agent state and owner key store.
    struct Fx {
        root: PathBuf,
        host: PathBuf,
        state: PathBuf,
        owner_store: PathBuf,
        denylist: PathBuf,
        agent: Option<Child>,
    }

    impl Fx {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!("hm7-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)
                .unwrap();
            let root = root.canonicalize().unwrap();
            let host = root.join("host");
            holden_embed::fixture::create_host(&workspace(), &host).unwrap();
            let denylist = root.join("denylist.txt");
            fs::write(
                &denylist,
                "# a synthetic denylist for the forge export scan\nzz-synthetic-private-token\n",
            )
            .unwrap();
            Self {
                state: root.join("state"),
                owner_store: root.join("owner"),
                host,
                denylist,
                root,
                agent: None,
            }
        }

        /// The same real Cargo product, but its only planning promise is an
        /// unmarked prose paragraph. This exercises proposal, not refusal.
        fn new_prose(tag: &str) -> Self {
            let fx = Self::new(tag);
            fs::write(
                fx.host.join("PLANNING.md"),
                "# Planning: Pocket Notes\n\nDomain: Pocket Notes: a small command-line note keeper\n\nA saved note is never lost, even if the app closes halfway through saving.\n",
            )
            .unwrap();
            fs::write(fx.host.join("README.md"), "# Pocket Notes\n").unwrap();
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

        fn embed(&self) -> Output {
            let b = bins();
            let out = self.holden(&[
                "embed",
                self.host.to_str().unwrap(),
                "--state",
                self.state.to_str().unwrap(),
                "--forge",
                b.forge.to_str().unwrap(),
                "--denylist",
                self.denylist.to_str().unwrap(),
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
                self.owner_store.to_str().unwrap(),
            ]);
            assert!(out.status.success(), "embed: {}", text(&out));
            out
        }

        /// A disposable owner store. On macOS the real `holden approve` child
        /// creates the key inside this temporary named keychain, so its own
        /// executable identity owns the item ACL. On Linux it uses a real
        /// random throwaway file key, as before slice-12.
        fn test_owner_store(&self) {
            #[cfg(target_os = "macos")]
            {
                FileKeyStore::open(&self.owner_store).unwrap();
                let path = holden_embed::approve::owner_keychain_path(&self.owner_store);
                if !path.exists() {
                    holden_seal::KeychainKeyStore::create(
                        &path,
                        "throwaway-test-keychain-password",
                    )
                    .unwrap();
                }
            }
            #[cfg(target_os = "linux")]
            {
                // The unchanged CLI creates and seals the random owner key
                // through the real PTY approval below. Only the public-key
                // directory is prepared here; no plaintext key file exists.
                FileKeyStore::open(&self.owner_store).unwrap();
            }
        }

        #[cfg(target_os = "macos")]
        fn owner_approve(&self, create: bool) -> Output {
            let mut command = Command::new("/usr/bin/script");
            command.args(["-q", "/dev/null"]).arg(&bins().holden).args([
                "approve",
                "--host",
                self.host.to_str().unwrap(),
                "--owner-store",
                self.owner_store.to_str().unwrap(),
            ]);
            if create {
                command.arg("--create-owner-key");
            }
            command
                .current_dir(&self.host)
                .env(
                    "HOLDEN_TEST_OWNER_KEYCHAIN_PASSWORD",
                    "throwaway-test-keychain-password",
                )
                .output()
                .unwrap()
        }

        #[cfg(target_os = "linux")]
        fn owner_approve(&self, create: bool) -> Output {
            let command = format!(
                "{} approve --host {} --owner-store {}{}",
                bins().holden.display(),
                self.host.display(),
                self.owner_store.display(),
                if create { " --create-owner-key" } else { "" },
            );
            let mut child = Command::new("/usr/bin/script")
                .args(["-q", "-e", "-c", &command, "/dev/null"])
                .current_dir(&self.host)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let mut stdin = child.stdin.take().unwrap();
            let mut stdout = child.stdout.take().unwrap();
            let (prompt_tx, prompt_rx) = mpsc::channel();
            let reader = std::thread::spawn(move || {
                let mut captured = Vec::new();
                let mut prompts = 0usize;
                let mut byte = [0u8; 1];
                while stdout.read(&mut byte).unwrap_or(0) == 1 {
                    captured.push(byte[0]);
                    if captured.ends_with(b"Owner passphrase: ") {
                        prompts += 1;
                        prompt_tx.send(()).unwrap();
                    }
                }
                (captured, prompts)
            });
            // Creation seals on put and then reopens the key. A reopen needs
            // one prompt. Input is sent only after the real PTY has disabled
            // echo, so even this throwaway passphrase is never captured.
            for _ in 0..if create { 2 } else { 1 } {
                prompt_rx.recv_timeout(Duration::from_secs(30)).unwrap();
                stdin
                    .write_all(b"throwaway-test-owner-passphrase\n")
                    .unwrap();
                stdin.flush().unwrap();
            }
            drop(stdin);
            let status = child.wait().unwrap();
            let (captured, prompts) = reader.join().unwrap();
            assert_eq!(prompts, if create { 2 } else { 1 });
            Output {
                status,
                stdout: captured,
                stderr: Vec::new(),
            }
        }

        /// Drive an approval that must be refused before the owner-key code
        /// asks for a passphrase. It still uses a real PTY, but correctly
        /// expects no prompt and lets the child's refusal exit normally.
        #[cfg(target_os = "macos")]
        fn owner_approve_refused(&self) -> Output {
            self.owner_approve(true)
        }

        #[cfg(target_os = "linux")]
        fn owner_approve_refused(&self) -> Output {
            let command = format!(
                "{} approve --host {} --owner-store {} --create-owner-key",
                bins().holden.display(),
                self.host.display(),
                self.owner_store.display(),
            );
            Command::new("/usr/bin/script")
                .args(["-q", "-e", "-c", &command, "/dev/null"])
                .current_dir(&self.host)
                .output()
                .unwrap()
        }

        fn approve(&self) -> Output {
            self.test_owner_store();
            let out = self.owner_approve(true);
            assert!(out.status.success(), "approve: {}", text(&out));
            out
        }

        fn state(&self) -> State {
            State::new(&self.state.canonicalize().unwrap())
        }

        /// Start the agent as its own process; returns its ready lines.
        fn start_agent(&mut self) -> Vec<String> {
            let mut child = Command::new(&bins().holden)
                .args(["agent", "serve", "--state", self.state.to_str().unwrap()])
                .env("AGENT_PRIVATE_ROOT", self.root.join("agent-private"))
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let stdout = child.stdout.take().unwrap();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let mut lines = Vec::new();
                for line in BufReader::new(stdout).lines() {
                    let Ok(line) = line else { break };
                    let ready = line.starts_with("holden agent: ready");
                    lines.push(line);
                    if ready {
                        let _ = tx.send(lines.clone());
                    }
                }
            });
            self.agent = Some(child);
            rx.recv_timeout(Duration::from_secs(300))
                .expect("the agent became ready")
        }

        /// Commit a change to the README with real git; the post-commit
        /// hook runs `holden` from PATH. Returns the new commit and git's
        /// output (the hook's output included).
        fn commit(&self, n: u32) -> (String, String) {
            let readme = self.host.join("README.md");
            let mut t = fs::read_to_string(&readme).unwrap();
            t.push_str(&format!("\nChange {n}.\n"));
            fs::write(&readme, t).unwrap();
            let bin_dir = bins().holden.parent().unwrap().to_owned();
            let home = self.root.join("git-user-home");
            fs::create_dir_all(&home).unwrap();
            let out = Command::new("git")
                .env_clear()
                .env("PATH", format!("{}:/usr/bin:/bin", bin_dir.display()))
                .env("HOME", &home)
                .env("AGENT_PRIVATE_ROOT", self.root.join("agent-private"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .args(["-C", self.host.to_str().unwrap()])
                .args([
                    "-c",
                    "user.name=host-dev",
                    "-c",
                    "user.email=dev@example.invalid",
                ])
                .args(["commit", "--quiet", "-a", "-m", &format!("change {n}")])
                .output()
                .unwrap();
            assert!(out.status.success(), "git commit: {}", text(&out));
            (self.git(&["rev-parse", "HEAD"]), text(&out))
        }

        fn git(&self, args: &[&str]) -> String {
            let out = Command::new("git")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .args(["-C", self.host.to_str().unwrap()])
                .args(args)
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?}: {}", text(&out));
            String::from_utf8_lossy(&out.stdout).trim().to_owned()
        }

        fn config(&self) -> HostConfig {
            HostConfig::read(&self.host).unwrap()
        }

        /// The runs of `commit`, with their traces.
        fn runs_of(&self, commit: &str) -> Vec<(String, TraceFile)> {
            let mut out = Vec::new();
            let dir = self.state().runs_dir();
            for e in fs::read_dir(&dir).unwrap().filter_map(Result::ok) {
                let f = e.path().join("trace.json");
                if let Ok(b) = fs::read(&f) {
                    let t: TraceFile = serde_json::from_slice(&b).unwrap();
                    if t.evaluated_commit == commit {
                        out.push((t.run_id.clone(), t));
                    }
                }
            }
            out.sort_by(|a, b| a.1.plan_id.cmp(&b.1.plan_id));
            out
        }
    }

    impl Drop for Fx {
        fn drop(&mut self) {
            if let Some(mut c) = self.agent.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// Slice-17 P7. Fail-without mutant: remove `impl Drop for Fx`; the
    /// forced-panic fixture then leaks and the leak-check goes red.
    #[test]
    fn fixture_temp_dirs_are_drop_guard_cleaned() {
        fn unguarded_fixture(root: &Path) {
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(root)
                .unwrap();
        }

        let control =
            std::env::temp_dir().join(format!("hm7-unguarded-control-{}", std::process::id()));
        let _ = fs::remove_dir_all(&control);
        let failed = std::panic::catch_unwind(|| {
            unguarded_fixture(&control);
            panic!("force the unguarded failure path");
        });
        assert!(failed.is_err());
        assert!(control.exists(), "the unguarded control must leak");
        fs::remove_dir_all(&control).unwrap();

        let root = std::env::temp_dir().join(format!("hm7-drop-guard-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let failed = std::panic::catch_unwind(|| {
            let _fx = Fx::new("drop-guard");
            panic!("force the guarded failure path");
        });
        assert!(failed.is_err());
        let leaked = root.exists();
        let _ = fs::remove_dir_all(&root);
        assert!(!leaked, "the guarded fixture leaked {}", root.display());
    }

    fn text(out: &Output) -> String {
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    }

    /// Every regular file under the host, except git's object store (which
    /// holds only compressed copies of committed files), by relative path.
    fn host_files(host: &Path) -> BTreeMap<String, (Vec<u8>, u32)> {
        fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, (Vec<u8>, u32)>) {
            for e in fs::read_dir(dir).unwrap().filter_map(Result::ok) {
                let p = e.path();
                let rel = p.strip_prefix(base).unwrap().display().to_string();
                if rel == ".git/objects" {
                    continue;
                }
                let ty = e.file_type().unwrap();
                if ty.is_dir() {
                    walk(base, &p, out);
                } else if ty.is_file() {
                    let mode = fs::metadata(&p).unwrap().permissions().mode() & 0o777;
                    out.insert(rel, (fs::read(&p).unwrap(), mode));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(host, host, &mut out);
        out
    }

    /// The sealed side's bytes that must never reach the host: every agent
    /// key and the owner key (raw and hex), each sealed plan's ciphertext
    /// (raw and hex), and each plan's canary tokens (its plaintext marks).
    fn sealed_needles(fx: &Fx) -> Vec<(String, Vec<u8>)> {
        let state = fx.state();
        let mut needles = Vec::new();
        let mut key_files: Vec<(String, PathBuf)> = KEYS
            .iter()
            .map(|n| {
                (
                    format!("key {n}"),
                    state.keys_dir().join(format!("{n}.key")),
                )
            })
            .collect();
        key_files.push(("owner key".to_owned(), fx.owner_store.join("owner.key")));
        for (what, f) in key_files {
            if let Ok(k) = fs::read(&f) {
                assert_eq!(k.len(), 32, "{what}");
                needles.push((format!("{what} hex"), hex::encode(&k).into_bytes()));
                needles.push((what, k));
            }
        }
        let seal = SealKey::new(state.key("seal").unwrap());
        for f in state.sealed_plan_files().unwrap() {
            let wire: SealedPlanFile = serde_json::from_slice(&fs::read(&f).unwrap()).unwrap();
            let sealed = wire.to_sealed().unwrap();
            needles.push((
                "sealed plan hex".to_owned(),
                wire.ciphertext.as_bytes()[..64].to_vec(),
            ));
            needles.push((
                "sealed plan bytes".to_owned(),
                sealed.ciphertext[..32].to_vec(),
            ));
            let plan: GenPlan = unseal_plan(&seal, &sealed, &plan_commit(&sealed)).unwrap();
            for c in holden_eval::canary::canaries(&plan) {
                needles.push(("plan plaintext canary".to_owned(), c.into_bytes()));
            }
        }
        needles
    }

    fn assert_no_sealed_bytes(fx: &Fx) {
        let needles = sealed_needles(fx);
        assert!(
            needles.len() > 10,
            "the scan has sealed material to look for"
        );
        for (rel, (bytes, _)) in host_files(&fx.host) {
            for (what, n) in &needles {
                assert!(
                    !bytes.windows(n.len()).any(|w| w == n.as_slice()),
                    "{what} found in the host at {rel}"
                );
            }
        }
    }

    /// Slice-12b P1: a real approval loads its signing key from the platform
    /// keystore, even when a different throwaway key occupies the former
    /// owner.key path. Every signed cognition is independently admitted
    /// against owner.pub. Linux uses a disposable passphrase-sealed store and
    /// a real PTY; macOS retains its disposable named Keychain.
    /// Fail-without mutant: select FileKeyStore on Linux; the decoy key becomes
    /// the signer and the owner.pub assertion fails.
    #[test]
    fn owner_key_resolves_through_keystore_not_file() {
        let fx = Fx::new("owner-keystore-custody");
        fx.embed();
        let name = KeyName::new("owner").unwrap();
        let public_dir = FileKeyStore::open(&fx.owner_store).unwrap();
        #[cfg(target_os = "macos")]
        let path = holden_embed::approve::owner_keychain_path(&fx.owner_store);
        #[cfg(target_os = "macos")]
        let store =
            holden_seal::KeychainKeyStore::create(&path, "throwaway-test-keychain-password")
                .unwrap();

        // Decoy at exactly the former private-root file path. Production must
        // neither load it nor replace it with the Keychain key.
        let decoy =
            holden_seal::KeyMaterial::from_bytes(holden_seal::random_bytes::<32>().unwrap());
        public_dir.put(&name, &decoy).unwrap();
        let decoy_owner = holden_seal::OwnerKey::load(&public_dir, &name).unwrap();

        // `holden` itself creates the item, so both this invocation and the
        // reopen below have the item ACL's trusted executable identity.
        let out = fx.owner_approve(true);
        assert!(out.status.success(), "approve: {}", text(&out));
        let trust = holden_cognition::OwnerTrust::read_published(&fx.owner_store).unwrap();
        assert_ne!(trust.public(), &decoy_owner.public());

        let config = fx.config();
        let doc = holden_embed::host::planning(&fx.host, &config.planning).unwrap();
        let promises = holden_cognition::extract_promises(&doc).unwrap();
        let cognitions = holden_embed::host::read_cognitions(&fx.host).unwrap();
        assert!(!cognitions.is_empty());
        for cognition in cognitions {
            let approval: holden_cognition::Approval = serde_json::from_slice(
                &fs::read(holden_embed::host::approval_file(
                    &fx.host,
                    &cognition.id.to_hex(),
                ))
                .unwrap(),
            )
            .unwrap();
            let promise = promises
                .iter()
                .find(|promise| promise.id == cognition.promise)
                .unwrap();
            let admitted = holden_cognition::HostableCognition::admit(
                &doc,
                promise,
                &cognition,
                Some(&approval),
                trust.public(),
            )
            .unwrap();
            assert_eq!(admitted.owner(), trust.public());
        }

        let reopened = fx.owner_approve(false);
        assert!(reopened.status.success(), "reopen: {}", text(&reopened));
        assert_eq!(
            holden_cognition::OwnerTrust::read_published(&fx.owner_store)
                .unwrap()
                .public(),
            trust.public()
        );

        #[cfg(target_os = "macos")]
        {
            drop(store);
            fs::remove_file(&path).unwrap();
        }
    }

    /// HC-1(b): without the owner's terminal, the real CLI children cannot
    /// approve or migrate through the disposable platform keystore, and no
    /// bars mutation command exists. No signing artifact may be published.
    /// Fail-without mutant: remove `require_owner_terminal` from `owner_key`;
    /// the no-TTY approval then succeeds and this test fails.
    #[test]
    fn owner_commands_require_presence_and_bars_stay_refused() {
        let fx = Fx::new("owner-presence");
        fx.embed();
        fx.test_owner_store();
        let out = fx.holden(&[
            "approve",
            "--host",
            fx.host.to_str().unwrap(),
            "--owner-store",
            fx.owner_store.to_str().unwrap(),
            "--create-owner-key",
        ]);
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains("owner presence required"));
        assert!(!fx.owner_store.join("owner.pub").exists());
        assert!(
            !fs::read_dir(fx.host.join(".holden/cognitions"))
                .unwrap()
                .filter_map(Result::ok)
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".approval.json"))
        );

        let migrated = fx.holden(&[
            "migrate-owner-key",
            "--owner-store",
            fx.owner_store.to_str().unwrap(),
        ]);
        assert_eq!(migrated.status.code(), Some(1));
        assert!(text(&migrated).contains("owner presence required"));

        let bars = fx.holden(&["bars", "--host", fx.host.to_str().unwrap()]);
        assert_eq!(bars.status.code(), Some(2));
        assert!(text(&bars).contains("unknown command"));
    }

    /// Slice-12b P2: once a real sealed owner key exists, a no-TTY approval
    /// is refused and publishes neither a signature nor owner.pub.
    /// Fail-without mutant: return the stored key without interactive unseal;
    /// this second approval then succeeds and creates signing artifacts.
    #[cfg(target_os = "linux")]
    #[test]
    fn non_tty_caller_cannot_approve() {
        let fx = Fx::new("non-tty-approval");
        fx.embed();
        fx.approve();
        for entry in fs::read_dir(fx.host.join(".holden/cognitions"))
            .unwrap()
            .filter_map(Result::ok)
        {
            if entry
                .file_name()
                .to_string_lossy()
                .ends_with(".approval.json")
            {
                fs::remove_file(entry.path()).unwrap();
            }
        }
        fs::remove_file(fx.owner_store.join("owner.pub")).unwrap();
        let out = fx.holden(&[
            "approve",
            "--host",
            fx.host.to_str().unwrap(),
            "--owner-store",
            fx.owner_store.to_str().unwrap(),
        ]);
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains("owner presence required"));
        assert!(!fx.owner_store.join("owner.pub").exists());
        assert!(
            !fs::read_dir(fx.host.join(".holden/cognitions"))
                .unwrap()
                .filter_map(Result::ok)
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".approval.json"))
        );
    }

    // ---------------------------------------------------------------- P1

    /// P1: `holden embed <project>` writes exactly the host's public files
    /// into a real host repository: the run file, the agent's constitution
    /// forged for the host's domain through the forge's export, one
    /// cognition's public half per promise, the implementer's MCP client
    /// config, and the two-line post-commit shim. The sealed side is in the
    /// agent's state only.
    /// Fail-without mutant: do not install the post-commit hook.
    #[test]
    fn installs_expected_files() {
        let fx = Fx::new("install");
        let before = host_files(&fx.host);
        fx.embed();
        let config = fx.config();
        let state = fx.state();
        // Exactly these new working-tree files, plus the hook.
        let untracked: BTreeSet<String> = fx
            .git(&["status", "--porcelain", "--untracked-files=all"])
            .lines()
            .map(|l| l.trim_start_matches("?? ").to_owned())
            .collect();
        let mut want: BTreeSet<String> = [
            CONFIG_FILE,
            CONSTITUTION_FILE,
            PROMISE_PROPOSAL_FILE,
            MCP_CONFIG_FILE,
            ".claude/commands/holden.md",
            ".codex/config.toml",
            ".codex/commands/holden.md",
            ".factory/mcp.json",
            ".factory/commands/holden.md",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        for id in &config.cognitions {
            want.insert(format!(".holden/cognitions/{id}.json"));
        }
        assert_eq!(untracked, want);
        let after = host_files(&fx.host);
        let added: BTreeSet<&String> = after
            .keys()
            .filter(|k| !before.contains_key(*k) && !k.starts_with(".git/"))
            .collect();
        assert_eq!(added.len(), want.len());
        // The run file.
        assert_eq!(config.format, "holden.embed/1");
        assert!(
            config
                .domain
                .starts_with("Pocket Notes: a small command-line note keeper")
        );
        assert_eq!(config.promises.len(), 4);
        assert_eq!(config.cognitions.len(), 4);
        assert_eq!(config.agent.role, "evaluator");
        assert_eq!(config.state, PathBuf::from("hosts/host"));
        assert_eq!(config.agent.process, "holden agent serve --host .");
        // The constitution, forged for the host's domain.
        let constitution = fs::read_to_string(fx.host.join(CONSTITUTION_FILE)).unwrap();
        assert!(constitution.contains(&config.domain));
        assert!(constitution.to_lowercase().contains("evaluator"));
        assert_eq!(
            config.agent.constitution_sha256,
            hex::encode(<sha2::Sha256 as sha2::Digest>::digest(
                constitution.as_bytes()
            ))
        );
        let pack = fs::read_to_string(state.host_pack()).unwrap();
        assert!(pack.contains(&config.domain));
        // The cognitions' public halves, shaped by the host-domain pack.
        for id in &config.cognitions {
            let c: SituatedCognition = serde_json::from_slice(
                &fs::read(fx.host.join(format!(".holden/cognitions/{id}.json"))).unwrap(),
            )
            .unwrap();
            assert_eq!(&c.id.to_hex(), id);
            assert_eq!(c.computed_id(), c.id);
            assert_eq!(c.overlay.as_ref().unwrap().domain, config.domain);
            assert!(config.promises.iter().any(|p| p.id == c.promise.to_hex()));
        }
        // The implementer's MCP client config resolves state at run time.
        let mcp: Value =
            serde_json::from_slice(&fs::read(fx.host.join(MCP_CONFIG_FILE)).unwrap()).unwrap();
        assert_eq!(mcp["mcpServers"]["holden"]["command"], "holden");
        assert_eq!(mcp["mcpServers"]["holden"]["args"], json!(["mcp"]));
        // The post-commit shim: two lines that only exec the holden binary.
        let hook = fx.host.join(".git/hooks/post-commit");
        assert_eq!(fs::read_to_string(&hook).unwrap(), HOOK_SHIM);
        assert_eq!(HOOK_SHIM.lines().count(), 2);
        assert_eq!(HOOK_SHIM.lines().nth(1), Some("exec holden jit \"$@\""));
        assert_eq!(
            fs::metadata(&hook).unwrap().permissions().mode() & 0o111,
            0o111
        );
        // The sealed side lives in the agent's state, not in the host.
        for name in KEYS {
            assert!(
                state.keys_dir().join(format!("{name}.key")).is_file(),
                "{name}"
            );
        }
        for role in [Role::Evaluator, Role::Implementer] {
            let mode = fs::metadata(state.server_config(role))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0, "server configs are 0600");
        }
        assert!(!fx.host.join(".holden/plans").exists());
    }

    /// P1: nothing sealed lands in the host: after embedding, the owner's
    /// approval, the agent sealing a plan for every approved cognition and a
    /// second embed, no agent key, no owner key, no sealed plan byte and no
    /// plan plaintext is in any host file.
    /// Fail-without mutant: write the sealed plans into `.holden/`.
    #[test]
    fn no_sealed_bytes_in_host() {
        let fx = Fx::new("sealed");
        fx.embed();
        fx.approve();
        let prepared = holden_embed::agent::prepare_plans(&fx.state()).unwrap();
        assert_eq!(prepared.plans.len(), 4, "{:?}", prepared.refused);
        assert!(prepared.refused.is_empty());
        fx.embed();
        assert_no_sealed_bytes(&fx);
        for (rel, _) in host_files(&fx.host) {
            assert!(!rel.ends_with(".sealed.json"), "{rel}");
            assert!(!rel.ends_with(".key"), "{rel}");
        }
    }

    /// P1: re-running embed is idempotent: the same bytes and modes in every
    /// host file (the owner's approvals stay), no key created or replaced,
    /// one hook of exactly two lines, and another MCP server already in the
    /// host's `.mcp.json` kept.
    /// Fail-without mutant: append the shim to the hook instead of writing
    /// it.
    #[test]
    fn embed_is_idempotent() {
        let fx = Fx::new("idem");
        fs::write(
            fx.host.join(MCP_CONFIG_FILE),
            "{\"mcpServers\":{\"other\":{\"command\":\"other-server\"}}}\n",
        )
        .unwrap();
        let first = fx.embed();
        assert!(text(&first).contains("agent keys created"));
        let keys = |fx: &Fx| -> Vec<Vec<u8>> {
            KEYS.iter()
                .map(|n| fs::read(fx.state().keys_dir().join(format!("{n}.key"))).unwrap())
                .collect()
        };
        let keys_before = keys(&fx);
        fx.approve();
        let snapshot = host_files(&fx.host);
        let state_config = fs::read(fx.state().agent_config_path()).unwrap();
        let again = fx.embed();
        let said = text(&again);
        assert!(!said.contains("wrote:"), "{said}");
        assert!(!said.contains("agent keys created"), "{said}");
        assert_eq!(host_files(&fx.host), snapshot);
        assert_eq!(keys(&fx), keys_before);
        assert_eq!(
            fs::read(fx.state().agent_config_path()).unwrap(),
            state_config
        );
        let hook = fs::read_to_string(fx.host.join(".git/hooks/post-commit")).unwrap();
        assert_eq!(hook, HOOK_SHIM);
        let mcp: Value =
            serde_json::from_slice(&fs::read(fx.host.join(MCP_CONFIG_FILE)).unwrap()).unwrap();
        assert_eq!(mcp["mcpServers"]["other"]["command"], "other-server");
        assert_eq!(mcp["mcpServers"].as_object().unwrap().len(), 2);
        let approvals = snapshot
            .keys()
            .filter(|k| k.ends_with(".approval.json"))
            .count();
        assert_eq!(approvals, 4);
    }

    /// Slice-17 P1. Fail-without mutant: restore `--socket <absolute>` in
    /// the written `.mcp.json`; the temporary root assertion goes red.
    #[test]
    fn host_files_carry_no_machine_paths() {
        let fx = Fx::new("path-free");
        let before = host_files(&fx.host);
        fx.embed();
        let after = host_files(&fx.host);
        let home = std::env::var_os("HOME").unwrap_or_default();
        let user = std::env::var_os("USER")
            .or_else(|| std::env::var_os("LOGNAME"))
            .unwrap_or_default();
        for (name, (bytes, _)) in &after {
            if before.contains_key(name) {
                continue;
            }
            let text = String::from_utf8_lossy(bytes);
            assert!(!text.contains(fx.root.to_string_lossy().as_ref()), "{name}");
            if !home.is_empty() {
                assert!(!text.contains(home.to_string_lossy().as_ref()), "{name}");
            }
            if user != "root" && user.len() >= 4 {
                let user = user.to_string_lossy();
                let has_account_name = text
                    .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .any(|word| word == user);
                assert!(!has_account_name, "account name in {name}");
            }
            for token in text.split(|c: char| {
                c.is_whitespace() || matches!(c, '\"' | '\'' | ',' | '[' | ']' | '{' | '}')
            }) {
                let token = token.trim_matches(|c| matches!(c, ':' | ';' | '(' | ')'));
                let home_anchor = format!("/{}/", "home");
                let users_anchor = format!("/{}/", "Users");
                let filesystem_root = [
                    "/tmp/",
                    "/private/",
                    "/var/",
                    home_anchor.as_str(),
                    users_anchor.as_str(),
                    "/mnt/",
                    "/opt/",
                    "/etc/",
                    "/usr/",
                ]
                .iter()
                .any(|prefix| token.starts_with(prefix));
                assert!(!filesystem_root, "absolute path in {name}: {token}");
            }
        }
    }

    /// Slice-17 P2. Fail-without mutant: write either root into `.mcp.json`;
    /// the byte comparison goes red.
    #[test]
    fn mcp_json_byte_identical_across_roots() {
        let first = Fx::new("machine-a");
        let second = Fx::new("machine-b");
        first.embed();
        second.embed();
        assert_eq!(
            fs::read(first.host.join(MCP_CONFIG_FILE)).unwrap(),
            fs::read(second.host.join(MCP_CONFIG_FILE)).unwrap()
        );
    }

    /// Slice-17 P4. Fail-without mutant: append the legacy socket on every
    /// re-embed; the second snapshot comparison goes red.
    #[test]
    fn reembed_migrates_old_mcp_json_and_keeps_approvals() {
        let fx = Fx::new("migrate-mcp");
        fx.embed();
        fx.approve();
        let approvals = read_approvals(&fx.host).unwrap();
        let legacy = json!({
            "mcpServers": {
                "holden": {
                    "command": "holden",
                    "args": ["connect", "--role", "implementer", "--socket", fx.state.join("si/holden-mcp.sock")]
                }
            }
        });
        fs::write(
            fx.host.join(MCP_CONFIG_FILE),
            serde_json::to_vec_pretty(&legacy).unwrap(),
        )
        .unwrap();
        fx.embed();
        let migrated = fs::read(fx.host.join(MCP_CONFIG_FILE)).unwrap();
        let value: Value = serde_json::from_slice(&migrated).unwrap();
        assert_eq!(value["mcpServers"]["holden"]["args"], json!(["mcp"]));
        fx.embed();
        assert_eq!(fs::read(fx.host.join(MCP_CONFIG_FILE)).unwrap(), migrated);
        assert_eq!(read_approvals(&fx.host).unwrap(), approvals);
        let client = holden_cognition::adapters::read_promises(&fx.host).unwrap();
        let cognitions = holden_embed::host::read_cognitions(&fx.host).unwrap();
        let trust = holden_cognition::OwnerTrust::read_published(&fx.owner_store).unwrap();
        let (admitted, refused) = client.admit(&cognitions, &approvals, &trust);
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(admitted.len(), cognitions.len());
    }

    // ---------------------------------------------------------------- P2

    const STDIO_TIMEOUT: Duration = Duration::from_secs(120);

    async fn mcp_call(
        c: &rmcp::service::RunningService<rmcp::RoleClient, ()>,
        tool: &str,
        args: Value,
    ) -> (bool, Value) {
        let mut p = CallToolRequestParams::new(tool.to_owned());
        if let Value::Object(m) = args {
            p = p.with_arguments(m);
        }
        let r = tokio::time::timeout(STDIO_TIMEOUT, c.call_tool(p))
            .await
            .unwrap_or_else(|_| panic!("MCP call {tool} exceeded {STDIO_TIMEOUT:?}"))
            .unwrap();
        (
            r.is_error.unwrap_or(false),
            r.structured_content.clone().unwrap_or(Value::Null),
        )
    }

    /// Own the stdio child on a deadline thread. A blocked read on the test
    /// runtime is not a bound: killing this child closes both stdio pipes and
    /// wakes that read whether the exchange completes, errors, or stalls.
    fn watch_stdio_child(
        mut child: Child,
        finished: mpsc::Receiver<()>,
    ) -> std::process::ExitStatus {
        let deadline = Instant::now() + STDIO_TIMEOUT;
        if finished.recv_timeout(STDIO_TIMEOUT).is_err() {
            let _ = child.kill();
            return child.wait().unwrap();
        }
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                return child.wait().unwrap();
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// P2: the embedded agent runs as its own process with its own
    /// constitution and the evaluator role; the host's installed MCP client
    /// config (run exactly as a harness would: its command and arguments)
    /// reaches the server with the implementer role, sees only public facts
    /// in `brief`, and gets `forbidden_role` from `reveal_params` on an open
    /// run that the evaluator can reveal.
    /// Fail-without mutant: install the implementer config with the
    /// evaluator role.
    #[test]
    fn host_implementer_cannot_reveal() {
        let mut fx = Fx::new("roles");
        fx.embed();
        fx.approve();
        let ready = fx.start_agent();
        let config = fx.config();
        assert!(
            ready.iter().any(|l| l
                == &format!(
                    "holden agent: constitution sha256 {}",
                    config.agent.constitution_sha256
                )),
            "{ready:?}"
        );
        assert_ne!(fx.agent.as_ref().unwrap().id(), std::process::id());
        let state = fx.state();
        let head = fx.git(&["rev-parse", "HEAD"]);

        // The host's installed config, run as the harness runs it.
        let mcp: Value =
            serde_json::from_slice(&fs::read(fx.host.join(MCP_CONFIG_FILE)).unwrap()).unwrap();
        let entry = &mcp["mcpServers"]["holden"];
        assert_eq!(entry["command"], "holden");
        let args: Vec<String> = entry["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap().to_owned())
            .collect();
        let mut bridge = Command::new(&bins().holden)
            .args(&args)
            .current_dir(&fx.host)
            .env("AGENT_PRIVATE_ROOT", fx.root.join("agent-private"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let bridge_stdin = bridge.stdin.take().unwrap();
        let bridge_stdout = bridge.stdout.take().unwrap();
        let (finished_tx, finished_rx) = mpsc::channel();
        let deadline = std::thread::Builder::new()
            .name("stdio-child-deadline".to_owned())
            .spawn(move || watch_stdio_child(bridge, finished_rx))
            .unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (brief, reveal, revealed, run_id) = rt.block_on(async {
            // The agent's own socket: the evaluator role; open a run.
            let stream = tokio::net::UnixStream::connect(state.socket(Role::Evaluator))
                .await
                .unwrap();
            let evaluator = ().serve(stream).await.unwrap();
            let evaluator_brief = mcp_call(&evaluator, "brief", json!({})).await;
            assert!(!evaluator_brief.0, "{}", evaluator_brief.1);
            assert_eq!(evaluator_brief.1["caller_role"], "evaluator");
            let plan = evaluator_brief.1["plans"][0].as_str().unwrap().to_owned();
            let run = mcp_call(
                &evaluator,
                "begin_run",
                json!({
                    "plan_commit": plan,
                    "evaluated_commit": head,
                    "builder_nonce": hex::encode(holden_seal::random_bytes::<32>().unwrap()),
                }),
            )
            .await;
            assert!(!run.0, "{}", run.1);
            let run_id = run.1["run_id"].as_str().unwrap().to_owned();

            let stdin = tokio::process::ChildStdin::from_std(bridge_stdin).unwrap();
            let stdout = tokio::process::ChildStdout::from_std(bridge_stdout).unwrap();
            let client = ().serve((stdout, stdin)).await.unwrap();
            let brief = mcp_call(&client, "brief", json!({})).await;
            let reveal = mcp_call(&client, "reveal_params", json!({ "run_id": run_id })).await;
            client.cancel().await.unwrap();
            let _ = finished_tx.send(());

            // The run is open and revealable, by the evaluator only.
            let revealed = mcp_call(&evaluator, "reveal_params", json!({ "run_id": run_id })).await;
            evaluator.cancel().await.unwrap();
            (brief, reveal, revealed, run_id)
        });
        let status = deadline.join().unwrap();
        assert!(status.success(), "holden connect exited with {status}");
        assert!(!brief.0);
        let b = &brief.1;
        assert_eq!(b["caller_role"], "implementer");
        let allowed = |tool: &str| {
            b["tools"]
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["name"] == tool)
                .unwrap()["allowed"]
                .as_bool()
                .unwrap()
        };
        assert!(!allowed("reveal_params"));
        assert!(allowed("brief"));
        let keys: BTreeSet<&str> = b.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            [
                "caller_role",
                "epoch_public_key",
                "plans",
                "run_window_ms",
                "server",
                "tools",
                "version"
            ]
            .into_iter()
            .collect()
        );
        let brief_text = b.to_string().into_bytes();
        for (what, n) in sealed_needles(&fx) {
            assert!(
                !brief_text.windows(n.len()).any(|w| w == n.as_slice()),
                "{what} in the implementer's brief"
            );
        }
        assert!(
            reveal.0,
            "the implementer revealed parameters: {}",
            reveal.1
        );
        assert_eq!(reveal.1["refusal"], "forbidden_role");
        assert!(!revealed.0, "{}", revealed.1);
        assert_eq!(revealed.1["run_id"], run_id.as_str());
    }

    // ---------------------------------------------------------------- P3

    fn record_of(fx: &Fx, run_id: &str) -> VerdictRecord {
        let text = fs::read_to_string(fx.state().run_dir(run_id).join("record.json")).unwrap();
        parse_record(&text).unwrap()
    }

    /// Reopen one run: `holden replay` and `holden trace` both hold, and the
    /// audit reveal verifies and equals what the run used.
    fn reopen(fx: &Fx, run_id: &str, t: &TraceFile) {
        let state = fx.state();
        let evaluator_public =
            EvaluatorKey::from_material(&state.key("evaluator").unwrap()).public();
        let record = record_of(fx, run_id);
        replay(&record, Some(&evaluator_public)).unwrap();
        let path = state.run_dir(run_id).join("record.json");
        let out = fx.holden(&[
            "replay",
            path.to_str().unwrap(),
            "--evaluator-key",
            &hex::encode(evaluator_public.0),
        ]);
        assert!(out.status.success(), "replay: {}", text(&out));
        let out = fx.holden(&["trace", run_id, "--host", fx.host.to_str().unwrap()]);
        let said = text(&out);
        assert!(out.status.success(), "trace: {said}");
        for link in [
            "run ",
            "promise ",
            "cognition ",
            "root ",
            "lens ",
            "plan ",
            "nonce ",
            "params ",
            "verdict ",
            "ledger ",
        ] {
            assert!(
                said.lines().any(|l| l.starts_with(link)),
                "no {link} line: {said}"
            );
        }
        assert!(said.contains(&t.promise.text));
        let audit: Value =
            serde_json::from_slice(&fs::read(state.run_dir(run_id).join("audit.json")).unwrap())
                .unwrap();
        let plan_commit =
            holden_seal::PlanCommit(hex::decode(&t.plan_commit).unwrap().try_into().unwrap());
        assert!(audit["verdict"]["sha256"] == record.sha256().as_str());
        let reveal = ServerReveal::from_reveal_body(
            &holden_embed::jit::reveal_part(&audit),
            audit["epoch_public_key"].as_str().unwrap(),
            &plan_commit,
            &t.evaluated_commit,
        )
        .unwrap();
        assert_eq!(*reveal.wire(), record.body.reveal);
        assert_eq!(reveal.wire().params, t.params);
    }

    /// P3: a real `git commit` in the host fires the post-commit shim, which
    /// runs `holden jit`: for every approved plan, a nonce committed before
    /// the commit existed opens the run on the new commit's SHA, the
    /// revealed parameters drive a confined run of the commit, the signed
    /// verdict is in the signed ledger, the audit reveal verifies, and the
    /// host gets the fixed diagnostic; `holden trace` prints and verifies
    /// the chain and `holden replay` reproduces each decision.
    /// Fail-without mutant: make the hook shim exit without running holden.
    #[test]
    fn post_commit_runs_jit_holdout() {
        let mut fx = Fx::new("jit");
        let base = fx.git(&["rev-parse", "HEAD"]);
        fx.embed();
        fx.approve();
        fx.start_agent();
        let armed: Vec<Armed> = fx.state().armed().unwrap();
        assert_eq!(armed.len(), 4);
        assert!(
            armed
                .iter()
                .all(|a| a.head == base && a.consumed_by.is_none())
        );
        let (commit, said) = fx.commit(1);
        assert_ne!(commit, base);
        let short = &commit[..12];
        let passes = said
            .lines()
            .filter(|l| l.starts_with(&format!("holden: {short}: pass")))
            .count();
        assert_eq!(passes, 4, "{said}");
        assert!(said.contains("trace: holden trace "), "{said}");
        let runs = fx.runs_of(&commit);
        assert_eq!(runs.len(), 4, "{said}");
        let state = fx.state();
        let ledger = holden_ledger::Ledger::open(&state.ledger_db()).unwrap();
        let ledger_key = EvaluatorKey::from_material(&state.key("ledger").unwrap()).public();
        assert!(ledger.verify(Some(&ledger_key), None).unwrap().holds());
        let entries = ledger.entries().unwrap();
        for (run_id, t) in &runs {
            // The nonce was committed before the commit existed: at its
            // parent, and consumed by exactly this run.
            assert_eq!(t.armed.head, base);
            assert_eq!(t.armed.consumed_by.as_deref(), Some(run_id.as_str()));
            assert!(armed.iter().any(|a| a.commitment == t.armed.commitment));
            assert_eq!(t.evaluated_commit, commit);
            assert_eq!(t.verdict.decision, "pass", "{:?}", t.verdict);
            let record = record_of(&fx, run_id);
            assert_eq!(record.body.evaluated_commit, commit);
            assert_eq!(record.sha256(), t.verdict.sha256);
            assert!(record.body.confinement.limitations.is_empty() || cfg!(target_os = "linux"));
            assert!(entries.iter().any(|e| {
                e.kind == "verdict"
                    && e.decode::<holden_ledger::records::VerdictEntry>()
                        .is_ok_and(|v| v.verdict_sha256 == t.verdict.sha256 && &v.run_id == run_id)
            }));
            reopen(&fx, run_id, t);
        }
        // The next nonces are armed at the new commit, before the next one.
        let armed = fx.state().armed().unwrap();
        assert_eq!(
            armed
                .iter()
                .filter(|a| a.head == commit && a.consumed_by.is_none())
                .count(),
            4
        );
        assert_no_sealed_bytes(&fx);
    }

    /// P3: two consecutive commits get different moment-of-test
    /// parameters for every plan: what each run recorded, what the audit
    /// reveal proves and what was planted all differ, and each run reopens
    /// (`holden trace`, `holden replay`).
    /// Fail-without mutant: have `holden jit` reuse the previous run's
    /// parameters for the plan.
    #[test]
    fn consecutive_commits_get_different_params() {
        let mut fx = Fx::new("params");
        fx.embed();
        fx.approve();
        fx.start_agent();
        let (first, said1) = fx.commit(1);
        let (second, said2) = fx.commit(2);
        let a = fx.runs_of(&first);
        let b = fx.runs_of(&second);
        assert_eq!(a.len(), 4, "{said1}");
        assert_eq!(b.len(), 4, "{said2}");
        for ((ra, ta), (rb, tb)) in a.iter().zip(&b) {
            assert_eq!(ta.plan_id, tb.plan_id);
            assert_ne!(ta.params, tb.params, "plan {}", ta.plan_id);
            assert_ne!(ta.params.bytes, tb.params.bytes);
            let (pa, pb) = (record_of(&fx, ra), record_of(&fx, rb));
            assert_eq!(pa.body.reveal.params, ta.params);
            assert_eq!(pb.body.reveal.params, tb.params);
            assert_ne!(pa.body.planted, pb.body.planted);
            assert_eq!(
                tb.armed.head, first,
                "the second run's nonce was armed at the first commit"
            );
            reopen(&fx, ra, ta);
            reopen(&fx, rb, tb);
        }
    }

    /// slice-13 P3: prose discovery still writes the complete public embed,
    /// but admission and plan preparation refuse every unapproved candidate.
    /// Fail-without: admit an unapproved candidate and prepare/run its check.
    #[test]
    fn embed_completes_awaiting_owner_approval() {
        let fx = Fx::new_prose("s13-p3");
        let output = fx.embed();
        assert!(output.status.success(), "{}", text(&output));
        let proposal_path = fx.host.join(PROMISE_PROPOSAL_FILE);
        let proposal: Value = serde_json::from_slice(&fs::read(proposal_path).unwrap()).unwrap();
        assert_eq!(proposal["status"], AWAITING_OWNER_APPROVAL);
        assert_eq!(proposal["discovery"], "prose");
        assert_eq!(proposal["detection"]["planning_process"], "markdown");
        assert_eq!(
            proposal["candidates"][0]["promises"][0]["text"],
            "A saved note is never lost, even if the app closes halfway through saving."
        );
        for file in [
            MCP_CONFIG_FILE,
            ".claude/commands/holden.md",
            ".codex/commands/holden.md",
            ".factory/commands/holden.md",
        ] {
            assert!(fx.host.join(file).is_file(), "missing {file}");
        }
        let config = fx.config();
        assert_eq!(config.promises.len(), 1);
        assert!(
            holden_embed::host::read_approvals(&fx.host)
                .unwrap()
                .is_empty()
        );
        let prepared = holden_embed::agent::prepare_plans(&fx.state()).unwrap();
        assert!(prepared.plans.is_empty());
        assert_eq!(prepared.refused.len(), 1);
        assert!(prepared.refused[0].1.contains("no owner approval"));
        assert!(fx.state().sealed_plan_files().unwrap().is_empty());
        assert_eq!(
            fs::read_dir(fx.state().runs_dir()).unwrap().count(),
            0,
            "an unapproved candidate produced a check run"
        );
    }

    /// slice-13 P4: a real owner-terminal approval admits the verbatim prose
    /// candidate; the next real post-commit hook executes its JIT holdout and
    /// the trace retains the source span. A paraphrase at the same offsets is
    /// independently refused by the verbatim constructor.
    /// Fail-without: approve a non-verbatim paraphrase or bypass span checks.
    #[test]
    fn owner_approval_enables_w2_on_approved_promise() {
        let mut fx = Fx::new_prose("s13-p4");
        fx.embed();

        // The owner is shown a paraphrase carrying the original source and
        // offsets. The real owner-terminal command must refuse it before a
        // signing key or approval is created.
        fx.test_owner_store();
        let proposal_path = fx.host.join(PROMISE_PROPOSAL_FILE);
        let original_proposal = fs::read(&proposal_path).unwrap();
        let mut tampered: Value = serde_json::from_slice(&original_proposal).unwrap();
        tampered["candidates"][0]["promises"][0]["text"] =
            Value::from("A saved note is usually retained after saving.");
        fs::write(
            &proposal_path,
            serde_json::to_vec_pretty(&tampered).unwrap(),
        )
        .unwrap();
        let refused = fx.owner_approve_refused();
        assert!(
            !refused.status.success(),
            "paraphrase approved: {}",
            text(&refused)
        );
        assert!(
            holden_embed::host::read_approvals(&fx.host)
                .unwrap()
                .is_empty()
        );
        fs::write(&proposal_path, original_proposal).unwrap();

        fx.approve();
        fx.start_agent();
        let (commit, said) = fx.commit(1);
        let runs = fx.runs_of(&commit);
        assert_eq!(runs.len(), 1, "{said}");
        let (run_id, trace) = &runs[0];
        assert_eq!(trace.verdict.decision, "pass", "{said}");
        assert_eq!(
            trace.promise.text,
            "A saved note is never lost, even if the app closes halfway through saving."
        );
        let document = holden_embed::host::planning(&fx.host, "PLANNING.md").unwrap();
        trace.promise.verify_against(&document).unwrap();
        let start = usize::try_from(trace.promise.start).unwrap();
        let end = usize::try_from(trace.promise.end).unwrap();
        assert_eq!(trace.promise.text.as_bytes(), &document.bytes[start..end]);
        assert!(said.contains(&trace.promise.text));
        reopen(&fx, run_id, trace);

        let paraphrase = "A saved note is usually retained after saving.";
        assert!(matches!(
            holden_cognition::Promise::from_quote(
                &document,
                trace.promise.start,
                trace.promise.end,
                paraphrase,
            ),
            Err(holden_cognition::PromiseError::QuoteDiffers { .. })
        ));
    }
}
