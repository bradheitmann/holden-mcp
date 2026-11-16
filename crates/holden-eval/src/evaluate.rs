//! One evaluation, end to end (slice-4 P1–P5).
//!
//! 1. Over MCP (evaluator role): `brief` for the epoch key, then the run's
//!    `reveal_params`. The reveal is accepted only as a verified
//!    [`ServerReveal`] for this plan and evaluated commit, and the planted
//!    values are derived from it alone.
//! 2. A fresh world (unique id) is created.
//! 3. The evaluated commit's own tree is scanned for canaries; the public
//!    config is diffed against the baseline commit.
//! 4. The commit's tree is exported into a scratch directory inside the world and
//!    built from source, confined, with a scrubbed environment.
//! 5. Each probe a plan check needs is run against the built product,
//!    confined, and observed at the boundary.
//! 6. The rules decide; the record is signed; each observation's digest is
//!    submitted with `submit_observation`; the record is sent with `verdict`,
//!    which closes the run.

use std::path::{Path, PathBuf};

use holden_gen::GenPlan;
use holden_seal::{EvaluatorKey, PlanCommit};
use serde_json::{Value, json};

use crate::EvalError;
use crate::binding::{ActiveBinding, Binding, Probe};
use crate::canary;
use crate::client::{McpClient, ToolRefusal};
use crate::config;
use crate::confine::{self, Confiner, HelperReport, Profile};
use crate::drive::{Driver, Observed};
use crate::git::Git;
use crate::params::ServerReveal;
use crate::plant::Planted;
use crate::record::{ENGINE_VERSION, RECORD_FORMAT, VerdictBody, VerdictRecord};
use crate::rules::{self, Integrity, RULES_VERSION};
use crate::spawn::{BoundedBytes, Limits, Outcome, Spawn};
use crate::world::World;

/// The Rust toolchain the product is built with, chosen by the evaluator's
/// configuration (never taken from the caller's environment at run time).
#[derive(Clone, Debug)]
pub struct Toolchain {
    /// The `cargo` binary of the toolchain (not a rustup proxy).
    pub cargo: PathBuf,
}

impl Toolchain {
    fn bin_dir(&self) -> PathBuf {
        self.cargo
            .parent()
            .map_or_else(|| PathBuf::from("/usr/bin"), Path::to_path_buf)
    }

    /// The toolchain's root directory (the parent of its `bin`), which a
    /// confined build may read (slice-7 P5).
    #[must_use]
    pub fn root(&self) -> PathBuf {
        let cargo = std::fs::canonicalize(&self.cargo).unwrap_or_else(|_| self.cargo.clone());
        cargo
            .parent()
            .and_then(Path::parent)
            .map_or_else(|| PathBuf::from("/usr"), Path::to_path_buf)
    }

    /// The build's whole environment: toolchain locations and world paths.
    /// Incremental compilation is off: every build is a fresh world, and
    /// rustc's incremental session handling uses a local socket that the
    /// confinement treats as a network operation (slice-7). On macOS the SDK
    /// and the C linker are resolved once, outside confinement, and named
    /// here, so a confined build never has to look them up itself.
    #[must_use]
    pub fn env(&self, world: &World) -> Vec<(String, String)> {
        let bin = self.bin_dir();
        let mut env = vec![
            (
                "PATH".to_owned(),
                format!("{}:/usr/bin:/bin", bin.display()),
            ),
            ("RUSTC".to_owned(), bin.join("rustc").display().to_string()),
            ("HOME".to_owned(), world.path("home").display().to_string()),
            ("TMPDIR".to_owned(), world.path("tmp").display().to_string()),
            (
                "CARGO_HOME".to_owned(),
                world.path("cargo-home").display().to_string(),
            ),
            (
                "CARGO_TARGET_DIR".to_owned(),
                world.path("target").display().to_string(),
            ),
            ("CARGO_TERM_COLOR".to_owned(), "never".to_owned()),
            ("CARGO_INCREMENTAL".to_owned(), "0".to_owned()),
            ("LC_ALL".to_owned(), "C".to_owned()),
        ];
        env.extend(macos_link_env().iter().cloned());
        env
    }
}

