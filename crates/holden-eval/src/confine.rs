//! Confinement without Docker (docs/ARCHITECTURE.md §3.4; slice-4 P4).
//!
//! Every product process, and the product's build, runs through the
//! `holden-confine` launcher, which sets rlimits and then `exec`s the
//! program:
//!
//! | Platform | Backend |
//! |---|---|
//! | macOS | `sandbox-exec` with the committed SBPL profile `confine/macos.sb` (network denied, writes denied outside the world), then the launcher's rlimits |
//! | Linux | the launcher applies Landlock (writes denied outside the world; TCP bind and connect denied when the kernel's Landlock ABI is 4 or later) and rlimits |
//!
//! The launcher checks its own confinement before it `exec`s: it must not be
//! able to create a file next to the world, and on macOS it must not be able
//! to bind a socket. It writes a [`HelperReport`] into the world, which the
//! evaluator reads back and records. A network deny the kernel cannot give
//! becomes a typed [`Limitation`] in the verdict, never a silent pass.
//!
//! Slice-5 P6: product runs use a [`Profile::Product`]. On macOS its profile
//! (`confine/macos-product.sb`) kills a process that attempts a write outside
//! its world or any network operation, so the attempt is observed at the
//! boundary (the run ends by `SIGKILL`, which holden itself never sends
//! outside its recorded limits) and the rules fail the verdict with the
//! typed reason `confinement_violation`.
//!
//! Slice-7 P5: reads are confined too, so a product (or a build script)
//! cannot read the private root, a key store or the sealed side.
//!
//! - macOS product runs: `file-read*` outside the world, the launcher binary
//!   and the system paths a process needs is a violation (killed, observed);
//!   so is starting a child process, so every violation is the product's own.
//! - macOS builds (`confine/macos.sb`): reading a file's data outside the
//!   world, the toolchain, the developer directory and the system paths is a
//!   violation, and so are writes and network; each kills the offending
//!   process, which fails the build and so the verdict.
//! - Linux: Landlock grants read access only to the world, the system paths
//!   a process needs and, for builds, the toolchain ([`Confiner::with_read_roots`]).
//!
//! The launcher checks reads itself: listing the world's parent directory
//! must be refused. If it is not, the run carries the typed limitation
//! [`Limitation::ReadsNotConfined`], never a silent pass.
//!
//! What a backend cannot observe or confine is recorded as a typed
//! [`Limitation`]: on Linux, Landlock denies a write, read or TCP operation
//! silently, so violation attempts are not observed, and UDP is not
//! restricted. Every limitation caps a check's authority at `recommend`
//! (holden-ledger); a macOS verdict whose runs were all confined carries
//! none, so its checks can earn `block`.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::EvalError;
use crate::spawn::{self, Limits, Outcome, Spawn};
use crate::world::World;

/// The committed macOS profile (builds and direct confined runs: violations
/// are denied).
pub const MACOS_PROFILE: &str = include_str!("../confine/macos.sb");

/// The committed macOS profile of product runs (violations are denied and
/// the offending process is killed, so the attempt is observed).
pub const MACOS_PRODUCT_PROFILE: &str = include_str!("../confine/macos-product.sb");

/// The port the launcher's network self-check binds on 127.0.0.1. The
/// product profile denies exactly this bind without killing, so the
/// launcher survives its own check.
pub const SELF_CHECK_PORT: u16 = 1;

/// Which confinement a run gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    /// A build, or a direct confined run: violations are denied.
    Build,
    /// A product run of an evaluation: violations are denied and, where the
    /// backend can, observed (the process is killed).
    Product,
}

impl Profile {
    /// Its stable name (the launcher's `--profile` value).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Product => "product",
        }
    }

    /// Parse a stable name.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "build" => Some(Self::Build),
            "product" => Some(Self::Product),
            _ => None,
        }
    }
}

/// The program that applies the macOS profile.
pub const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

/// The confinement backend of this platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    /// `sandbox-exec` plus rlimits.
    MacosSandboxExec,
    /// Landlock plus rlimits.
    LinuxLandlock,
}

impl Backend {
    /// This platform's backend.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacosSandboxExec
        } else {
            Self::LinuxLandlock
        }
    }
}

