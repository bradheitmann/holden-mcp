//! slice-2 integration tests: a real `rmcp` client over a real Unix socket,
//! real stdio (the server binary as a child process) and loopback TCP. The
//! caller's uid is the real kernel-reported peer uid of this test process;
//! each test maps it to a role through the sealed config.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::fs::{self, DirBuilder, Permissions};
    use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::process::Stdio;
    use std::sync::Arc;
    use std::time::Duration;

    use holden_mcp::{
        Clock, ConfigError, Core, Log, ManualClock, Role, ServerConfig, StartError, TOOLS,
        loopback_bind, serve_socket, serve_tcp, workflow::WORKFLOW_TOOLS,
    };
    use holden_seal::seal::seal;
    use holden_seal::{
        BuilderNonce, DrawInputs, EpochKeypair, EpochPublicKey, EvaluatedCommit, FileKeyStore,
        KeyMaterial, KeyName, KeyStore, MomentParams, NonceCommitment, Plan, PlanCommit, PlanId,
        PlanStep, Reveal, SealKey, ServerNonce, VrfOutput, VrfProof, plan_commit,
    };
    use rmcp::model::{CallToolRequestParams, CallToolResult};
    use rmcp::service::RunningService;
    use rmcp::transport::TokioChildProcess;
    use rmcp::{RoleClient, ServiceExt};
    use serde_json::{Value, json};
    use tokio::io::AsyncReadExt;

    type Client = RunningService<RoleClient, ()>;

    /// Distinctive, non-repeating bytes, so a pattern scan cannot match by accident.
    const fn seeded(k: u8) -> [u8; 32] {
        let mut b = [0u8; 32];
        let mut i = 0;
        while i < 32 {
            b[i] = (i as u8).wrapping_mul(37).wrapping_add(k);
            i += 1;
        }
        b
    }
    const EPOCH_SEED: [u8; 32] = seeded(11);
    const SEAL_KEY: [u8; 32] = seeded(201);
    const COGNITION_MARKER: &str = "cognition-l1-7f3a-sealed-holdout-under-test";
    const ACTION_MARKER: &str = "SEALED-HOW-qx9-press-the-hidden-button-twice";
    const EXPECT_MARKER: &str = "SEALED-EXPECT-zr4-receipt-names-the-refund";
    const EVALUATED: &str = "0123456789abcdef0123456789abcdef01234567";
    const BUILDER: &str = "b0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecf";
    const WINDOW_MS: u64 = 60_000;

    fn uid() -> u32 {
        holden_seal::keystore::current_uid()
    }

    fn plan() -> Plan {
        Plan {
            id: PlanId(seeded(91)),
            cognition: COGNITION_MARKER.to_owned(),
            steps: vec![PlanStep {
                action: ACTION_MARKER.to_owned(),
                expect: EXPECT_MARKER.to_owned(),
            }],
        }
    }

    /// A scratch tree: keys (0600 in 0700), one sealed plan, and configs.
    struct Fixture {
        root: PathBuf,
        plan_commit: PlanCommit,
        ciphertext_hex: String,
    }

    impl Fixture {
        fn new(tag: &str) -> Self {
            // Short: Unix socket paths are limited to about 104 bytes.
            let root = std::env::temp_dir().join(format!("hm2-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)
                .unwrap();
            let store = FileKeyStore::open(root.join("k")).unwrap();
            store
                .put(
                    &KeyName::new("epoch").unwrap(),
                    &KeyMaterial::from_bytes(EPOCH_SEED),
                )
                .unwrap();
            store
                .put(
                    &KeyName::new("seal").unwrap(),
                    &KeyMaterial::from_bytes(SEAL_KEY),
                )
                .unwrap();
            let sealed = seal(&SealKey::new(KeyMaterial::from_bytes(SEAL_KEY)), &plan()).unwrap();
            let plans = root.join("p");
            fs::create_dir_all(&plans).unwrap();
            let file = holden_mcp::SealedPlanFile::from_sealed(&sealed);
            fs::write(
                plans.join("one.sealed.json"),
                serde_json::to_vec(&file).unwrap(),
            )
            .unwrap();
            Self {
                root,
                plan_commit: plan_commit(&sealed),
                ciphertext_hex: hex::encode(&sealed.ciphertext),
            }
        }

        fn plan_hex(&self) -> String {
            hex::encode(self.plan_commit.0)
        }

        /// Config JSON with `uid` mapped to `role` (`None`: map another uid,
        /// so this process is unmapped).
        fn config_json(&self, sock: &str, role: Option<Role>, extra: Value) -> Value {
            let roles = match role {
                Some(r) => json!([{ "uid": uid(), "role": r.as_str() }]),
                None => json!([{ "uid": uid().wrapping_add(1), "role": "evaluator" }]),
            };
            let mut v = json!({
                "socket_dir": self.root.join(sock),
                "key_dir": self.root.join("k"),
                "epoch_key": "epoch",
                "seal_key": "seal",
                "plans_dir": self.root.join("p"),
                "run_window_ms": WINDOW_MS,
                "roles": roles,
            });
            if let Value::Object(m) = extra {
                for (k, val) in m {
                    v[k] = val;
                }
            }
            v
        }

        fn config(&self, sock: &str, role: Option<Role>) -> Arc<ServerConfig> {
            let text = self.config_json(sock, role, json!({})).to_string();
            Arc::new(ServerConfig::from_json(&text).unwrap())
        }

        fn write_config(&self, name: &str, v: &Value) -> PathBuf {
            let p = self.root.join(name);
            fs::write(&p, v.to_string()).unwrap();
            fs::set_permissions(&p, Permissions::from_mode(0o600)).unwrap();
            p
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// One core on a manual clock, serving a socket per requested role.
    struct Served {
        fx: Fixture,
        core: Arc<Core>,
        clock: ManualClock,
        log: Log,
        listeners: Vec<holden_mcp::Listener>,
    }

    impl Served {
        fn new(tag: &str) -> Self {
            let fx = Fixture::new(tag);
            let clock = ManualClock::new(1_000_000);
            let log = Log::capture();
            let base = fx.config("s0", Some(Role::Evaluator));
            let core =
                Arc::new(Core::open(&base, Clock::Manual(clock.clone()), log.clone()).unwrap());
            Self {
                fx,
                core,
                clock,
                log,
                listeners: Vec::new(),
            }
        }

        /// Serve a socket where this process has `role`, and connect a client.
        async fn client(&mut self, sock: &str, role: Option<Role>) -> Client {
            let cfg = self.fx.config(sock, role);
            let path = cfg.socket_path();
            let l = serve_socket(Arc::clone(&self.core), cfg).unwrap();
            self.listeners.push(l);
            connect(&path).await
        }

        fn close_runs(&self) {
            self.clock.advance(WINDOW_MS);
        }
    }

    impl Drop for Served {
        fn drop(&mut self) {
            for l in &self.listeners {
                l.stop();
            }
        }
    }

    async fn connect(path: &Path) -> Client {
        let stream = tokio::net::UnixStream::connect(path).await.unwrap();
        ().serve(stream).await.unwrap()
    }

    /// A tool call's outcome and the exact bytes of the response.
    struct Answer {
        body: Value,
        is_error: bool,
        raw: String,
    }

    impl Answer {
        fn refusal(&self) -> Option<&str> {
            if self.is_error {
                self.body["refusal"].as_str()
            } else {
                None
            }
        }
        fn s(&self, k: &str) -> String {
            self.body[k].as_str().unwrap_or_default().to_owned()
        }
    }

    async fn call(c: &Client, tool: &str, args: Value) -> Answer {
        let mut p = CallToolRequestParams::new(tool.to_owned());
        if let Value::Object(m) = args {
            p = p.with_arguments(m);
        }
        match c.call_tool(p).await {
            Ok(r) => answer_of(&r),
            Err(e) => Answer {
                body: Value::Null,
                is_error: true,
                raw: e.to_string(),
            },
        }
    }

    fn answer_of(r: &CallToolResult) -> Answer {
        Answer {
            body: r.structured_content.clone().unwrap_or(Value::Null),
            is_error: r.is_error.unwrap_or(false),
            raw: serde_json::to_string(r).unwrap(),
        }
    }

    fn unhex<const N: usize>(s: &str) -> [u8; N] {
        hex::decode(s).unwrap().try_into().unwrap()
    }

    /// Verify a reveal or audit body with `holden-seal::verify` under the
    /// published epoch public key (taken from `brief`).
    fn verify(body: &Value, published: &str) -> MomentParams {
        let public = EpochPublicKey::from_bytes(&unhex::<32>(published)).unwrap();
        let s = |k: &str| body[k].as_str().unwrap().to_owned();
        let server_nonce = ServerNonce(unhex(&s("server_nonce")));
        assert_eq!(
            NonceCommitment::commit(&server_nonce).0,
            unhex::<32>(&s("commitment")),
            "the server nonce opens the published commitment"
        );
        let inputs = DrawInputs {
            plan_commit: PlanCommit(unhex(&s("plan_commit"))),
            evaluated_commit: EvaluatedCommit::from_hex(&s("evaluated_commit")).unwrap(),
            server_nonce,
            builder_nonce: BuilderNonce(unhex(&s("builder_nonce"))),
        };
        let p = &body["params"];
        let reveal = Reveal {
            output: VrfOutput(unhex(&s("output"))),
            proof: VrfProof(unhex::<64>(&s("proof"))),
            params: MomentParams {
                seed: p["seed"].as_u64().unwrap(),
                lens_slot: u16::try_from(p["lens_slot"].as_u64().unwrap()).unwrap(),
                step_order_salt: u32::try_from(p["step_order_salt"].as_u64().unwrap()).unwrap(),
                bytes: unhex(p["bytes"].as_str().unwrap()),
            },
        };
        holden_seal::verify(&public, &inputs, &reveal).unwrap()
    }

    /// commit_nonce then begin_run; returns the run id.
    async fn begin(c: &Client, plan: &str) -> String {
        let commit = call(c, "commit_nonce", json!({ "plan_commit": plan })).await;
        assert!(!commit.is_error, "{}", commit.raw);
        let run = call(
            c,
            "begin_run",
            json!({ "plan_commit": plan, "evaluated_commit": EVALUATED, "builder_nonce": BUILDER }),
        )
        .await;
        assert!(!run.is_error, "{}", run.raw);
        assert_eq!(run.s("commitment"), commit.s("commitment"));
        run.s("run_id")
    }

    async fn published_key(c: &Client) -> String {
        call(c, "brief", json!({})).await.s("epoch_public_key")
    }

    /// The secret byte patterns of a seeded run, in several encodings.
    fn encodings(bytes: &[u8]) -> Vec<Vec<u8>> {
        let dec = bytes
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        vec![
            bytes.to_vec(),
            hex::encode(bytes).into_bytes(),
            hex::encode_upper(bytes).into_bytes(),
            base64(bytes).into_bytes(),
            dec.into_bytes(),
        ]
    }

    fn base64(bytes: &[u8]) -> String {
        const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let b = [
                chunk[0],
                chunk.get(1).copied().unwrap_or(0),
                chunk.get(2).copied().unwrap_or(0),
            ];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            for i in 0..4 {
                if i <= chunk.len() {
                    out.push(char::from(A[((n >> (18 - 6 * i)) & 63) as usize]));
                } else {
                    out.push('=');
                }
            }
        }
        out
    }

    fn contains(hay: &[u8], needle: &[u8]) -> bool {
        !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
    }

    /// The patterns that must never leave the server: key material and plan
    /// plaintext.
    fn always_secret() -> Vec<(String, Vec<u8>)> {
        use schnorrkel::{ExpansionMode, MiniSecretKey};
        let expanded = MiniSecretKey::from_bytes(&EPOCH_SEED)
            .unwrap()
            .expand(ExpansionMode::Ed25519)
            .to_bytes();
        let mut v = Vec::new();
        for (name, bytes) in [
            ("vrf seed", EPOCH_SEED.to_vec()),
            ("vrf secret scalar", expanded[..32].to_vec()),
            ("vrf secret nonce", expanded[32..].to_vec()),
            ("aead key", SEAL_KEY.to_vec()),
        ] {
            for e in encodings(&bytes) {
                v.push((name.to_owned(), e));
            }
        }
        for m in [COGNITION_MARKER, ACTION_MARKER, EXPECT_MARKER] {
            v.push((format!("plaintext {m}"), m.as_bytes().to_vec()));
        }
        v.push((
            "plaintext json".to_owned(),
            serde_json::to_vec(&plan()).unwrap(),
        ));
        v
    }

    /// The patterns of a run's parameters (secret before close).
    fn param_patterns(reveal: &Value) -> Vec<(String, Vec<u8>)> {
        let mut v = Vec::new();
        for k in ["output", "proof", "server_nonce"] {
            let bytes = hex::decode(reveal[k].as_str().unwrap()).unwrap();
            for e in encodings(&bytes) {
                v.push((k.to_owned(), e));
            }
        }
        let bytes = hex::decode(reveal["params"]["bytes"].as_str().unwrap()).unwrap();
        for e in encodings(&bytes) {
            v.push(("params.bytes".to_owned(), e));
        }
        v.push((
            "params.seed".to_owned(),
            reveal["params"]["seed"]
                .as_u64()
                .unwrap()
                .to_string()
                .into_bytes(),
        ));
        v
    }

    fn leaks(hay: &str, patterns: &[(String, Vec<u8>)]) -> Vec<String> {
        patterns
            .iter()
            .filter(|(_, p)| contains(hay.as_bytes(), p))
            .map(|(n, _)| n.clone())
            .collect()
    }

    // ---------------------------------------------------------------- P1

    /// P1: the tool list is exactly the server's tools (`TOOLS`: five in
    /// slice-2, seven since slice-4 added `submit_observation` and
    /// `verdict`), each with an object input schema. Fail-without mutant:
    /// drop `audit_reveal` from the router.
    #[tokio::test]
    async fn lists_exactly_five_tools() {
        let mut s = Served::new("list");
        let c = s.client("s", Some(Role::Evaluator)).await;
        let tools = c.list_all_tools().await.unwrap();
        let mut names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
        names.sort();
        let mut want: Vec<String> = TOOLS
            .iter()
            .chain(&WORKFLOW_TOOLS)
            .map(|t| (*t).to_owned())
            .collect();
        want.sort();
        assert_eq!(names, want);
        for t in &tools {
            assert_eq!(
                t.input_schema.get("type").and_then(Value::as_str),
                Some("object"),
                "{} has no object input schema",
                t.name
            );
        }
        c.cancel().await.unwrap();
    }

    /// P1: a full run over the Unix socket, verified under the published key.
    #[tokio::test]
    async fn end_to_end_run_over_socket() {
        let mut s = Served::new("e2es");
        let c = s.client("s", Some(Role::Evaluator)).await;
        let published = published_key(&c).await;
        assert_eq!(
            published,
            hex::encode(
                EpochKeypair::from_seed(EPOCH_SEED)
                    .unwrap()
                    .public()
                    .to_bytes()
            )
        );
        let plan = s.fx.plan_hex();
        let run = begin(&c, &plan).await;
        let revealed = call(&c, "reveal_params", json!({ "run_id": run })).await;
        assert!(!revealed.is_error, "{}", revealed.raw);
        let p1 = verify(&revealed.body, &published);
        s.close_runs();
        let audit = call(&c, "audit_reveal", json!({ "run_id": run })).await;
        assert!(!audit.is_error, "{}", audit.raw);
        let p2 = verify(&audit.body, &published);
        assert_eq!(p1, p2);
        c.cancel().await.unwrap();
    }

    fn bin() -> &'static str {
        env!("CARGO_BIN_EXE_holden-mcp")
    }

    async fn spawn_stdio(config: &Path) -> (Client, tokio::process::ChildStderr) {
        let mut cmd = tokio::process::Command::new(bin());
        cmd.arg("serve").arg("--config").arg(config).arg("--stdio");
        let (transport, stderr) = TokioChildProcess::builder(cmd)
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        (().serve(transport).await.unwrap(), stderr.unwrap())
    }

    /// P1 (and P2: stdio is evaluator-only and must be enabled): a full run
    /// over real stdio against the server binary. The binary runs on the
    /// system clock with a 3 s window. The reveal is made right after
    /// `begin_run`, and close is observed by polling `audit_reveal` (up to
    /// 20 s) until it stops answering `run_open`, not by a fixed sleep.
    #[tokio::test]
    async fn end_to_end_run_over_stdio() {
        let fx = Fixture::new("e2eio");
        // Not enabled: the binary refuses stdio at startup with a typed error.
        let off = fx.write_config("off.json", &fx.config_json("s", None, json!({})));
        let out = std::process::Command::new(bin())
            .args(["serve", "--config"])
            .arg(&off)
            .arg("--stdio")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("stdio is not enabled"),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );

        // Enabled, with the role map naming no uid: the stdio caller is
        // still the evaluator, and only the evaluator.
        let mut v = fx.config_json("s", None, json!({ "stdio": { "enabled": true } }));
        v["run_window_ms"] = json!(3000);
        let on = fx.write_config("on.json", &v);
        let (c, mut stderr) = spawn_stdio(&on).await;
        let tools = c.list_all_tools().await.unwrap();
        assert_eq!(tools.len(), TOOLS.len() + WORKFLOW_TOOLS.len());
        let brief = call(&c, "brief", json!({})).await;
        assert_eq!(brief.s("caller_role"), "evaluator");
        let published = brief.s("epoch_public_key");
        let run = begin(&c, &fx.plan_hex()).await;
        let revealed = call(&c, "reveal_params", json!({ "run_id": run })).await;
        assert!(!revealed.is_error, "{}", revealed.raw);
        let p1 = verify(&revealed.body, &published);
        // Close: the run window expires. Poll until audit answers.
        let mut audit = call(&c, "audit_reveal", json!({ "run_id": run })).await;
        for _ in 0..400 {
            if audit.refusal() != Some("run_open") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            audit = call(&c, "audit_reveal", json!({ "run_id": run })).await;
        }
        assert!(!audit.is_error, "{}", audit.raw);
        assert_eq!(verify(&audit.body, &published), p1);
        let after = call(&c, "reveal_params", json!({ "run_id": run })).await;
        assert_eq!(after.refusal(), Some("run_closed"));
        c.cancel().await.unwrap();
        let mut log = String::new();
        let _ =
            tokio::time::timeout(Duration::from_secs(10), stderr.read_to_string(&mut log)).await;
        assert!(
            log.contains("tool=reveal_params caller=evaluator via=Stdio outcome=ok"),
            "{log}"
        );
        let leaked = leaks(&log, &always_secret());
        assert!(leaked.is_empty(), "stdio log leaks {leaked:?}");
        let leaked = leaks(&log, &param_patterns(&revealed.body));
        assert!(leaked.is_empty(), "stdio log leaks {leaked:?}");
    }

    // ---------------------------------------------------------------- P2

    /// P2: an implementer (this uid, mapped through config, read from the
    /// kernel) is refused `reveal_params` with `forbidden_role`, for a real
    /// open run, and the refusal carries no parameter, proof or sealed byte.
    /// Fail-without mutant: skip the role check in `reveal_params`.
    #[tokio::test]
    async fn implementer_refused_reveal_params() {
        let mut s = Served::new("impl");
        let eval = s.client("se", Some(Role::Evaluator)).await;
        let imp = s.client("si", Some(Role::Implementer)).await;
        assert_eq!(
            call(&imp, "brief", json!({})).await.s("caller_role"),
            "implementer"
        );
        let run = begin(&eval, &s.fx.plan_hex()).await;
        let refused = call(&imp, "reveal_params", json!({ "run_id": run })).await;
        assert_eq!(refused.refusal(), Some("forbidden_role"), "{}", refused.raw);
        let begin_refused = call(
            &imp,
            "begin_run",
            json!({ "plan_commit": s.fx.plan_hex(), "evaluated_commit": EVALUATED, "builder_nonce": BUILDER }),
        )
        .await;
        assert_eq!(begin_refused.refusal(), Some("forbidden_role"));
        let revealed = call(&eval, "reveal_params", json!({ "run_id": run })).await;
        assert!(!revealed.is_error);
        let mut secret = param_patterns(&revealed.body);
        secret.extend(always_secret());
        for e in encodings(&hex::decode(&s.fx.ciphertext_hex).unwrap()) {
            secret.push(("sealed ciphertext".to_owned(), e));
        }
        for a in [&refused, &begin_refused] {
            let leaked = leaks(&a.raw, &secret);
            assert!(leaked.is_empty(), "implementer refusal leaks {leaked:?}");
        }
        assert!(s.log.lines().iter().any(|l| l.contains(
            "tool=reveal_params caller=implementer via=Socket outcome=refused:forbidden_role"
        )));
        imp.cancel().await.unwrap();
        eval.cancel().await.unwrap();
    }

    /// P2: an evaluator (this uid mapped to evaluator) gets the parameters,
    /// and they verify.
    #[tokio::test]
    async fn evaluator_allowed_reveal_params() {
        let mut s = Served::new("eval");
        let c = s.client("s", Some(Role::Evaluator)).await;
        assert_eq!(
            call(&c, "brief", json!({})).await.s("caller_role"),
            "evaluator"
        );
        let published = published_key(&c).await;
        let run = begin(&c, &s.fx.plan_hex()).await;
        let revealed = call(&c, "reveal_params", json!({ "run_id": run })).await;
        assert!(!revealed.is_error, "{}", revealed.raw);
        verify(&revealed.body, &published);
        c.cancel().await.unwrap();
    }

    /// P2: an unmapped uid may call `brief` and nothing else.
    #[tokio::test]
    async fn unmapped_uid_brief_only() {
        let mut s = Served::new("unmap");
        let eval = s.client("se", Some(Role::Evaluator)).await;
        let run = begin(&eval, &s.fx.plan_hex()).await;
        let c = s.client("su", None).await;
        let brief = call(&c, "brief", json!({})).await;
        assert!(!brief.is_error);
        assert_eq!(brief.s("caller_role"), "unmapped");
        let plan = s.fx.plan_hex();
        for (tool, args) in [
            ("commit_nonce", json!({ "plan_commit": plan })),
            (
                "begin_run",
                json!({ "plan_commit": plan, "evaluated_commit": EVALUATED, "builder_nonce": BUILDER }),
            ),
            ("reveal_params", json!({ "run_id": run })),
            ("audit_reveal", json!({ "run_id": run })),
        ] {
            assert_eq!(
                call(&c, tool, args.clone()).await.refusal(),
                Some("unmapped_caller"),
                "{tool} before close"
            );
        }
        s.close_runs();
        assert_eq!(
            call(&c, "audit_reveal", json!({ "run_id": run }))
                .await
                .refusal(),
            Some("unmapped_caller"),
            "audit after close"
        );
        c.cancel().await.unwrap();
        eval.cancel().await.unwrap();
    }

    // ---------------------------------------------------------------- P3

    /// P3: `begin_run` is refused unless a commitment for the plan was
    /// recorded first.
    #[tokio::test]
    async fn begin_without_commitment_is_refused() {
        let mut s = Served::new("nocmt");
        let c = s.client("s", Some(Role::Evaluator)).await;
        let args = json!({ "plan_commit": s.fx.plan_hex(), "evaluated_commit": EVALUATED, "builder_nonce": BUILDER });
        let a = call(&c, "begin_run", args.clone()).await;
        assert_eq!(a.refusal(), Some("no_commitment"), "{}", a.raw);
        let other = hex::encode([0u8; 32]);
        let b = call(
            &c,
            "begin_run",
            json!({ "plan_commit": other, "evaluated_commit": EVALUATED, "builder_nonce": BUILDER }),
        )
        .await;
        assert_eq!(b.refusal(), Some("unknown_plan"));
        // After a commitment is recorded, the same call opens a run.
        call(
            &c,
            "commit_nonce",
            json!({ "plan_commit": s.fx.plan_hex() }),
        )
        .await;
        assert!(!call(&c, "begin_run", args).await.is_error);
        c.cancel().await.unwrap();
    }

    /// P3: one server nonce opens at most one run; the run record stores the
    /// commitment, both nonces, the evaluated commit and each step's time.
    /// Fail-without mutant: allow a second `begin_run` with the same nonce.
    #[tokio::test]
    async fn server_nonce_replay_is_refused() {
        let mut s = Served::new("replay");
        let c = s.client("s", Some(Role::Evaluator)).await;
        let plan = s.fx.plan_hex();
        let commit = call(&c, "commit_nonce", json!({ "plan_commit": plan })).await;
        s.clock.advance(5);
        let args =
            json!({ "plan_commit": plan, "evaluated_commit": EVALUATED, "builder_nonce": BUILDER });
        let first = call(&c, "begin_run", args.clone()).await;
        assert!(!first.is_error, "{}", first.raw);
        let replay = call(&c, "begin_run", args.clone()).await;
        assert_eq!(
            replay.refusal(),
            Some("server_nonce_used"),
            "{}",
            replay.raw
        );
        let other_builder = json!({ "plan_commit": plan, "evaluated_commit": EVALUATED, "builder_nonce": hex::encode([9u8; 32]) });
        assert_eq!(
            call(&c, "begin_run", other_builder).await.refusal(),
            Some("server_nonce_used")
        );
        // A fresh commitment opens a fresh run with a different id.
        call(&c, "commit_nonce", json!({ "plan_commit": plan })).await;
        let second = call(&c, "begin_run", args).await;
        assert!(!second.is_error);
        assert_ne!(second.s("run_id"), first.s("run_id"));

        // The run record.
        let run = first.s("run_id");
        s.clock.advance(7);
        call(&c, "reveal_params", json!({ "run_id": run })).await;
        s.close_runs();
        let audit = call(&c, "audit_reveal", json!({ "run_id": run })).await;
        assert!(!audit.is_error, "{}", audit.raw);
        assert_eq!(audit.s("commitment"), commit.s("commitment"));
        assert_eq!(audit.s("builder_nonce"), BUILDER);
        assert_eq!(audit.s("evaluated_commit"), EVALUATED);
        let nonce = ServerNonce(unhex(&audit.s("server_nonce")));
        assert_eq!(
            hex::encode(NonceCommitment::commit(&nonce).0),
            commit.s("commitment")
        );
        let r = &audit.body["record"];
        let t0 = commit.body["committed_at_ms"].as_u64().unwrap();
        assert_eq!(r["committed_at_ms"].as_u64(), Some(t0));
        assert_eq!(r["begun_at_ms"].as_u64(), Some(t0 + 5));
        assert_eq!(r["revealed_at_ms"].as_u64(), Some(t0 + 12));
        assert_eq!(r["closes_at_ms"].as_u64(), Some(t0 + 5 + WINDOW_MS));
        c.cancel().await.unwrap();
    }

    /// P3: `reveal_params` for a run that was never begun is refused, also
    /// for a commitment that never opened a run.
    #[tokio::test]
    async fn reveal_without_run_is_refused() {
        let mut s = Served::new("norun");
        let c = s.client("s", Some(Role::Evaluator)).await;
        let commit = call(
            &c,
            "commit_nonce",
            json!({ "plan_commit": s.fx.plan_hex() }),
        )
        .await;
        for id in [commit.s("commitment"), hex::encode([3u8; 32])] {
            let a = call(&c, "reveal_params", json!({ "run_id": id })).await;
            assert_eq!(a.refusal(), Some("unknown_run"), "{}", a.raw);
        }
        let bad = call(&c, "reveal_params", json!({ "run_id": "zz" })).await;
        assert_eq!(bad.refusal(), Some("bad_input"));
        c.cancel().await.unwrap();
    }

    // ---------------------------------------------------------------- P4

    /// P4: `audit_reveal` refuses an open run with `run_open`, for every
    /// mapped role. Fail-without mutant: let audit answer open runs.
    #[tokio::test]
    async fn audit_refuses_open_run() {
        let mut s = Served::new("aopen");
        let eval = s.client("se", Some(Role::Evaluator)).await;
        let imp = s.client("si", Some(Role::Implementer)).await;
        let run = begin(&eval, &s.fx.plan_hex()).await;
        let revealed = call(&eval, "reveal_params", json!({ "run_id": run })).await;
        let secret = param_patterns(&revealed.body);
        for c in [&eval, &imp] {
            let a = call(c, "audit_reveal", json!({ "run_id": run })).await;
            assert_eq!(a.refusal(), Some("run_open"), "{}", a.raw);
            assert!(leaks(&a.raw, &secret).is_empty());
        }
        imp.cancel().await.unwrap();
        eval.cancel().await.unwrap();
    }

    /// P4: after close, `audit_reveal` answers any mapped role, and the
    /// answer verifies; `reveal_params` is then refused with `run_closed`.
    /// Before close, the implementer obtains no parameter by any tool.
    #[tokio::test]
    async fn audit_after_close_verifies() {
        let mut s = Served::new("aclose");
        let eval = s.client("se", Some(Role::Evaluator)).await;
        let imp = s.client("si", Some(Role::Implementer)).await;
        let published = published_key(&imp).await;
        let plan = s.fx.plan_hex();
        let run = begin(&eval, &plan).await;
        let revealed = call(&eval, "reveal_params", json!({ "run_id": run })).await;
        let p = verify(&revealed.body, &published);
        let secret = param_patterns(&revealed.body);
        let mut before = Vec::new();
        for (tool, args) in [
            ("brief", json!({})),
            ("commit_nonce", json!({ "plan_commit": plan })),
            (
                "begin_run",
                json!({ "plan_commit": plan, "evaluated_commit": EVALUATED, "builder_nonce": BUILDER }),
            ),
            ("reveal_params", json!({ "run_id": run })),
            ("audit_reveal", json!({ "run_id": run })),
        ] {
            before.push(call(&imp, tool, args).await);
        }
        for a in &before {
            let leaked = leaks(&a.raw, &secret);
            assert!(
                leaked.is_empty(),
                "implementer before close leaks {leaked:?}"
            );
        }
        s.close_runs();
        assert_eq!(
            call(&eval, "reveal_params", json!({ "run_id": run }))
                .await
                .refusal(),
            Some("run_closed")
        );
        for c in [&eval, &imp] {
            let a = call(c, "audit_reveal", json!({ "run_id": run })).await;
            assert!(!a.is_error, "{}", a.raw);
            assert_eq!(verify(&a.body, &published), p);
        }
        imp.cancel().await.unwrap();
        eval.cancel().await.unwrap();
    }

    // ---------------------------------------------------------------- P5

    /// P5: every tool, every role, success and failure paths; no response or
    /// log line carries key material or plan plaintext, and none before close
    /// carries the parameters (except the evaluator's own `reveal_params`).
    /// Fail-without mutant: put the parameters in the `forbidden_role` text.
    #[tokio::test]
    async fn no_secret_in_any_response_or_log() {
        let mut s = Served::new("egress");
        let eval = s.client("se", Some(Role::Evaluator)).await;
        let imp = s.client("si", Some(Role::Implementer)).await;
        let unm = s.client("su", None).await;
        let plan = s.fx.plan_hex();
        let run = begin(&eval, &plan).await;
        let revealed = call(&eval, "reveal_params", json!({ "run_id": run })).await;
        assert!(!revealed.is_error);
        let params = param_patterns(&revealed.body);
        // The scanner is not vacuous: it finds the parameters where they belong.
        assert!(!leaks(&revealed.raw, &params).is_empty());

        let bad = "zz".to_owned();
        let unknown = hex::encode([5u8; 32]);
        let calls = |run: &str| -> Vec<(&'static str, Value)> {
            vec![
                ("brief", json!({})),
                ("commit_nonce", json!({ "plan_commit": plan })),
                ("commit_nonce", json!({ "plan_commit": bad })),
                ("commit_nonce", json!({ "plan_commit": unknown })),
                (
                    "begin_run",
                    json!({ "plan_commit": plan, "evaluated_commit": EVALUATED, "builder_nonce": BUILDER }),
                ),
                (
                    "begin_run",
                    json!({ "plan_commit": plan, "evaluated_commit": EVALUATED, "builder_nonce": BUILDER }),
                ),
                (
                    "begin_run",
                    json!({ "plan_commit": plan, "evaluated_commit": "XYZ", "builder_nonce": BUILDER }),
                ),
                ("begin_run", json!({ "plan_commit": plan })),
                ("reveal_params", json!({ "run_id": run })),
                ("reveal_params", json!({ "run_id": unknown })),
                ("reveal_params", json!({ "run_id": bad })),
                ("audit_reveal", json!({ "run_id": run })),
                ("audit_reveal", json!({ "run_id": unknown })),
                ("audit_reveal", json!({ "run_id": bad })),
                ("no_such_tool", json!({})),
            ]
        };
        let always = always_secret();
        let mut before: Vec<(String, Answer)> = Vec::new();
        let mut after: Vec<(String, Answer)> = Vec::new();
        let mut seen_ok = 0;
        let mut seen_refused = 0;
        for (role, c) in [
            ("evaluator", &eval),
            ("implementer", &imp),
            ("unmapped", &unm),
        ] {
            let listed = c.list_all_tools().await.unwrap();
            before.push((
                format!("{role} list_tools"),
                Answer {
                    body: Value::Null,
                    is_error: false,
                    raw: serde_json::to_string(&listed).unwrap(),
                },
            ));
            for (tool, args) in calls(&run) {
                let a = call(c, tool, args).await;
                if a.is_error {
                    seen_refused += 1
                } else {
                    seen_ok += 1
                }
                before.push((format!("{role} {tool}"), a));
            }
        }
        s.close_runs();
        for (role, c) in [
            ("evaluator", &eval),
            ("implementer", &imp),
            ("unmapped", &unm),
        ] {
            for (tool, args) in calls(&run) {
                let a = call(c, tool, args).await;
                after.push((format!("{role} {tool}"), a));
            }
        }
        assert!(
            seen_ok > 5 && seen_refused > 20,
            "{seen_ok} ok, {seen_refused} refused"
        );
        // Responses.
        for (what, a) in before.iter().chain(after.iter()) {
            let leaked = leaks(&a.raw, &always);
            assert!(leaked.is_empty(), "{what} leaks {leaked:?}");
        }
        for (what, a) in &before {
            // The one place parameters belong before close: the evaluator's
            // own successful `reveal_params`.
            if what == "evaluator reveal_params" && !a.is_error {
                continue;
            }
            let leaked = leaks(&a.raw, &params);
            assert!(
                leaked.is_empty(),
                "{what} leaks before close {leaked:?}: {}",
                a.raw
            );
        }
        // Captured logs.
        let lines = s.log.lines();
        assert!(lines.len() > 50, "{} log lines", lines.len());
        for l in &lines {
            let mut leaked = leaks(l, &always);
            leaked.extend(leaks(l, &params));
            assert!(leaked.is_empty(), "log line leaks {leaked:?}: {l}");
        }
        for c in [unm, imp, eval] {
            c.cancel().await.unwrap();
        }
    }

    // ---------------------------------------------------------------- P6

    /// P6: TCP binds only 127.0.0.1 or ::1; anything else is refused at
    /// startup with a typed error. Fail-without mutant: accept `0.0.0.0`.
    #[tokio::test]
    async fn non_loopback_bind_is_refused() {
        for addr in [
            "0.0.0.0",
            "::",
            "192.168.1.10",
            "10.0.0.1",
            "127.0.0.2",
            "::ffff:127.0.0.1",
            "localhost",
            "",
        ] {
            assert_eq!(
                loopback_bind(addr, 7411),
                Err(ConfigError::NonLoopbackBind {
                    addr: addr.to_owned()
                }),
                "{addr}"
            );
        }
        assert!(loopback_bind("127.0.0.1", 7411).is_ok());
        assert!(loopback_bind("::1", 7411).is_ok());
        assert_eq!(
            loopback_bind("127.0.0.1", 0),
            Err(ConfigError::PortNotConfigured)
        );

        let fx = Fixture::new("tcp");
        for addr in ["0.0.0.0", "::"] {
            let v = fx.config_json(
                "s",
                Some(Role::Evaluator),
                json!({ "tcp": { "addr": addr, "port": 7411 } }),
            );
            assert!(matches!(
                ServerConfig::from_json(&v.to_string()),
                Err(ConfigError::NonLoopbackBind { .. })
            ));
            // The binary refuses at startup, before binding anything.
            let path = fx.write_config("bad.json", &v);
            let out = std::process::Command::new(bin())
                .args(["serve", "--config"])
                .arg(&path)
                .stdin(Stdio::null())
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(1));
            let err = String::from_utf8_lossy(&out.stderr);
            assert!(err.contains("non_loopback_bind"), "{err}");
        }

        // A loopback config binds, on loopback, and its callers are unmapped.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let v = fx.config_json(
            "s",
            Some(Role::Evaluator),
            json!({ "tcp": { "addr": "127.0.0.1", "port": port } }),
        );
        let cfg = Arc::new(ServerConfig::from_json(&v.to_string()).unwrap());
        let core = Arc::new(Core::open(&cfg, Clock::System, Log::capture()).unwrap());
        let l = serve_tcp(core, cfg).await.unwrap();
        let addr = l.tcp_addr.unwrap();
        assert!(addr.ip().is_loopback());
        assert_eq!(addr.port(), port);
        let c = ().serve(tokio::net::TcpStream::connect(addr).await.unwrap()).await.unwrap();
        assert_eq!(
            call(&c, "brief", json!({})).await.s("caller_role"),
            "unmapped"
        );
        assert_eq!(
            call(&c, "commit_nonce", json!({ "plan_commit": fx.plan_hex() }))
                .await
                .refusal(),
            Some("unmapped_caller")
        );
        c.cancel().await.unwrap();
        l.stop();
    }

    /// P6: the default transport is a Unix socket with mode 0600 inside a
    /// 0700 directory; an existing directory open to group or world is refused.
    #[tokio::test]
    async fn socket_mode_is_0600() {
        let fx = Fixture::new("mode");
        let cfg = fx.config("s", Some(Role::Evaluator));
        let core = Arc::new(Core::open(&cfg, Clock::System, Log::capture()).unwrap());
        let l = serve_socket(Arc::clone(&core), Arc::clone(&cfg)).unwrap();
        let sock = fs::symlink_metadata(cfg.socket_path()).unwrap();
        assert!(sock.file_type().is_socket());
        assert_eq!(sock.mode() & 0o777, 0o600);
        assert_eq!(sock.uid(), uid());
        let dir = fs::symlink_metadata(cfg.socket_dir()).unwrap();
        assert_eq!(dir.mode() & 0o777, 0o700);
        let c = connect(&cfg.socket_path()).await;
        assert_eq!(
            call(&c, "brief", json!({})).await.s("caller_role"),
            "evaluator"
        );
        c.cancel().await.unwrap();
        l.stop();

        let open = fx.config("open", Some(Role::Evaluator));
        DirBuilder::new()
            .mode(0o755)
            .create(open.socket_dir())
            .unwrap();
        fs::set_permissions(open.socket_dir(), Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            serve_socket(core, open),
            Err(StartError::InsecureSocketPath { .. })
        ));
    }
}