/// On macOS, the SDK path and the C compiler that links the product,
/// resolved once with `xcrun` outside confinement (empty elsewhere, or when
/// `xcrun` cannot answer, in which case the build looks them up itself).
fn macos_link_env() -> &'static [(String, String)] {
    static ENV: std::sync::OnceLock<Vec<(String, String)>> = std::sync::OnceLock::new();
    ENV.get_or_init(|| {
        if !cfg!(target_os = "macos") {
            return Vec::new();
        }
        let ask = |args: &[&str]| {
            crate::spawn::scrubbed(
                "/usr/bin/xcrun",
                &[("PATH".to_owned(), "/usr/bin:/bin".to_owned())],
            )
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
        };
        let (Some(sdk), Some(cc), Some(ar), Some(ranlib)) = (
            ask(&["--sdk", "macosx", "--show-sdk-path"]),
            ask(&["--sdk", "macosx", "--find", "clang"]),
            ask(&["--sdk", "macosx", "--find", "ar"]),
            ask(&["--sdk", "macosx", "--find", "ranlib"]),
        ) else {
            return Vec::new();
        };
        let triple = format!("{}_apple_darwin", std::env::consts::ARCH).to_uppercase();
        vec![
            ("SDKROOT".to_owned(), sdk),
            ("AR".to_owned(), ar),
            ("RANLIB".to_owned(), ranlib),
            (format!("CARGO_TARGET_{triple}_LINKER"), cc),
        ]
    })
}

/// What to evaluate.
#[derive(Clone, Copy, Debug)]
pub struct EvalRequest<'a> {
    /// The repository holding the evaluated commit.
    pub repo: &'a Path,
    /// The unsealed plan (the evaluator is the sealed side).
    pub plan: &'a GenPlan,
    /// Its commit.
    pub plan_commit: PlanCommit,
    /// The evaluated commit (any revision git resolves).
    pub evaluated_commit: &'a str,
    /// The baseline commit for the public-config diff.
    pub baseline_commit: &'a str,
}

/// A finished evaluation.
#[derive(Debug)]
pub struct Evaluation {
    /// The signed record.
    pub record: VerdictRecord,
    /// The server's answer to `verdict`.
    pub server: Result<Value, ToolRefusal>,
    /// The world it ran in.
    pub world: World,
}

/// One manifest's build in a world.
#[derive(Debug)]
pub struct BuildPart {
    /// Host-relative manifest path.
    pub manifest: PathBuf,
    /// This manifest's real binary artifacts.
    pub products: Vec<PathBuf>,
    /// The build process's outcome.
    pub outcome: Outcome,
    /// The launcher's report.
    pub report: HelperReport,
}

/// A build in a world.
#[derive(Debug)]
pub struct Built {
    /// The first product binary, when every declared build succeeded.
    pub product: Option<PathBuf>,
    /// Every real binary artifact selected from compiler-artifact messages.
    pub products: Vec<PathBuf>,
    /// The first build process's outcome (retained for direct callers).
    pub outcome: Outcome,
    /// The first launcher's report (retained for direct callers).
    pub report: HelperReport,
    /// Each manifest's independent build.
    pub parts: Vec<BuildPart>,
}

/// The sealed evaluator.
#[derive(Debug)]
pub struct Evaluator {
    key: EvaluatorKey,
    toolchain: Toolchain,
    confiner: Confiner,
    scratch: PathBuf,
    binding: Binding,
    cargo_registry: Option<PathBuf>,
    /// The limits of every product run.
    pub limits: Limits,
    /// The limits of the build.
    pub build_limits: Limits,
}

fn refusal(r: ToolRefusal) -> EvalError {
    EvalError::Server(r)
}