/// A typed limitation of the confinement on this run.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case", deny_unknown_fields)]
pub enum Limitation {
    /// The kernel cannot deny TCP bind and connect (Landlock ABI below 4).
    NetworkNotDenied {
        /// The Landlock ABI in effect, or why none was.
        detail: String,
    },
    /// Landlock restricts TCP only; UDP and raw sockets are not denied.
    UdpNotRestricted,
    /// File reads are not confined: the run may read outside its world.
    ReadsNotConfined,
    /// A denied write or network operation is not observed: the backend
    /// refuses it silently, so an attempt cannot fail the verdict.
    ViolationAttemptsNotObserved,
    /// A violation is observed only when the product's own process makes
    /// it; a process the product started is killed without the evaluator
    /// seeing it.
    ChildViolationsNotObserved,
}

/// The typed limitations of `backend` under `profile`, besides any the
/// launcher adds at run time (for example [`Limitation::NetworkNotDenied`],
/// or [`Limitation::ReadsNotConfined`] when its read self-check fails).
///
/// Slice-7: reads are confined on both backends, so `reads_not_confined`
/// is no longer static. On macOS both profiles kill on a violation (a build
/// that violates fails), and product runs may not start a child process, so
/// neither `violation_attempts_not_observed` nor
/// `child_violations_not_observed` applies there.
#[must_use]
pub fn static_limitations(backend: Backend, _profile: Profile) -> Vec<Limitation> {
    let mut out = Vec::new();
    match backend {
        Backend::LinuxLandlock => {
            out.push(Limitation::UdpNotRestricted);
            out.push(Limitation::ViolationAttemptsNotObserved);
        }
        Backend::MacosSandboxExec => {}
    }
    out.sort();
    out
}

/// What the launcher reports about one run, written before it `exec`s.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelperReport {
    /// The backend.
    pub backend: Backend,
    /// The profile the run was confined under.
    pub profile: Profile,
    /// Whether a violation kills the offending process (so it is observed).
    pub kills_on_violation: bool,
    /// Whether writes outside the world were checked denied.
    pub writes_confined: bool,
    /// Whether reads outside the world were checked denied (slice-7).
    pub reads_confined: bool,
    /// Whether network was denied.
    pub network_denied: bool,
    /// The rlimits set.
    pub rlimits: Rlimits,
    /// Limitations.
    pub limitations: Vec<Limitation>,
    /// Set when the launcher refused to run the program.
    pub refused: Option<String>,
}

/// The rlimits the launcher sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rlimits {
    /// `RLIMIT_CPU`, seconds.
    pub cpu_secs: u64,
    /// `RLIMIT_FSIZE`, bytes.
    pub file_bytes: u64,
    /// `RLIMIT_NOFILE`.
    pub open_files: u64,
    /// `RLIMIT_CORE` (always 0).
    pub core_bytes: u64,
}

impl Rlimits {
    /// The rlimits for `limits`.
    #[must_use]
    pub const fn of(limits: &Limits) -> Self {
        Self {
            cpu_secs: limits.cpu_secs,
            file_bytes: limits.file_bytes,
            open_files: limits.open_files,
            core_bytes: 0,
        }
    }
}

/// The confinement summary recorded in a verdict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfinementReport {
    /// The backend.
    pub backend: Backend,
    /// SHA-256 of the macOS profile, when that backend ran.
    pub profile_sha256: Option<String>,
    /// Whether every confined run denied writes outside its world.
    pub writes_confined: bool,
    /// Whether every confined run denied network.
    pub network_denied: bool,
    /// Whether every product run was confined so that a violation kills the
    /// offending process (false when there was no product run).
    pub product_violations_kill: bool,
    /// The union of every run's limitations.
    pub limitations: Vec<Limitation>,
    /// How many processes ran confined.
    pub runs: u64,
}

/// One confined run's outcome and the launcher's report.
#[derive(Clone, Debug)]
pub struct ConfinedRun {
    /// The process outcome.
    pub outcome: Outcome,
    /// The launcher's report.
    pub report: HelperReport,
}

/// Runs programs confined inside a world.
#[derive(Debug)]
pub struct Confiner {
    helper: PathBuf,
    toolchain: Option<PathBuf>,
    registry: Option<PathBuf>,
    runs: AtomicU64,
}

