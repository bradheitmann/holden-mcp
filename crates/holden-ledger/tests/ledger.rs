//! slice-5 integration tests: a real SQLite ledger file, a real holden-mcp
//! server on a real Unix socket (with the signed ledger configured), a real
//! fixture git repository built from the committed notes product and its
//! overlays, real `cargo` builds and real confined product runs through the
//! real evaluator. Nothing under test is mocked: every control count comes
//! from a signed verdict the evaluator produced and the server accepted.
//!
//! When `HOLDEN_LEDGER_RECORD_DIR` is set, every ledger a test produces is
//! copied there, for the `holden ledger verify` reopen.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs::{self, DirBuilder};
    use std::os::unix::fs::DirBuilderExt;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use holden_canon::Doctrine;
    use holden_cognition::{
        Approval, HostableCognition, PlanningDocument, candidates, extract_promises,
    };
    use holden_eval::binding::{Binding, Probe};
    use holden_eval::confine::{Backend, Limitation, Profile, static_limitations};
    use holden_eval::fixture::{FixtureRepo, NOTES_FIXTURE};
    use holden_eval::rules::CheckOutcome;
    use holden_eval::{
        Decision, EvalRequest, Evaluator, FailReason, McpClient, Toolchain, notes_binding, replay,
    };
    use holden_gen::{DrawCorpus, DrawInputs, GenPlan, draw_lens, generate, seal_plan};
    use holden_ledger::controls::{self, Control, RanControl, Suite};
    use holden_ledger::feedback::{
        self, holdout_version, lens_weights, pending_triggers, weight_of,
    };
    use holden_ledger::measure::{LIMIT_KEY, check_key, integrity_key};
    use holden_ledger::records::{
        ControlKind, ControlRecord, Expect, Target, Trigger, holdout_of, kind, promise_of,
    };
    use holden_ledger::{
        Authority, Bar, Bars, CheckStats, Ledger, LedgerError, Measurements, Problem, Rate,
        TipAnchor, Z95, authority, capped, verdict_authority, wilson,
    };
    use holden_mcp::{Caller, Core, Listener, Log, Refusal, Role, ServerConfig, Via, serve_socket};
    use holden_mcp::{Clock, SealedPlanFile};
    use holden_seal::{
        DrawKey, EvaluatorKey, EvaluatorPublicKey, FileKeyStore, KeyMaterial, KeyName, KeyStore,
        OwnerKey, PlanCommit, SealKey, plan_commit,
    };
    use serde_json::json;

    const fn seeded(k: u8) -> [u8; 32] {
        let mut b = [0u8; 32];
        let mut i = 0;
        while i < 32 {
            b[i] = (i as u8).wrapping_mul(29).wrapping_add(k);
            i += 1;
        }
        b
    }
    const EPOCH_SEED: [u8; 32] = seeded(3);
    const SEAL_KEY: [u8; 32] = seeded(77);
    const OWNER_KEY: [u8; 32] = seeded(131);
    const DRAW_KEY: [u8; 32] = seeded(171);
    const EVALUATOR_KEY: [u8; 32] = seeded(213);
    const LEDGER_KEY: [u8; 32] = seeded(41);
    const OTHER_KEY: [u8; 32] = seeded(59);

    fn crate_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn synthetic() -> PathBuf {
        crate_dir().join("../holden-cognition/fixtures/project-synthetic")
    }

    fn corpus() -> DrawCorpus {
        DrawCorpus::load(&synthetic().join("lens")).unwrap()
    }

    fn scratch(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("hm5-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&root)
            .unwrap();
        root
    }

    /// The real `holden-confine` launcher of holden-eval. It is another
    /// package's binary, so it is built (once per test process) into this
    /// test's own target directory with the same cargo.
    fn confine_helper() -> PathBuf {
        static HELPER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        HELPER
            .get_or_init(|| {
                let exe = std::env::current_exe().unwrap();
                // <target>/<profile>/deps/<test binary>
                let profile_dir = exe.parent().and_then(Path::parent).unwrap();
                let target = profile_dir.parent().unwrap();
                let mut cmd = std::process::Command::new(env!("CARGO"));
                cmd.args([
                    "build",
                    "--locked",
                    "--quiet",
                    "-p",
                    "holden-eval",
                    "--bin",
                    "holden-confine",
                ])
                .arg("--manifest-path")
                .arg(crate_dir().join("../../Cargo.toml"))
                .env("CARGO_TARGET_DIR", target);
                if profile_dir.file_name().is_some_and(|n| n == "release") {
                    cmd.arg("--release");
                }
                let status = cmd.status().unwrap();
                assert!(status.success(), "building holden-confine failed");
                let helper = profile_dir.join("holden-confine");
                assert!(helper.is_file(), "{}", helper.display());
                helper
            })
            .clone()
    }

    fn key(bytes: [u8; 32]) -> EvaluatorKey {
        EvaluatorKey::from_material(&KeyMaterial::from_bytes(bytes))
    }

    /// Copy a produced ledger to `HOLDEN_LEDGER_RECORD_DIR`, when set.
    fn keep_ledger(name: &str, path: &Path) {
        if let Some(dir) = std::env::var_os("HOLDEN_LEDGER_RECORD_DIR") {
            let dir = PathBuf::from(dir);
            fs::create_dir_all(&dir).unwrap();
            fs::copy(
                path,
                dir.join(format!("{name}-{}.sqlite", std::process::id())),
            )
            .unwrap();
        }
    }

    /// The approved plan for promise `k` of the synthetic planning document
    /// (first candidate cognition, owner-approved), drawn under the draw key
    /// from `corpus` at `epoch`.
    fn plan_for(store: &FileKeyStore, k: usize, epoch: u64, corpus: &DrawCorpus) -> GenPlan {
        let doctrine = Doctrine::holden();
        let doc = PlanningDocument::read(&synthetic().join("PLANNING.md"), "PLANNING.md").unwrap();
        let promises = extract_promises(&doc).unwrap();
        let owner = OwnerKey::load(store, &KeyName::new("owner").unwrap()).unwrap();
        let draw_key = DrawKey::load(store, &KeyName::new("draw").unwrap()).unwrap();
        let p = &promises[k];
        let c = candidates(&doctrine, p, None).into_iter().next().unwrap();
        let approval = Approval::sign(&owner, &c);
        let hosted =
            HostableCognition::admit(&doc, p, &c, Some(&approval), &owner.public()).unwrap();
        let inputs = DrawInputs {
            promise: hosted.promise().id,
            cognition: hosted.cognition().id,
            epoch,
        };
        let d = draw_lens(&draw_key, corpus, &inputs).unwrap();
        let approved: Vec<_> = promises.iter().collect();
        generate(&doctrine, &hosted, &approved, corpus, &d)
            .unwrap()
            .plan
    }

    /// A key store with every key a test uses.
    fn keystore(root: &Path) -> FileKeyStore {
        let store = FileKeyStore::open(root.join("k")).unwrap();
        for (name, bytes) in [
            ("epoch", EPOCH_SEED),
            ("seal", SEAL_KEY),
            ("owner", OWNER_KEY),
            ("draw", DRAW_KEY),
            ("evaluator", EVALUATOR_KEY),
            ("ledger", LEDGER_KEY),
        ] {
            store
                .put(
                    &KeyName::new(name).unwrap(),
                    &KeyMaterial::from_bytes(bytes),
                )
                .unwrap();
        }
        store
    }

    /// One test's world: sealed plans registered with a real server whose
    /// sealed config names a signed ledger, a fixture repository, an
    /// evaluator, and the test's own connection to the same ledger file.
    struct Fx {
        tag: String,
        root: PathBuf,
        _rt: tokio::runtime::Runtime,
        listener: Listener,
        socket: PathBuf,
        core: Arc<Core>,
        plans: Vec<(GenPlan, PlanCommit)>,
        repo: FixtureRepo,
        evaluator: Evaluator,
        evaluator_public: EvaluatorPublicKey,
        ledger: Ledger,
        ledger_key: EvaluatorKey,
    }

    /// The sealed config's bars in the telemetry test (distinct from the
    /// defaults, so the test can see they came from the config).
    fn configured_bars() -> Bars {
        Bars {
            block: Bar {
                catch_lower_min: 0.95,
                false_fail_upper_max: 0.04,
            },
            recommend: Bar {
                catch_lower_min: 0.40,
                false_fail_upper_max: 0.30,
            },
            min_broken_trials: 2,
            min_good_trials: 2,
        }
    }

    impl Fx {
        /// `make` builds the plans from the key store; `binding` is the
        /// evaluator's product binding.
        fn new(
            tag: &str,
            make: impl FnOnce(&FileKeyStore) -> Vec<GenPlan>,
            binding: Binding,
            bars: Option<Bars>,
        ) -> Self {
            let root = scratch(tag);
            let store = keystore(&root);
            let seal_key = SealKey::new(store.get(&KeyName::new("seal").unwrap()).unwrap());
            let plans_dir = root.join("p");
            fs::create_dir_all(&plans_dir).unwrap();
            let mut plans = Vec::new();
            for (i, plan) in make(&store).into_iter().enumerate() {
                let (sealed, _) = seal_plan(&seal_key, &plan).unwrap();
                fs::write(
                    plans_dir.join(format!("plan-{i}.sealed.json")),
                    serde_json::to_vec(&SealedPlanFile::from_sealed(&sealed)).unwrap(),
                )
                .unwrap();
                plans.push((plan, plan_commit(&sealed)));
            }
            let evaluator_key =
                EvaluatorKey::load(&store, &KeyName::new("evaluator").unwrap()).unwrap();
            let evaluator_public = evaluator_key.public();
            let ledger_path = root.join("db").join("ledger.sqlite");
            let mut config = json!({
                "socket_dir": root.join("s"),
                "key_dir": root.join("k"),
                "epoch_key": "epoch",
                "seal_key": "seal",
                "plans_dir": plans_dir,
                "run_window_ms": 600_000,
                "roles": [{ "uid": holden_seal::keystore::current_uid(), "role": "evaluator" }],
                "verdict_key": hex::encode(evaluator_public.0),
                "ledger": { "db": ledger_path, "key": "ledger" },
            });
            if let Some(b) = bars {
                config["authority_bars"] = serde_json::to_value(b).unwrap();
            }
            let config = Arc::new(ServerConfig::from_json(&config.to_string()).unwrap());
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            let core = Arc::new(Core::open(&config, Clock::System, Log::capture()).unwrap());
            let listener = {
                let _guard = rt.enter();
                serve_socket(Arc::clone(&core), Arc::clone(&config)).unwrap()
            };
            let repo = FixtureRepo::create(
                &crate_dir().join("../..").join(NOTES_FIXTURE),
                &root.join("repo"),
            )
            .unwrap();
            let mut evaluator = Evaluator::new(
                evaluator_key,
                Toolchain {
                    cargo: PathBuf::from(env!("CARGO")),
                },
                confine_helper(),
                root.join("worlds"),
                binding,
            );
            // The `hangs` control is ended by this limit (bounded: 3 s per
            // hanging run instead of the 20 s product default).
            evaluator.limits.run_ms = 3_000;
            let ledger = Ledger::open(&ledger_path).unwrap();
            let ledger_key = EvaluatorKey::load(&store, &KeyName::new("ledger").unwrap()).unwrap();
            Self {
                tag: tag.to_owned(),
                socket: config.socket_path(),
                root,
                _rt: rt,
                listener,
                core,
                plans,
                repo,
                evaluator,
                evaluator_public,
                ledger,
                ledger_key,
            }
        }

        fn client(&self) -> McpClient {
            McpClient::connect(&self.socket).unwrap()
        }

        /// Run `controls` `reps` times under plan `k` at holdout `version`.
        fn run(
            &mut self,
            k: usize,
            controls: &[Control],
            reps: usize,
            version: u64,
        ) -> Vec<RanControl> {
            let client = self.client();
            let (plan, commit) = &self.plans[k];
            let suite = Suite {
                evaluator: &self.evaluator,
                client: &client,
                repo: &self.repo,
                plan,
                plan_commit: *commit,
                version,
            };
            controls::run_controls(&suite, controls, reps, &mut self.ledger, &self.ledger_key)
                .unwrap()
        }

        /// The in-scope controls of plan `k` named in `names` (all when
        /// empty).
        fn controls(&self, k: usize, names: &[&str]) -> Vec<Control> {
            let plan = &self.plans[k].0;
            let binding = self.evaluator.binding();
            controls::catalog(&self.repo, plan, binding)
                .unwrap()
                .into_iter()
                .filter(|c| controls::in_scope(c, plan, binding))
                .filter(|c| names.is_empty() || names.contains(&c.name.as_str()))
                .collect()
        }

        fn measured(&self) -> Measurements {
            Measurements::from_entries(&self.ledger.entries().unwrap()).unwrap()
        }
    }

    impl Drop for Fx {
        fn drop(&mut self) {
            self.listener.stop();
            keep_ledger(&self.tag, self.ledger.path());
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn save_promise() -> &'static str {
        promise_of(Probe::InterruptedSave).unwrap()
    }

    // ---------------------------------------------------------------- P1

    /// Open a raw SQLite connection to a ledger file, as someone with file
    /// access would, with the append-only triggers dropped.
    fn tamper(path: &Path) -> rusqlite::Connection {
        let c = rusqlite::Connection::open(path).unwrap();
        c.execute_batch(
            "DROP TRIGGER IF EXISTS ledger_records_no_update;
             DROP TRIGGER IF EXISTS ledger_records_no_delete;",
        )
        .unwrap();
        c
    }

    fn copy_of(src: &Path, root: &Path, name: &str) -> PathBuf {
        let p = root.join(format!("{name}.sqlite"));
        fs::copy(src, &p).unwrap();
        p
    }

    fn problems(path: &Path) -> Vec<(u64, Problem)> {
        Ledger::open(path)
            .unwrap()
            .verify(Some(&key(LEDGER_KEY).public()), None)
            .unwrap()
            .findings
            .into_iter()
            .map(|f| (f.seq, f.problem))
            .collect()
    }

    /// P1: SQLite in a ledger file, each record chained by SHA-256 of the
    /// previous record and signed with Ed25519. Any edit, deletion, reorder,
    /// or splice of past records (and a truncation, against a recorded tip)
    /// is detected by full verification, which names the record. Through
    /// SQLite itself, updates and deletes are refused.
    /// Fail-without mutant: drop the previous-hash link from the record.
    #[test]
    fn tampered_history_is_detected() {
        let root = scratch("tamper");
        let path = root.join("db").join("ledger.sqlite");
        let k = key(LEDGER_KEY);
        let mut ledger = Ledger::open(&path).unwrap();
        for i in 1..=6u64 {
            ledger
                .append(&k, "note", &format!("{{\"n\":{i}}}"))
                .unwrap();
        }
        let clean = ledger.verify(Some(&k.public()), None).unwrap();
        assert!(clean.holds(), "{:?}", clean.findings);
        assert_eq!(clean.records, 6);
        let tip = TipAnchor::parse(clean.tip.as_deref().unwrap()).unwrap();
        keep_ledger("tamper-clean", &path);

        // Append-only through SQLite.
        {
            let c = rusqlite::Connection::open(&path).unwrap();
            assert!(
                c.execute("UPDATE ledger_records SET body = '{}' WHERE seq = 2", [])
                    .is_err()
            );
            assert!(
                c.execute("DELETE FROM ledger_records WHERE seq = 2", [])
                    .is_err()
            );
        }

        // Edit.
        let edited = copy_of(&path, &root, "edited");
        tamper(&edited)
            .execute(
                "UPDATE ledger_records SET body = '{\"n\":30}' WHERE seq = 3",
                [],
            )
            .unwrap();
        assert_eq!(problems(&edited), vec![(3, Problem::Edited)]);

        // Delete.
        let deleted = copy_of(&path, &root, "deleted");
        tamper(&deleted)
            .execute("DELETE FROM ledger_records WHERE seq = 4", [])
            .unwrap();
        assert_eq!(
            problems(&deleted),
            vec![
                (4, Problem::Missing { from: 4, to: 4 }),
                (5, Problem::Unlinked)
            ]
        );

        // Reorder (swap records 2 and 5).
        let reordered = copy_of(&path, &root, "reordered");
        tamper(&reordered)
            .execute_batch(
                "UPDATE ledger_records SET seq = 100 WHERE seq = 2;
                 UPDATE ledger_records SET seq = 2 WHERE seq = 5;
                 UPDATE ledger_records SET seq = 5 WHERE seq = 100;",
            )
            .unwrap();
        let found = problems(&reordered);
        for seq in [2, 5] {
            assert!(
                found.contains(&(seq, Problem::Edited)),
                "record {seq} not named: {found:?}"
            );
        }

        // Splice: another copy of the ledger under the same key diverges at
        // record 4; its record 4 is validly hashed and signed, so only the
        // chain (record 5's link) catches it.
        let fork = copy_of(&path, &root, "fork");
        {
            let c = tamper(&fork);
            c.execute("DELETE FROM ledger_records WHERE seq >= 4", [])
                .unwrap();
        }
        Ledger::open(&fork)
            .unwrap()
            .append(&k, "note", "{\"n\":40}")
            .unwrap();
        let (fseq, fprev, fkind, fbody, fsigner, fsig, fhash): (
            i64,
            String,
            String,
            String,
            String,
            String,
            String,
        ) = rusqlite::Connection::open(&fork)
            .unwrap()
            .query_row(
                "SELECT seq, prev_hash, kind, body, signer, signature, hash FROM ledger_records WHERE seq = 4",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                    ))
                },
            )
            .unwrap();
        let spliced = copy_of(&path, &root, "spliced");
        {
            let c = tamper(&spliced);
            c.execute("DELETE FROM ledger_records WHERE seq = 4", [])
                .unwrap();
            c.execute(
                "INSERT INTO ledger_records (seq, prev_hash, kind, body, signer, signature, hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![fseq, fprev, fkind, fbody, fsigner, fsig, fhash],
            )
            .unwrap();
        }
        assert_eq!(problems(&spliced), vec![(5, Problem::Unlinked)]);

        // Truncation: caught against the recorded tip.
        let truncated = copy_of(&path, &root, "truncated");
        tamper(&truncated)
            .execute("DELETE FROM ledger_records WHERE seq = 6", [])
            .unwrap();
        let report = Ledger::open(&truncated)
            .unwrap()
            .verify(Some(&k.public()), Some(&tip))
            .unwrap();
        assert_eq!(
            report
                .findings
                .iter()
                .map(|f| (f.seq, f.problem.clone()))
                .collect::<Vec<_>>(),
            vec![(6, Problem::TipMissing)]
        );

        // Another signer's ledger is named under the pinned key.
        let other = root.join("other.sqlite");
        let mut o = Ledger::open(&other).unwrap();
        o.append(&key(OTHER_KEY), "note", "{}").unwrap();
        assert_eq!(problems(&other), vec![(1, Problem::OtherSigner)]);
        let _ = fs::remove_dir_all(&root);
    }

    /// P1: an append checks only the tip. A tampered past record does not
    /// stop an append (the chain is not re-read; full verification still
    /// names it), while a tampered tip, or a tip signed by another key,
    /// refuses the append.
    /// Fail-without mutant: re-verify the whole chain on every append.
    #[test]
    fn append_checks_tip_only() {
        let root = scratch("tip");
        let path = root.join("ledger.sqlite");
        let k = key(LEDGER_KEY);
        let mut ledger = Ledger::open(&path).unwrap();
        for i in 1..=5u64 {
            ledger
                .append(&k, "note", &format!("{{\"n\":{i}}}"))
                .unwrap();
        }
        tamper(&path)
            .execute(
                "UPDATE ledger_records SET body = '{\"n\":20}' WHERE seq = 2",
                [],
            )
            .unwrap();
        let sixth = ledger.append(&k, "note", "{\"n\":6}").unwrap();
        assert_eq!(sixth.seq, 6);
        let report = ledger.verify(Some(&k.public()), None).unwrap();
        assert_eq!(
            report
                .findings
                .iter()
                .map(|f| (f.seq, f.problem.clone()))
                .collect::<Vec<_>>(),
            vec![(2, Problem::Edited)]
        );
        // Another key may not append after this ledger's tip.
        assert_eq!(
            ledger.append(&key(OTHER_KEY), "note", "{}").unwrap_err(),
            LedgerError::TipRefused {
                seq: 6,
                why: "was signed by another key"
            }
        );
        // A tampered tip refuses the append, naming the tip.
        tamper(&path)
            .execute(
                "UPDATE ledger_records SET body = '{\"n\":60}' WHERE seq = 6",
                [],
            )
            .unwrap();
        assert_eq!(
            ledger.append(&k, "note", "{\"n\":7}").unwrap_err(),
            LedgerError::TipRefused {
                seq: 6,
                why: "does not match its hash"
            }
        );
        assert_eq!(ledger.entries().unwrap().len(), 6);
        let _ = fs::remove_dir_all(&root);
    }

    // ---------------------------------------------------------------- P2

    /// The Wilson interval found independently: the two roots of
    /// `(p̂ − p)² = z² p (1 − p) / n` in `p`, by bisection.
    #[allow(clippy::cast_precision_loss)]
    fn wilson_by_roots(k: u64, n: u64) -> (f64, f64) {
        let ph = k as f64 / n as f64;
        let g = |p: f64| (ph - p).powi(2) - Z95 * Z95 * p * (1.0 - p) / n as f64;
        let root = |mut lo: f64, mut hi: f64| {
            for _ in 0..200 {
                let mid = (lo + hi) / 2.0;
                if (g(lo) <= 0.0) == (g(mid) <= 0.0) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            (lo + hi) / 2.0
        };
        let lower = if k == 0 { 0.0 } else { root(0.0, ph) };
        let upper = if k == n { 1.0 } else { root(ph, 1.0) };
        (lower, upper)
    }

    /// P2: the 95% Wilson bounds match published values and an independent
    /// root-finding computation, including the sample sizes at which the
    /// default block bars become reachable.
    /// Fail-without mutant: drop the `z²/4n²` term under the square root.
    #[test]
    fn wilson_bounds_correct() {
        assert_eq!(wilson(0, 0, Z95), (0.0, 1.0));
        // Published (four decimals): Newcombe (1998), and the classic
        // 0/10, 5/10 and 10/10 intervals.
        for (k, n, lo, hi) in [
            (0u64, 10u64, 0.0, 0.2775),
            (5, 10, 0.2366, 0.7634),
            (10, 10, 0.7225, 1.0),
            (81, 263, 0.2553, 0.3662),
            (15, 148, 0.0624, 0.1605),
            (0, 20, 0.0, 0.1611),
            (1, 29, 0.0061, 0.1718),
        ] {
            let (l, h) = wilson(k, n, Z95);
            assert!((l - lo).abs() < 6e-5, "{k}/{n} lower {l} vs {lo}");
            assert!((h - hi).abs() < 6e-5, "{k}/{n} upper {h} vs {hi}");
        }
        for n in 1..=120u64 {
            for k in [0, 1, n / 3, n / 2, n - 1, n] {
                let (l, h) = wilson(k, n, Z95);
                let (rl, rh) = wilson_by_roots(k, n);
                assert!((l - rl).abs() < 1e-9 && (h - rh).abs() < 1e-9, "{k}/{n}");
                assert!(l <= h);
            }
        }
        // Block at catch lower ≥ 0.90 needs 35 of 35; false-fail upper
        // ≤ 0.05 needs 0 of 73.
        assert!(wilson(34, 34, Z95).0 < 0.90 && wilson(35, 35, Z95).0 >= 0.90);
        assert!(wilson(0, 72, Z95).1 > 0.05 && wilson(0, 73, Z95).1 <= 0.05);
        let r = Rate { k: 3, n: 4 };
        assert_eq!(r.point(), Some(0.75));
        assert!((r.lower() - wilson(3, 4, Z95).0).abs() < f64::EPSILON);
    }

    /// The counts a test computes on its own from replayed verdict records:
    /// (plan-level catch k/n, plan-level false-fail k/n, per-check catch
    /// for the saving promise).
    type Counts = (u64, u64);

    fn recount(fx: &Fx, ran: &[RanControl]) -> (Counts, Counts, BTreeMap<String, Counts>) {
        let mut catch = (0, 0);
        let mut ff = (0, 0);
        let mut per_check: BTreeMap<String, (u64, u64)> = BTreeMap::new();
        for r in ran {
            let decided = replay(&r.record, Some(&fx.evaluator_public)).unwrap();
            let b = &r.record.body;
            let failed = decided.decision == Decision::Fail;
            match &r.control.expect {
                Expect::Good => {
                    ff.1 += 1;
                    ff.0 += u64::from(failed);
                }
                Expect::Broken { target } => {
                    catch.1 += 1;
                    catch.0 += u64::from(failed);
                    if let Target::Promise { text } = target
                        && text == save_promise()
                    {
                        for (c, res) in b.checks.iter().zip(&decided.results) {
                            if c.promise_text == *text {
                                let e = per_check.entry(check_key(&b.plan_id, &c.id)).or_default();
                                e.1 += 1;
                                e.0 += u64::from(res.outcome != CheckOutcome::Held);
                            }
                        }
                    }
                }
            }
        }
        (catch, ff, per_check)
    }

    /// P2: every in-scope sealed control of the saving promise's plan is run
    /// through the real evaluator against the fixture; the ledger's catch
    /// and false-fail counts equal the counts recomputed from the replayed
    /// signed verdicts, a real miss (`torn-write`, whose defect the probe
    /// cannot see) is counted as a miss, and a rerun with fresh parameters
    /// records the same counts.
    /// Fail-without mutant: count controls without running them (assume
    /// "broken ⇒ fail" in the tally).
    #[test]
    fn catch_rate_from_real_runs() {
        let mut fx = Fx::new(
            "catch",
            |s| vec![plan_for(s, 0, 0, &corpus())],
            notes_binding(),
            None,
        );
        let controls = fx.controls(0, &[]);
        let names: BTreeSet<&str> = controls.iter().map(|c| c.name.as_str()).collect();
        for n in [
            "keeps",
            "keeps-alt",
            "reconfig",
            "breaks",
            "refuses",
            "torn-write",
            "import-noop",
            "escapes",
            "sneaks",
            "hangs",
            "canary-leak",
        ] {
            assert!(names.contains(n), "{n} is not in the suite: {names:?}");
        }
        let first = fx.run(0, &controls, 1, 1);
        assert_eq!(first.len(), controls.len());
        let plan_id = fx.plans[0].0.id.to_hex();
        let m = fx.measured();
        let (catch, ff, per_check) = recount(&fx, &first);
        let plan = m.plans[&plan_id];
        assert_eq!((plan.catch.k, plan.catch.n), catch);
        assert_eq!((plan.false_fail.k, plan.false_fail.n), ff);
        assert!(!per_check.is_empty());
        for (key, (k, n)) in &per_check {
            let s = m.checks[key];
            assert_eq!((s.catch.k, s.catch.n), (*k, *n), "{key}");
        }
        // A real miss is a miss; real catches are catches.
        let decision = |name: &str| {
            first
                .iter()
                .find(|r| r.control.name == name)
                .unwrap()
                .record
                .decision()
        };
        assert_eq!(
            decision("torn-write"),
            Decision::Pass,
            "the probe cannot see a torn write"
        );
        assert_eq!(decision("breaks"), Decision::Fail);
        assert_eq!(decision("refuses"), Decision::Fail);
        assert_eq!(decision("keeps"), Decision::Pass);
        assert!(catch.0 < catch.1, "a known miss must lower the catch count");
        assert_eq!(ff.0, 0, "no known-good build failed");
        // The never-again controls are caught by their rules (on macOS the
        // confinement rule is observed; on Linux it is a recorded miss).
        let limit = m.integrity[LIMIT_KEY];
        assert_eq!((limit.catch.k, limit.catch.n), (1, 1));
        let canary = m.integrity[&integrity_key(FailReason::CanaryLeak).unwrap()];
        assert_eq!((canary.catch.k, canary.catch.n), (1, 1));
        let confinement = m.integrity[&integrity_key(FailReason::ConfinementViolation).unwrap()];
        assert_eq!(confinement.catch.n, 2, "escapes and sneaks");
        let reasons = |name: &str| {
            first
                .iter()
                .find(|r| r.control.name == name)
                .unwrap()
                .record
                .body
                .decided
                .reasons
                .clone()
        };
        if cfg!(target_os = "macos") {
            // `escapes` tries to leave on every import: observed and failed.
            // (`sneaks` tries only on the delete and export refusal paths,
            // which this plan may not exercise: whatever happened is what
            // is counted.)
            assert!(reasons("escapes").contains(&FailReason::ConfinementViolation));
        }
        // Rerun with fresh moment-of-test parameters: the same counts.
        let second = fx.run(0, &controls, 1, 1);
        let bodies = |ran: &[RanControl]| -> Vec<ControlRecord> {
            ran.iter().map(|r| r.body.clone()).collect()
        };
        let m1 = Measurements::from_records(&bodies(&first));
        let m2 = Measurements::from_records(&bodies(&second));
        assert_eq!(m1, m2, "the rerun recorded different counts");
        let planted = |ran: &[RanControl]| -> BTreeSet<String> {
            ran.iter()
                .map(|r| r.record.body.planted.export_name.clone())
                .collect()
        };
        assert!(planted(&first).is_disjoint(&planted(&second)));
        // The ledger holds both rounds and verifies.
        let all = fx.measured();
        assert_eq!(all.plans[&plan_id].catch.n, 2 * catch.1);
        assert!(
            fx.ledger
                .verify(Some(&key(LEDGER_KEY).public()), None)
                .unwrap()
                .holds()
        );
    }

    /// P2: the check rates roll up per lens concept and per cognition: each
    /// lens's rates are the sum over the checks its steps shaped, and the
    /// cognition's are the sum over every check of its plan.
    /// Fail-without mutant: key the lens rates by the check id instead of
    /// the lens concept.
    #[test]
    fn per_lens_and_cognition_rates() {
        let mut fx = Fx::new(
            "lens",
            |s| vec![plan_for(s, 0, 0, &corpus())],
            notes_binding(),
            None,
        );
        let plan = fx.plans[0].0.clone();
        let controls = fx.controls(0, &["keeps", "breaks", "torn-write", "export-noop"]);
        let ran = fx.run(0, &controls, 1, 1);
        assert!(!ran.is_empty());
        let m = fx.measured();
        let lenses = holden_ledger::records::lens_by_check(&plan);
        assert!(!lenses.is_empty(), "the plan's steps were shaped by lenses");
        let mut want_lens: BTreeMap<String, CheckStats> = BTreeMap::new();
        let mut want_cognition = CheckStats::default();
        for (key, s) in &m.checks {
            let id = key.rsplit('/').next().unwrap();
            if let Some(l) = lenses.get(id) {
                want_lens.entry(l.clone()).or_default().merge(s);
            }
            want_cognition.merge(s);
        }
        assert_eq!(m.lenses, want_lens);
        let cognition = plan.provenance.cognition.to_hex();
        assert_eq!(m.cognitions.len(), 1);
        assert_eq!(m.cognitions[&cognition], want_cognition);
        assert!(want_cognition.catch.n > 0 && want_cognition.false_fail.n > 0);
        for s in m.lenses.values() {
            assert!(s.catch.n + s.false_fail.n > 0);
            assert!(s.catch.lower() <= s.catch.upper());
        }
        // Every lens concept named is one the plan's draw picked.
        let picked: BTreeSet<String> = plan
            .provenance
            .lens_concepts
            .iter()
            .map(|c| c.to_hex())
            .collect();
        assert!(m.lenses.keys().all(|l| picked.contains(l)));
    }

    // ---------------------------------------------------------------- P3

    fn stats(caught: u64, broken: u64, failed: u64, good: u64) -> CheckStats {
        CheckStats {
            catch: Rate {
                k: caught,
                n: broken,
            },
            false_fail: Rate { k: failed, n: good },
        }
    }

    /// P3: authority comes only from the pessimistic Wilson bounds against
    /// the bars: a perfect point estimate on too few trials does not block,
    /// a catch lower bound just under the bar does not block, and a
    /// false-fail upper bound just over the bar does not block.
    /// Fail-without mutant: use the point estimate instead of the bound.
    #[test]
    fn authority_uses_pessimistic_bound() {
        let bars = Bars::DEFAULT;
        bars.validate().unwrap();
        // Point estimates 1.0 and 0.0, bounds 0.839 and 0.036: recommend.
        assert_eq!(
            authority(&stats(20, 20, 0, 100), &bars),
            Authority::Recommend
        );
        // 34/34 (lower 0.8985) misses the block bar; 35/35 (0.9011) meets it.
        assert_eq!(
            authority(&stats(34, 34, 0, 80), &bars),
            Authority::Recommend
        );
        assert_eq!(authority(&stats(35, 35, 0, 80), &bars), Authority::Block);
        // 0/72 good (upper 0.0507) misses the false-fail bar; 0/73 meets it.
        assert_eq!(
            authority(&stats(40, 40, 0, 72), &bars),
            Authority::Recommend
        );
        assert_eq!(authority(&stats(40, 40, 0, 73), &bars), Authority::Block);
        // 9 of 10 caught: point 0.9 at the bar, lower bound 0.596: recommend.
        assert_eq!(authority(&stats(9, 10, 0, 80), &bars), Authority::Recommend);
        // A high point estimate with a wide interval earns nothing.
        assert_eq!(authority(&stats(6, 10, 1, 10), &bars), Authority::Advise);
        // Out-of-range or inverted bars are refused.
        let mut bad = bars;
        bad.block.catch_lower_min = 0.4;
        assert!(bad.validate().is_err());
    }

    /// P3: an unmeasured or under-sampled check is advise-only, whatever its
    /// point estimate; a real verdict with no measured checks is advisory.
    /// Fail-without mutant: treat a check with no trials as fully measured
    /// (skip the minimum-trials rule).
    #[test]
    fn unmeasured_check_is_advisory_only() {
        let bars = Bars::DEFAULT;
        assert_eq!(authority(&CheckStats::default(), &bars), Authority::Advise);
        assert_eq!(authority(&stats(9, 9, 0, 200), &bars), Authority::Advise);
        assert_eq!(authority(&stats(200, 200, 0, 9), &bars), Authority::Advise);
        let lax = Bars {
            block: Bar {
                catch_lower_min: 0.0,
                false_fail_upper_max: 1.0,
            },
            recommend: Bar {
                catch_lower_min: 0.0,
                false_fail_upper_max: 1.0,
            },
            min_broken_trials: 1,
            min_good_trials: 1,
        };
        lax.validate().unwrap();
        // Even bars that anything meets give nothing without trials.
        assert_eq!(authority(&CheckStats::default(), &lax), Authority::Advise);
        assert_eq!(authority(&stats(0, 1, 0, 0), &lax), Authority::Advise);
        assert_eq!(authority(&stats(0, 1, 1, 1), &lax), Authority::Block);
        // A real verdict, no controls measured yet: advise.
        let fx = Fx::new(
            "unmeasured",
            |s| vec![plan_for(s, 0, 0, &corpus())],
            notes_binding(),
            None,
        );
        let (plan, commit) = &fx.plans[0];
        let keeps = fx.repo.commit("keeps").unwrap();
        let e = fx
            .evaluator
            .evaluate(
                &fx.client(),
                &EvalRequest {
                    repo: &fx.repo.path,
                    plan,
                    plan_commit: *commit,
                    evaluated_commit: &keeps,
                    baseline_commit: &keeps,
                },
            )
            .unwrap();
        let server = e.server.as_ref().unwrap();
        assert_eq!(server["authority"], "advise");
        let va = verdict_authority(&e.record, &Measurements::default(), &bars);
        assert_eq!(va.authority, Authority::Advise);
        assert_eq!(va.deciding.len(), e.record.body.checks.len());
        assert!(va.deciding.iter().all(|(_, a)| *a == Authority::Advise));
    }

    /// P6(c): a typed confinement limitation caps authority at `recommend`.
    /// Linux verdicts carry Landlock's limitations (UDP not restricted,
    /// attempts not observed); a real verdict whose checks have earned
    /// `block` is capped by its own recorded limitations, or, where the run
    /// recorded none (macOS since slice-7), by the limitation a run whose
    /// reads were not confined records.
    /// Fail-without mutant: ignore the limitations in `capped`.
    #[test]
    fn limitation_caps_authority() {
        let linux = static_limitations(Backend::LinuxLandlock, Profile::Product);
        for l in [
            Limitation::UdpNotRestricted,
            Limitation::ViolationAttemptsNotObserved,
        ] {
            assert!(linux.contains(&l), "{l:?}");
        }
        assert_eq!(capped(Authority::Block, &linux), Authority::Recommend);
        assert_eq!(capped(Authority::Advise, &linux), Authority::Advise);
        assert_eq!(capped(Authority::Block, &[]), Authority::Block);
        // A real verdict, with every deciding check measured at block level.
        let fx = Fx::new(
            "cap",
            |s| vec![plan_for(s, 0, 0, &corpus())],
            notes_binding(),
            None,
        );
        let (plan, commit) = &fx.plans[0];
        let keeps = fx.repo.commit("keeps").unwrap();
        let e = fx
            .evaluator
            .evaluate(
                &fx.client(),
                &EvalRequest {
                    repo: &fx.repo.path,
                    plan,
                    plan_commit: *commit,
                    evaluated_commit: &keeps,
                    baseline_commit: &keeps,
                },
            )
            .unwrap();
        let mut record = e.record.clone();
        if cfg!(target_os = "linux") {
            for l in [
                Limitation::UdpNotRestricted,
                Limitation::ViolationAttemptsNotObserved,
            ] {
                assert!(record.body.confinement.limitations.contains(&l), "{l:?}");
            }
        }
        if record.body.confinement.limitations.is_empty() {
            record.body.confinement.limitations = vec![Limitation::ReadsNotConfined];
        }
        let b = &record.body;
        let mut measured = Measurements::default();
        for c in &b.checks {
            measured
                .checks
                .insert(check_key(&b.plan_id, &c.id), stats(40, 40, 0, 80));
        }
        let va = verdict_authority(&record, &measured, &Bars::DEFAULT);
        assert!(va.deciding.iter().all(|(_, a)| *a == Authority::Block));
        assert_eq!(va.authority, Authority::Recommend);
        assert_eq!(va.capped_by, b.confinement.limitations);
    }

    /// slice-7 P5: with reads confined, no run carries
    /// `reads_not_confined`, and a check whose bounds, measured in the
    /// signed ledger from real control runs, meet the block bar reaches
    /// `block`: the server's answer to a real verdict says so. On macOS no
    /// other limitation applies, so the verdict's authority is `block`; on
    /// Linux the verdict is capped only by Landlock's two remaining
    /// limitations (UDP, unobserved attempts), never by reads.
    /// Fail-without mutant: remove the read restriction (the `file-read*`
    /// rules of `confine/macos-product.sb`; on Linux, the read rules
    /// `holden-confine` gives Landlock).
    #[test]
    fn block_reachable_when_bounds_met() {
        // Bars the controls below can meet with 2 trials each (95% Wilson:
        // 2 of 2 caught has lower bound 0.342; 0 of 4 failed has upper
        // bound 0.490).
        let bars = Bars {
            block: Bar {
                catch_lower_min: 0.30,
                false_fail_upper_max: 0.55,
            },
            recommend: Bar {
                catch_lower_min: 0.20,
                false_fail_upper_max: 0.60,
            },
            min_broken_trials: 2,
            min_good_trials: 2,
        };
        bars.validate().unwrap();
        let mut fx = Fx::new(
            "block",
            |s| vec![plan_for(s, 0, 0, &corpus())],
            notes_binding(),
            Some(bars),
        );
        // Known good, and every control that breaks a promise the plan
        // checks, except the real miss (`torn-write`).
        let controls: Vec<Control> = fx
            .controls(0, &[])
            .into_iter()
            .filter(|c| match &c.expect {
                Expect::Good => c.name != "reconfig",
                Expect::Broken {
                    target: Target::Promise { .. },
                } => c.name != "torn-write",
                Expect::Broken { .. } => false,
            })
            .collect();
        let ran = fx.run(0, &controls, 2, 1);
        assert!(ran.iter().all(|r| {
            !r.record
                .body
                .confinement
                .limitations
                .contains(&Limitation::ReadsNotConfined)
        }));
        let measured = fx.measured();
        let (plan, commit) = fx.plans[0].clone();
        let keeps = fx.repo.commit("keeps").unwrap();
        let client = fx.client();
        let e = fx
            .evaluator
            .evaluate(
                &client,
                &EvalRequest {
                    repo: &fx.repo.path,
                    plan: &plan,
                    plan_commit: commit,
                    evaluated_commit: &keeps,
                    baseline_commit: &keeps,
                },
            )
            .unwrap();
        assert_eq!(e.record.decision(), Decision::Pass);
        let limitations = e.record.body.confinement.limitations.clone();
        assert!(
            !limitations.contains(&Limitation::ReadsNotConfined),
            "{limitations:?}"
        );
        // Every deciding check earned block from its measured bounds.
        let want = verdict_authority(&e.record, &measured, &bars);
        assert!(!want.deciding.is_empty());
        assert!(
            want.deciding.iter().all(|(_, a)| *a == Authority::Block),
            "{:?}",
            want.deciding
        );
        let answer = e.server.unwrap();
        assert_eq!(
            answer["authority"],
            serde_json::to_value(want.authority).unwrap()
        );
        if cfg!(target_os = "macos") {
            assert!(limitations.is_empty(), "{limitations:?}");
            assert_eq!(answer["authority"], "block");
            assert_eq!(answer["authority_capped_by"], json!([]));
        } else {
            assert_eq!(
                limitations,
                vec![
                    Limitation::UdpNotRestricted,
                    Limitation::ViolationAttemptsNotObserved
                ]
            );
            assert_eq!(answer["authority"], "recommend");
        }
    }

    // ---------------------------------------------------------------- P4

    /// Picks of the keyed draw for the plan of promise 0 at `epoch` from
    /// `corpus`.
    fn picks(store: &FileKeyStore, corpus: &DrawCorpus, epoch: u64) -> Vec<String> {
        plan_for(store, 0, epoch, corpus)
            .provenance
            .lens_concepts
            .iter()
            .map(|c| c.to_hex())
            .collect()
    }

    /// P4(a): lens draw weights are recomputed from per-lens yield in the
    /// ledger and the next keyed draw uses them. The fixture loop runs
    /// twice: round one draws with the corpus weights, runs real controls,
    /// recomputes the weights; round two draws its plan with them, runs the
    /// same controls under it, and recomputes again. A lens whose checks
    /// miss is demoted but never zeroed.
    /// Fail-without mutant: ignore the weights in the draw.
    #[test]
    fn draw_uses_ledger_weights() {
        let base = corpus();
        let root = scratch("weights-keys");
        let store = keystore(&root);
        let names = [
            "keeps",
            "breaks",
            "refuses",
            "torn-write",
            "export-noop",
            "list-noop",
        ];

        // Round one.
        let mut fx = Fx::new(
            "weights1",
            |s| vec![plan_for(s, 0, 0, &base)],
            notes_binding(),
            None,
        );
        let ran = fx.run(0, &fx.controls(0, &names), 1, 1);
        assert!(!ran.is_empty());
        let (weighted, record1) = feedback::reweigh(&mut fx.ledger, &fx.ledger_key, &base).unwrap();
        let measured = fx.measured();
        assert_eq!(weighted.weights(), lens_weights(&measured, &base));
        assert!(weighted.weights().values().all(|w| *w >= 1));
        let unmeasured: BTreeMap<String, u32> = base
            .entries
            .iter()
            .map(|e| (e.concept.to_hex(), weight_of(e.weight, &Rate::default())))
            .collect();
        assert_ne!(record1.weights, unmeasured, "the ledger changed no weight");
        // The next keyed draw uses the weights: over a range of epochs, the
        // weighted draw differs from the corpus-weight draw.
        let differs: Vec<u64> = (0..48)
            .filter(|e| picks(&store, &weighted, *e) != picks(&store, &base, *e))
            .collect();
        assert!(!differs.is_empty(), "the draw ignored the ledger weights");
        // Never zeroed: a lens that misses every trial keeps weight 1.
        assert_eq!(weight_of(1, &Rate { k: 0, n: 1_000 }), 1);
        assert_eq!(weight_of(3, &Rate { k: 0, n: 1_000 }), 1);
        assert!(weight_of(3, &Rate { k: 50, n: 50 }) > weight_of(3, &Rate::default()));
        let round1 = picks(&store, &base, 0);
        let epoch2 = differs[0];
        let round2 = picks(&store, &weighted, epoch2);
        eprintln!("round 1 weights: {:?}", record1.weights);
        eprintln!("round 1 picks (epoch 0, corpus weights): {round1:?}");
        eprintln!("round 2 picks (epoch {epoch2}, ledger weights): {round2:?}");
        drop(fx);

        // Round two: a plan drawn with the ledger weights, run and measured.
        let weighted2 = weighted.clone();
        let mut fx2 = Fx::new(
            "weights2",
            move |s| vec![plan_for(s, 0, epoch2, &weighted2)],
            notes_binding(),
            None,
        );
        let drawn: Vec<String> = fx2.plans[0]
            .0
            .provenance
            .lens_concepts
            .iter()
            .map(|c| c.to_hex())
            .collect();
        assert_eq!(drawn, round2);
        let ran2 = fx2.run(0, &fx2.controls(0, &names), 1, 1);
        assert!(!ran2.is_empty());
        let (_, record2) = feedback::reweigh(&mut fx2.ledger, &fx2.ledger_key, &base).unwrap();
        eprintln!("round 2 weights: {:?}", record2.weights);
        let stored: Vec<_> = fx2
            .ledger
            .entries()
            .unwrap()
            .into_iter()
            .filter(|e| e.kind == kind::WEIGHTS)
            .collect();
        assert_eq!(stored.len(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    /// The plan of the delete promise, and a binding that (wrongly) observes
    /// that promise with the listing probe.
    fn misbound() -> Binding {
        let mut b = notes_binding();
        let delete = promise_of(Probe::DeleteAsksFirst).unwrap();
        for (t, p) in &mut b.promises {
            if *t == delete {
                *p = Probe::ListEvery;
            }
        }
        b
    }

    /// P4(b): vacuity triggers a re-plan. A binding that observes the delete
    /// promise with the wrong probe makes the delete plan pass the product
    /// that does nothing for deletes; the ledger's trigger scan fires
    /// `vacuity`, appends a re-plan item citing that control run, and bumps
    /// the holdout version; scanning again fires nothing new. With the right
    /// binding, the same do-nothing product fails and nothing fires.
    /// Fail-without mutant: do not treat a do-nothing pass as a trigger.
    #[test]
    fn vacuity_triggers_replan() {
        let mut fx = Fx::new(
            "vacuity",
            |s| vec![plan_for(s, 3, 0, &corpus())],
            misbound(),
            None,
        );
        let holdout = holdout_of(&fx.plans[0].0);
        let entries = fx.ledger.entries().unwrap();
        assert_eq!(holdout_version(&entries, &holdout).unwrap(), 1);
        let ran = fx.run(0, &fx.controls(0, &["delete-noop"]), 1, 1);
        assert_eq!(ran.len(), 1);
        assert_eq!(
            ran[0].record.decision(),
            Decision::Pass,
            "the plan is vacuous"
        );
        assert!(matches!(
            ran[0].body.kind,
            ControlKind::DoNothing | ControlKind::PlanTargeted
        ));
        let pending = pending_triggers(&fx.ledger.entries().unwrap()).unwrap();
        assert_eq!(pending.len(), 1, "{pending:?}");
        assert_eq!(pending[0].trigger, Trigger::Vacuity);
        assert_eq!(pending[0].evidence_seq, ran[0].entry.seq);
        let items = feedback::replan(&mut fx.ledger, &fx.ledger_key).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].trigger, Trigger::Vacuity);
        assert_eq!((items[0].from_version, items[0].to_version), (1, 2));
        assert!(!items[0].item.is_empty());
        let entries = fx.ledger.entries().unwrap();
        assert_eq!(holdout_version(&entries, &holdout).unwrap(), 2);
        assert!(
            feedback::replan(&mut fx.ledger, &fx.ledger_key)
                .unwrap()
                .is_empty()
        );
        drop(fx);

        let mut good = Fx::new(
            "vacuity-bound",
            |s| vec![plan_for(s, 3, 0, &corpus())],
            notes_binding(),
            None,
        );
        let ran = good.run(0, &good.controls(0, &["delete-noop"]), 1, 1);
        assert_eq!(ran[0].record.decision(), Decision::Fail);
        assert!(
            pending_triggers(&good.ledger.entries().unwrap())
                .unwrap()
                .is_empty()
        );
    }

    /// P4(b): a canary leak in an evaluated tree (a real evaluation, not a
    /// control) and a proven false result (a known-broken control that
    /// passed) each trigger a re-plan that bumps the holdout version.
    /// Fail-without mutant: skip verdict records in the trigger scan.
    #[test]
    fn canary_leak_and_proven_false_trigger_replan() {
        let mut fx = Fx::new(
            "triggers",
            |s| vec![plan_for(s, 0, 0, &corpus())],
            notes_binding(),
            None,
        );
        // Proven false: the torn write passes.
        let ran = fx.run(0, &fx.controls(0, &["torn-write"]), 1, 1);
        assert_eq!(ran[0].record.decision(), Decision::Pass);
        // A real evaluation of a commit carrying the plan's canary: the
        // server appends the verdict to the ledger.
        let (plan, commit) = fx.plans[0].clone();
        let token = holden_eval::canary::canaries(&plan)[0].clone();
        let keeps = fx.repo.commit("keeps").unwrap();
        let leaked = fx
            .repo
            .commit_on(
                &keeps,
                "builder copied the plan",
                &[("src/copied.txt", token.as_bytes())],
            )
            .unwrap();
        let e = fx
            .evaluator
            .evaluate(
                &fx.client(),
                &EvalRequest {
                    repo: &fx.repo.path,
                    plan: &plan,
                    plan_commit: commit,
                    evaluated_commit: &leaked,
                    baseline_commit: &keeps,
                },
            )
            .unwrap();
        assert!(
            e.record
                .body
                .decided
                .reasons
                .contains(&FailReason::CanaryLeak)
        );
        let entries = fx.ledger.entries().unwrap();
        assert!(entries.iter().any(|x| x.kind == kind::VERDICT));
        let triggers: BTreeSet<Trigger> = pending_triggers(&entries)
            .unwrap()
            .into_iter()
            .map(|p| p.trigger)
            .collect();
        assert_eq!(
            triggers,
            [Trigger::CanaryLeak, Trigger::ProvenFalse]
                .into_iter()
                .collect()
        );
        let items = feedback::replan(&mut fx.ledger, &fx.ledger_key).unwrap();
        assert_eq!(items.len(), 2);
        let holdout = holdout_of(&plan);
        assert!(
            items
                .iter()
                .all(|i| i.holdout == holdout && i.to_version == 2)
        );
        assert_eq!(
            holdout_version(&fx.ledger.entries().unwrap(), &holdout).unwrap(),
            2
        );
    }

    /// P4(c): the MCP `telemetry` tool (evaluator only) reports every rate
    /// with its sample size and Wilson bounds, equal to the ledger's counts,
    /// with each check's authority under the bars from the sealed config;
    /// the `verdict` answer carries the verdict's authority.
    /// Fail-without mutant: drop `n` from the reported rates.
    #[test]
    fn telemetry_reports_sample_sizes() {
        let mut fx = Fx::new(
            "telemetry",
            |s| vec![plan_for(s, 0, 0, &corpus())],
            notes_binding(),
            Some(configured_bars()),
        );
        let ran = fx.run(
            0,
            &fx.controls(0, &["keeps", "keeps-alt", "breaks", "refuses"]),
            1,
            1,
        );
        assert_eq!(ran.len(), 4);
        let client = fx.client();
        let t = client.call("telemetry", json!({})).unwrap();
        assert_eq!(t["format"], "holden.telemetry/1");
        assert_eq!(t["bars"], serde_json::to_value(configured_bars()).unwrap());
        assert_eq!(t["controls"], 4);
        let m = fx.measured();
        let checks = t["checks"].as_array().unwrap();
        assert_eq!(checks.len(), m.checks.len());
        for c in checks {
            let key = c["check"].as_str().unwrap();
            let s = m.checks[key];
            for (field, r) in [("catch", s.catch), ("false_fail", s.false_fail)] {
                assert_eq!(c[field]["n"], r.n, "{key} {field}");
                assert_eq!(c[field]["k"], r.k, "{key} {field}");
                assert!(c[field]["lower"].as_f64().unwrap() <= c[field]["upper"].as_f64().unwrap());
            }
            let a: Authority = serde_json::from_value(c["authority"].clone()).unwrap();
            assert_eq!(a, authority(&s, &configured_bars()));
        }
        let plan_id = fx.plans[0].0.id.to_hex();
        let plan = t["plans"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["plan"] == plan_id.as_str())
            .unwrap();
        assert_eq!(plan["catch"]["n"], 2);
        assert_eq!(plan["false_fail"]["n"], 2);
        assert!(!t["lenses"].as_array().unwrap().is_empty());
        assert!(!t["cognitions"].as_array().unwrap().is_empty());
        for l in t["lenses"].as_array().unwrap() {
            assert!(l["catch"]["n"].is_u64() && l["false_fail"]["n"].is_u64());
        }
        // The server appended each accepted verdict to the same ledger.
        assert_eq!(t["verdicts"], 4);
        // Evaluator only.
        let implementer = Caller {
            role: Some(Role::Implementer),
            uid: None,
            via: Via::Socket,
        };
        assert_eq!(fx.core.telemetry(&implementer), Err(Refusal::ForbiddenRole));
        let unmapped = Caller {
            role: None,
            uid: None,
            via: Via::Tcp,
        };
        assert_eq!(fx.core.telemetry(&unmapped), Err(Refusal::UnmappedCaller));
        // A verdict's answer carries its authority: the plan's checks now
        // have trials, so it is measured against the configured bars and
        // capped by the recorded limitations.
        let (plan, commit) = fx.plans[0].clone();
        let keeps = fx.repo.commit("keeps").unwrap();
        let e = fx
            .evaluator
            .evaluate(
                &client,
                &EvalRequest {
                    repo: &fx.repo.path,
                    plan: &plan,
                    plan_commit: commit,
                    evaluated_commit: &keeps,
                    baseline_commit: &keeps,
                },
            )
            .unwrap();
        let answer = e.server.unwrap();
        let measured = fx.measured();
        let want = verdict_authority(&e.record, &measured, &configured_bars());
        assert_eq!(
            answer["authority"],
            serde_json::to_value(want.authority).unwrap()
        );
        assert_eq!(
            answer["authority_capped_by"],
            serde_json::to_value(&want.capped_by).unwrap()
        );
    }
}