impl Evaluator {
    /// An evaluator that signs with `key`, builds with `toolchain`, confines
    /// with the launcher at `confine_helper`, and makes worlds under `scratch`.
    #[must_use]
    pub fn new(
        key: EvaluatorKey,
        toolchain: Toolchain,
        confine_helper: PathBuf,
        scratch: PathBuf,
        binding: Binding,
    ) -> Self {
        // The sandbox matches canonical paths: run the toolchain's cargo
        // by its canonical path, and let builds read its root only.
        let root = toolchain.root();
        let toolchain = Toolchain {
            cargo: std::fs::canonicalize(&toolchain.cargo).unwrap_or(toolchain.cargo),
        };
        let registry = std::env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
            .map(|home| home.join("registry"))
            .filter(|path| path.is_dir())
            .map(|path| std::fs::canonicalize(&path).unwrap_or(path));
        let mut confiner = Confiner::new(confine_helper).with_toolchain_root(&root);
        if let Some(path) = &registry {
            confiner = confiner.with_cargo_registry(path);
        }
        Self {
            key,
            toolchain,
            confiner,
            scratch,
            binding,
            cargo_registry: registry,
            limits: Limits::PRODUCT,
            build_limits: Limits::BUILD,
        }
    }

    /// The confiner (for direct confined runs).
    #[must_use]
    pub fn confiner(&self) -> &Confiner {
        &self.confiner
    }

    /// The binding.
    #[must_use]
    pub fn binding(&self) -> &Binding {
        &self.binding
    }

    /// The scratch directory worlds are made under.
    #[must_use]
    pub fn scratch(&self) -> &Path {
        &self.scratch
    }

    /// Build `commit` of `repo` from source in a scratch directory inside
    /// `world`, confined.
    ///
    /// # Errors
    /// [`EvalError`] when git or the confined run fails to start.
    pub fn build(&self, world: &World, repo: &Path, commit: &str) -> Result<Built, EvalError> {
        let binding = ActiveBinding::load(repo, self.binding.clone())?;
        self.build_with(world, repo, commit, &binding)
    }