/// `path`, canonical when it can be resolved. The sandbox matches canonical
/// paths, so every path given to a profile is resolved first.
fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The active developer directory (`xcode-select -p`), resolved once, with
/// the Command Line Tools location when it cannot be read. Builds may read
/// it: the system C toolchain links the product.
fn developer_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let env = [("PATH".to_owned(), "/usr/bin:/bin".to_owned())];
        let found = spawn::scrubbed("/usr/bin/xcode-select", &env)
            .arg("-p")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| PathBuf::from(s.trim()));
        canonical(&found.unwrap_or_else(|| PathBuf::from("/Library/Developer/CommandLineTools")))
    })
}

impl Confiner {
    /// A confiner that uses the `holden-confine` launcher at `helper`.
    #[must_use]
    pub fn new(helper: PathBuf) -> Self {
        Self {
            helper: canonical(&helper),
            toolchain: None,
            registry: None,
            runs: AtomicU64::new(0),
        }
    }

    /// Also let builds read below `root` (slice-7 P5): the Rust toolchain a
    /// build runs. Product runs never get it.
    #[must_use]
    pub fn with_toolchain_root(mut self, root: &Path) -> Self {
        self.toolchain = Some(canonical(root));
        self
    }

    /// Also let offline builds read Cargo's registry directory. It is a
    /// distinct path: registry credentials and other Cargo configuration are
    /// outside this read allowlist. Writes remain world-only.
    #[must_use]
    pub fn with_cargo_registry(mut self, root: &Path) -> Self {
        self.registry = Some(canonical(root));
        self
    }

    /// The directory builds may read besides the world and the system.
    #[must_use]
    pub fn toolchain_root(&self) -> Option<&Path> {
        self.toolchain.as_deref()
    }

    /// Wrap `spec` so that it runs confined in `world` under `limits` and
    /// `profile`. The launcher writes its report to `report`.
    #[must_use]
    pub fn wrap(
        &self,
        world: &World,
        spec: Spawn,
        limits: &Limits,
        profile: Profile,
        report: &Path,
    ) -> Spawn {
        let mut args: Vec<OsString> = vec![
            "--world".into(),
            world.root().as_os_str().to_owned(),
            "--report".into(),
            report.as_os_str().to_owned(),
            "--profile".into(),
            profile.as_str().into(),
            "--cpu".into(),
            limits.cpu_secs.to_string().into(),
            "--fsize".into(),
            limits.file_bytes.to_string().into(),
            "--nofile".into(),
            limits.open_files.to_string().into(),
        ];
        if profile == Profile::Build
            && let Some(t) = &self.toolchain
        {
            args.push("--read".into());
            args.push(t.as_os_str().to_owned());
        }
        if profile == Profile::Build
            && let Some(registry) = &self.registry
        {
            args.push("--read".into());
            args.push(registry.as_os_str().to_owned());
        }
        args.push("--".into());
        args.push(spec.program.as_os_str().to_owned());
        args.extend(spec.args);
        match Backend::current() {
            Backend::MacosSandboxExec => {
                let parent = world.root().parent().unwrap_or(world.root());
                let define = |name: &str, value: &Path| -> [OsString; 2] {
                    ["-D".into(), format!("{name}={}", value.display()).into()]
                };
                let mut sb: Vec<OsString> = Vec::new();
                sb.push("-p".into());
                sb.push(
                    match profile {
                        Profile::Build => MACOS_PROFILE,
                        Profile::Product => MACOS_PRODUCT_PROFILE,
                    }
                    .into(),
                );
                sb.extend(define("WORLD", world.root()));
                sb.extend(define("HELPER", &self.helper));
                sb.extend(define(
                    "HELPERDIR",
                    self.helper.parent().unwrap_or(Path::new("/")),
                ));
                sb.extend(define("PARENT", parent));
                if profile == Profile::Build {
                    // Without a toolchain root, the parameter names the
                    // world, which builds may read anyway.
                    let toolchain = self.toolchain.as_deref().unwrap_or(world.root());
                    sb.extend(define("TOOLCHAIN", toolchain));
                    sb.extend(define(
                        "REGISTRY",
                        self.registry.as_deref().unwrap_or(world.root()),
                    ));
                    sb.extend(define("DEVELOPER", developer_dir()));
                }
                sb.push(self.helper.as_os_str().to_owned());
                sb.extend(args);
                Spawn {
                    program: PathBuf::from(SANDBOX_EXEC),
                    args: sb,
                    cwd: spec.cwd,
                    env: spec.env,
                    stdin: spec.stdin,
                }
            }
            Backend::LinuxLandlock => Spawn {
                program: self.helper.clone(),
                args,
                cwd: spec.cwd,
                env: spec.env,
                stdin: spec.stdin,
            },
        }
    }