    fn build_with(
        &self,
        world: &World,
        repo: &Path,
        commit: &str,
        binding: &ActiveBinding,
    ) -> Result<Built, EvalError> {
        let git = Git::new(repo, &world.path("home"));
        let src = world.path("src");
        // The commit's tree is exported with a private index: no `.git` link
        // (which would name the repository, outside the world, that a
        // confined build may not read — slice-7 P5) and no worktree record
        // shared with concurrent evaluations of the same repository.
        git.export_tree(&src, commit, &world.path("git-index"))?;
        let generic = binding.cargo();
        if generic.is_some()
            && let Some(registry) = &self.cargo_registry
        {
            let home = world.path("cargo-home");
            std::fs::create_dir_all(&home)
                .map_err(|e| EvalError::Io("offline Cargo home", e.kind()))?;
            let cache = home.join("registry");
            if !cache.exists() {
                std::os::unix::fs::symlink(registry, &cache)
                    .map_err(|e| EvalError::Io("offline Cargo registry", e.kind()))?;
            }
        }
        let manifests = if generic.is_some() {
            binding.cargo_manifests()
        } else {
            vec![crate::product::CargoManifestBinding {
                manifest: PathBuf::from("Cargo.toml"),
                entry_points: Vec::new(),
            }]
        };
        let mut parts = Vec::new();
        let mut all_products = Vec::new();
        let mut primary_product = None;
        let mut all_ok = true;
        for manifest in manifests {
            let mut args = vec![
                "build".into(),
                "--offline".into(),
                "--locked".into(),
                "--manifest-path".into(),
                src.join(&manifest.manifest).into_os_string(),
            ];
            if generic.is_some() {
                args.extend(["--workspace".into(), "--message-format=json".into()]);
            } else {
                args.push("--quiet".into());
            }
            let spec = Spawn {
                program: self.toolchain.cargo.clone(),
                args,
                cwd: src.clone(),
                env: self.toolchain.env(world),
                stdin: Vec::new(),
            };
            let mut build_limits = self.build_limits;
            if generic.is_some() {
                // Compiler JSON includes dependency artifacts too; retain the
                // complete bounded stream so real binaries can be selected.
                build_limits.capture_bytes = 64 * 1024 * 1024;
            }
            let run = self.confiner.run(world, spec, &build_limits)?;
            let mut artifact_names = Vec::new();
            let mut products = Vec::new();
            if generic.is_some() {
                for line in run.outcome.stdout.text().lines() {
                    let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else {
                        continue;
                    };
                    if message["reason"] != "compiler-artifact"
                        || message["target"]["kind"]
                            .as_array()
                            .is_none_or(|kinds| !kinds.iter().any(|kind| kind == "bin"))
                    {
                        continue;
                    }
                    let Some(executable) = message["executable"].as_str() else {
                        continue;
                    };
                    let path = PathBuf::from(executable);
                    if path.starts_with(world.path("target")) && path.is_file() {
                        artifact_names.push(
                            message["target"]["name"]
                                .as_str()
                                .unwrap_or_default()
                                .to_owned(),
                        );
                        products.push(path);
                    }
                }
                artifact_names.sort();
                products.sort();
            }
            let discovered = generic.is_none_or(|_| {
                let mut expected: Vec<String> = manifest
                    .entry_points
                    .iter()
                    .map(|entry| entry.name.clone())
                    .collect();
                expected.sort();
                artifact_names == expected
            });
            let product = if generic.is_some() {
                products.first().cloned()
            } else {
                Some(world.path("target").join("debug").join(binding.product()))
            };
            let ok = run.outcome.exit.success()
                && discovered
                && product.as_ref().is_some_and(|path| path.is_file());
            if primary_product.is_none() {
                primary_product = product;
            }
            all_ok &= ok;
            all_products.extend(products.iter().cloned());
            parts.push(BuildPart {
                manifest: manifest.manifest,
                products: if ok { products } else { Vec::new() },
                outcome: run.outcome,
                report: run.report,
            });
        }
        let first = parts.first().ok_or_else(|| {
            EvalError::Fixture("the product binding selected no Cargo manifest".to_owned())
        })?;
        Ok(Built {
            product: if all_ok { primary_product } else { None },
            products: if all_ok { all_products } else { Vec::new() },
            outcome: first.outcome.clone(),
            report: first.report.clone(),
            parts,
        })
    }

    /// Open a run (commit a server nonce, begin it with a fresh builder
    /// nonce), then evaluate it.
    ///
    /// # Errors
    /// [`EvalError`]: a server refusal, a refused reveal, or a failure to
    /// prepare the world.
    pub fn evaluate(
        &self,
        client: &McpClient,
        req: &EvalRequest<'_>,
    ) -> Result<Evaluation, EvalError> {
        let run_id = self.begin(client, req)?;
        self.evaluate_run(client, req, &run_id)
    }

    /// `commit_nonce` then `begin_run` for `req`; returns the run id.
    ///
    /// # Errors
    /// [`EvalError`].
    pub fn begin(&self, client: &McpClient, req: &EvalRequest<'_>) -> Result<String, EvalError> {
        let plan = hex::encode(req.plan_commit.0);
        let scratch_home = self.scratch.join("git-home");
        std::fs::create_dir_all(&scratch_home)
            .map_err(|e| EvalError::Io("scratch directory", e.kind()))?;
        let commit = Git::new(req.repo, &scratch_home).resolve_commit(req.evaluated_commit)?;
        client
            .call("commit_nonce", json!({ "plan_commit": plan }))
            .map_err(refusal)?;
        let builder = holden_seal::random_bytes::<32>().map_err(|_| EvalError::NoRandomness)?;
        let run = client
            .call(
                "begin_run",
                json!({
                    "plan_commit": plan,
                    "evaluated_commit": commit,
                    "builder_nonce": hex::encode(builder),
                }),
            )
            .map_err(refusal)?;
        run["run_id"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| EvalError::Client("begin_run returned no run id".to_owned()))
    }