    /// Run `spec` confined in `world` under `limits`, with the
    /// [`Profile::Build`] confinement (violations are denied).
    ///
    /// # Errors
    /// [`EvalError::Confinement`] when the launcher did not report, or
    /// refused to run the program; [`EvalError::Io`] when nothing started.
    pub fn run(
        &self,
        world: &World,
        spec: Spawn,
        limits: &Limits,
    ) -> Result<ConfinedRun, EvalError> {
        self.run_as(world, spec, limits, Profile::Build)
    }

    /// Run `spec` confined in `world` under `limits` and `profile`.
    ///
    /// # Errors
    /// As [`Confiner::run`].
    pub fn run_as(
        &self,
        world: &World,
        spec: Spawn,
        limits: &Limits,
        profile: Profile,
    ) -> Result<ConfinedRun, EvalError> {
        let n = self.runs.fetch_add(1, Ordering::SeqCst);
        let report_path = world.path("confine").join(format!("run-{n}.json"));
        let wrapped = self.wrap(world, spec, limits, profile, &report_path);
        let outcome =
            spawn::run(&wrapped, limits).map_err(|e| EvalError::Io("confined run", e.kind()))?;
        if std::env::var_os("HOLDEN_E2E_TRANSPORT_DIAGNOSTIC").is_some() {
            let bytes = serde_json::json!({
                "exit": &outcome.exit,
                "stdout": &outcome.stdout,
                "stderr": &outcome.stderr,
                "elapsed_ms": &outcome.elapsed_ms,
            });
            let _ = fs::write(
                report_path.with_extension("outcome.json"),
                serde_json::to_vec(&bytes).unwrap_or_default(),
            );
        }
        if std::env::var_os("HOLDEN_E2E_TRANSPORT_DIAGNOSTIC").is_some() && !report_path.is_file() {
            eprintln!(
                "diagnostic confiner: helper={} world={} profile={} report={}",
                self.helper.display(),
                world.root().display(),
                profile.as_str(),
                report_path.display()
            );
        }
        let report: HelperReport = fs::read(&report_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .ok_or_else(|| {
                EvalError::Confinement(format!(
                    "the launcher wrote no report (exit {:?}): {}",
                    outcome.exit,
                    outcome.stderr.text().trim()
                ))
            })?;
        if let Some(why) = &report.refused {
            return Err(EvalError::Confinement(why.clone()));
        }
        Ok(ConfinedRun { outcome, report })
    }
}

/// Fold launcher reports into the verdict's confinement summary.
#[must_use]
pub fn summarize(reports: &[HelperReport]) -> ConfinementReport {
    let backend = Backend::current();
    let mut limitations: Vec<Limitation> = reports
        .iter()
        .flat_map(|r| r.limitations.iter().cloned())
        .collect();
    limitations.sort();
    limitations.dedup();
    let product: Vec<&HelperReport> = reports
        .iter()
        .filter(|r| r.profile == Profile::Product)
        .collect();
    ConfinementReport {
        backend,
        profile_sha256: (backend == Backend::MacosSandboxExec).then(|| {
            let mut h = Sha256::new();
            h.update(MACOS_PROFILE.as_bytes());
            h.update(MACOS_PRODUCT_PROFILE.as_bytes());
            hex::encode(h.finalize())
        }),
        writes_confined: !reports.is_empty() && reports.iter().all(|r| r.writes_confined),
        network_denied: !reports.is_empty() && reports.iter().all(|r| r.network_denied),
        product_violations_kill: !product.is_empty()
            && product.iter().all(|r| r.kills_on_violation),
        limitations,
        runs: reports.len() as u64,
    }
}