    /// Evaluate the already begun run `run_id`: [`Evaluator::prepare`] the
    /// record, then [`submit`] it.
    ///
    /// # Errors
    /// [`EvalError`]: a server refusal, a refused reveal, or a failure to
    /// prepare the world.
    pub fn evaluate_run(
        &self,
        client: &McpClient,
        req: &EvalRequest<'_>,
        run_id: &str,
    ) -> Result<Evaluation, EvalError> {
        let (record, world) = self.prepare(client, req, run_id)?;
        let server = submit(client, run_id, &record);
        Ok(Evaluation {
            record,
            server,
            world,
        })
    }

    /// Steps 1–6 up to the signed record, without submitting it.
    ///
    /// # Errors
    /// [`EvalError`]: a server refusal, a refused reveal, or a failure to
    /// prepare the world.
    pub fn prepare(
        &self,
        client: &McpClient,
        req: &EvalRequest<'_>,
        run_id: &str,
    ) -> Result<(VerdictRecord, World), EvalError> {
        let scratch_home = self.scratch.join("git-home");
        std::fs::create_dir_all(&scratch_home)
            .map_err(|e| EvalError::Io("scratch directory", e.kind()))?;
        let pre = Git::new(req.repo, &scratch_home);
        let commit = pre.resolve_commit(req.evaluated_commit)?;
        let baseline = pre.resolve_commit(req.baseline_commit)?;

        // 1. Parameters: from the server only, verified for this plan and commit.
        let brief = client.call("brief", json!({})).map_err(refusal)?;
        let published = brief["epoch_public_key"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let body = client
            .call("reveal_params", json!({ "run_id": run_id }))
            .map_err(refusal)?;
        let reveal = ServerReveal::from_reveal_body(&body, &published, &req.plan_commit, &commit)
            .map_err(EvalError::Params)?;
        let planted = Planted::derive(&reveal.params().bytes);

        // 2. A fresh world.
        let world = World::create(&self.scratch)?;
        let git = Git::new(req.repo, &world.path("home"));

        let binding = ActiveBinding::load(req.repo, self.binding.clone())?;
        // 3. Never-again controls on the evaluated tree.
        let canary = canary::scan_commit(&git, &commit, &canary::canaries(req.plan))?;
        let config_changes =
            config::diff_public_config(&git, &baseline, &commit, &binding.public_config())?;

        // 4–5. Build and probe, confined.
        let checks = binding.bind(req.plan);
        let (observations, reports) =
            self.observe(&world, req.repo, &commit, &binding, &checks, &planted)?;

        // 6. Decide, sign, submit.
        let confinement = confine::summarize(&reports);
        let decided = rules::decide(
            &checks,
            &planted,
            &observations,
            Integrity {
                canary: &canary,
                confinement: &confinement,
            },
        );
        let observation_digests = observations.iter().map(|o| o.digest()).collect();
        let body = VerdictBody {
            format: RECORD_FORMAT.to_owned(),
            engine_version: ENGINE_VERSION.to_owned(),
            rules_version: RULES_VERSION.to_owned(),
            product: binding.product().to_owned(),
            world_id: world.id().to_owned(),
            plan_id: req.plan.id.to_hex(),
            plan_commit: hex::encode(req.plan_commit.0),
            evaluated_commit: commit,
            baseline_commit: baseline,
            reveal: reveal.wire().clone(),
            planted,
            checks,
            limits: self.limits,
            confinement,
            canary,
            config_changes,
            observations,
            observation_digests,
            decided,
        };
        Ok((VerdictRecord::sign(body, &self.key), world))
    }

    fn observe(
        &self,
        world: &World,
        repo: &Path,
        commit: &str,
        binding: &ActiveBinding,
        checks: &[crate::binding::BoundCheck],
        planted: &Planted,
    ) -> Result<(Vec<crate::drive::Observation>, Vec<HelperReport>), EvalError> {
        let built = self.build_with(world, repo, commit, binding)?;
        let mut reports = Vec::new();
        let mut driver = Driver::new(
            world,
            &self.confiner,
            built.product.clone().unwrap_or_default(),
            self.limits,
        );
        for (index, part) in built.parts.iter().enumerate() {
            reports.push(part.report.clone());
            let label = if index == 0 {
                "build".to_owned()
            } else {
                format!("build:{}", part.manifest.display())
            };
            driver.push(
                None,
                &label,
                Observed::Process {
                    argv: vec!["cargo".to_owned(), "build".to_owned()],
                    stdin: BoundedBytes::keep(&[], 0, 0),
                    exit: part.outcome.exit,
                    stdout: BoundedBytes::keep(
                        &part.outcome.stdout.bytes().unwrap_or_default(),
                        part.outcome.stdout.len,
                        Limits::BUILD.capture_bytes,
                    ),
                    stderr: part.outcome.stderr.clone(),
                    elapsed_ms: part.outcome.elapsed_ms,
                },
            );
        }
        if built.product.is_some() {
            if checks
                .iter()
                .any(|c| c.probe == Some(Probe::CargoWorkspace))
            {
                let mut cli_index = 0;
                for (part_index, part) in built.parts.iter().enumerate() {
                    let spec = Spawn {
                        program: self.toolchain.cargo.clone(),
                        args: vec![
                            "test".into(),
                            "--workspace".into(),
                            "--offline".into(),
                            "--locked".into(),
                            "--quiet".into(),
                            "--manifest-path".into(),
                            world.path("src").join(&part.manifest).into_os_string(),
                        ],
                        cwd: world.path("src"),
                        env: self.toolchain.env(world),
                        stdin: Vec::new(),
                    };
                    let test =
                        self.confiner
                            .run_as(world, spec, &self.build_limits, Profile::Build)?;
                    reports.push(test.report);
                    let label = if part_index == 0 {
                        "workspace-tests".to_owned()
                    } else {
                        format!("workspace-tests:{}", part.manifest.display())
                    };
                    driver.push(
                        Some(Probe::CargoWorkspace),
                        &label,
                        Observed::Process {
                            argv: vec!["cargo".to_owned(), "test".to_owned()],
                            stdin: BoundedBytes::keep(&[], 0, 0),
                            exit: test.outcome.exit,
                            stdout: test.outcome.stdout,
                            stderr: test.outcome.stderr,
                            elapsed_ms: test.outcome.elapsed_ms,
                        },
                    );
                    for product in &part.products {
                        let mut runner =
                            Driver::new(world, &self.confiner, product.clone(), self.limits);
                        runner.run(
                            Probe::CargoWorkspace,
                            &format!("cli-{cli_index}"),
                            &["--help".to_owned()],
                            &[],
                        )?;
                        cli_index += 1;
                        reports.append(&mut runner.reports);
                        driver.observations.append(&mut runner.observations);
                    }
                }
                // Observation ids are canonical and unique even though each
                // isolated CLI driver numbers its own first observation.
                for (index, observation) in driver.observations.iter_mut().enumerate() {
                    observation.id = format!("obs-{}", index + 1);
                }
            } else {
                for p in Probe::ALL {
                    if checks.iter().any(|c| c.probe == Some(p)) {
                        driver.probe(p, planted)?;
                    }
                }
            }
        }
        reports.append(&mut driver.reports);
        Ok((driver.observations, reports))
    }
}

/// Submit each observation's digest with `submit_observation`, then the
/// record with `verdict`.
///
/// # Errors
/// The first [`ToolRefusal`].
pub fn submit(
    client: &McpClient,
    run_id: &str,
    record: &VerdictRecord,
) -> Result<Value, ToolRefusal> {
    for (o, d) in record
        .body
        .observations
        .iter()
        .zip(&record.body.observation_digests)
    {
        client.call(
            "submit_observation",
            json!({
                "run_id": run_id,
                "observation_id": o.id,
                "sha256": d,
                "len": o.len(),
            }),
        )?;
    }
    let text = serde_json::to_string(record).map_err(|e| ToolRefusal {
        code: "encode".to_owned(),
        message: e.to_string(),
    })?;
    client.call("verdict", json!({ "run_id": run_id, "record": text }))
}
