//! `cargo xtask gate <slice>`: fmt, clippy, tests and guard, each with its
//! own `CARGO_TARGET_DIR`, then a JSON run record in the private root.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use serde::Serialize;

use crate::forge;
use crate::guard::{self, GuardReport, Verdict};
use crate::util;

/// One gate step.
#[derive(Debug, Serialize)]
pub struct Step {
    /// Step name.
    pub name: &'static str,
    /// The exact command.
    pub command: String,
    /// The build directory it used.
    pub target_dir: String,
    /// `pass` or `fail`.
    pub status: &'static str,
    /// Wall time.
    pub duration_ms: u128,
}

/// Parsed test results.
#[derive(Debug, Default, Serialize)]
pub struct TestSummary {
    /// `<package>::<test path>`, sorted.
    pub names: Vec<String>,
    /// Sum of "passed" over every test binary.
    pub passed: u64,
    /// Sum of "failed".
    pub failed: u64,
    /// Sum of "ignored".
    pub ignored: u64,
}

/// The JSON run record.
#[derive(Debug, Serialize)]
pub struct RunRecord {
    /// Record schema.
    pub schema: &'static str,
    /// Slice id (one of [`SLICES`]).
    pub slice: &'static str,
    /// The base every step's `target_dir` sits under (`HOLDEN_TARGET_BASE`).
    pub target_base: String,
    /// Start time.
    pub started_utc: String,
    /// `std::env::consts::OS`.
    pub host_os: &'static str,
    /// `std::env::consts::ARCH`.
    pub arch: &'static str,
    /// `git rev-parse HEAD`.
    pub commit: String,
    /// True when `git status --porcelain` was empty.
    pub tree_clean: bool,
    /// `rustc -V`.
    pub rustc: String,
    /// Steps in order.
    pub steps: Vec<Step>,
    /// Test results.
    pub tests: TestSummary,
    /// Guard checks.
    pub guard: GuardReport,
    /// The guard verdict.
    pub guard_verdict: Verdict,
    /// `green`, `green_with_declared_limitations` or `red`.
    pub result: &'static str,
}

/// The slices that have a gate. Every slice runs the same steps; slices in
/// [`FORGE_SLICES`] also run the forge module's steps.
pub const SLICES: [&str; 26] = [
    "slice-0",
    "slice-1",
    "slice-1b",
    "slice-1c",
    "slice-2",
    "slice-3",
    "slice-4",
    "slice-5",
    "slice-6",
    "slice-7",
    "slice-8",
    "slice-9",
    "slice-10",
    "slice-11",
    "slice-12",
    "slice-12b",
    "slice-13",
    "slice-14",
    "slice-15",
    "slice-17",
    "slice-18",
    "slice-18a",
    "slice-18b",
    "slice-18e",
    "slice-18f",
    "slice-18g",
];

/// Slices whose gate also replays every verdict record the tests wrote
/// (`holden replay`, slice-4 P3 reopen).
pub const REPLAY_SLICES: [&str; 3] = ["slice-4", "slice-5", "slice-7"];

/// Slices whose gate also verifies every ledger the tests produced
/// (`holden ledger verify`, slice-5 P1 reopen) and runs `cargo xtask
/// selftest` (slice-5 P5).
pub const LEDGER_SLICES: [&str; 2] = ["slice-5", "slice-7"];

/// Slices whose gate also runs the forge module's fmt, clippy, tests,
/// `SHA256SUMS` verify and the export reopen (slice-6 P2).
pub const FORGE_SLICES: [&str; 1] = ["slice-6"];

/// Slices whose gate also exports the forge to a fresh directory and loads
/// the export with the typed pack loader (slice-3 P7 reopen).
pub const PACK_LOADER_SLICES: [&str; 1] = ["slice-3"];

/// The workspace packages whose tests the gate runs.
const PACKAGES: [&str; 10] = [
    "holden-canon",
    "holden-seal",
    "holden-cli",
    "holden-mcp",
    "holden-cognition",
    "holden-gen",
    "holden-eval",
    "holden-ledger",
    "holden-embed",
    "xtask",
];

/// Where the holden-ledger tests copy the ledgers they produce during a
/// gate run (`HOLDEN_LEDGER_RECORD_DIR`), for the verify reopen.
fn ledgers_dir(started: u64) -> PathBuf {
    util::target_base().join(format!(
        "holden-mcp-s5-ledgers-{}-{}",
        util::utc_compact(started),
        std::process::id()
    ))
}

/// The slice-5 P1 reopen: `holden ledger verify` on every ledger the tests
/// produced. Green only when at least one ledger was produced and every
/// one verifies.
fn ledger_verify_steps(root: &Path, ledgers: &Path, steps: &mut Vec<Step>) {
    let t = Instant::now();
    let mut files: Vec<PathBuf> = fs::read_dir(ledgers)
        .map(|d| {
            d.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "sqlite"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let args = [
        "run",
        "--locked",
        "--quiet",
        "-p",
        "holden-cli",
        "--",
        "ledger",
        "verify",
        "--db",
    ];
    let (_, text, dir) = cargo(root, "s5-ledger-verify", &args);
    let mut ok = !files.is_empty();
    println!(
        "gate: s5-ledger-verify: {} ledger(s) in {}",
        files.len(),
        ledgers.display()
    );
    for f in &files {
        let (mut cmd, _, _) = cargo(root, "s5-ledger-verify", &args);
        cmd.arg(f);
        let out = util::capture(&mut cmd);
        let passed = matches!(&out, Ok(o) if o.status.success());
        let said = out
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|l| l.starts_with("records:") || l.starts_with("verified:"))
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default();
        println!(
            "gate: s5-ledger-verify: {} {}: {said}",
            if passed { "ok" } else { "REFUSED" },
            f.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        );
        ok &= passed;
    }
    steps.push(Step {
        name: "s5-ledger-verify",
        command: format!("{text} <ledger> (for each ledger in {})", ledgers.display()),
        target_dir: dir.display().to_string(),
        status: if ok { "pass" } else { "fail" },
        duration_ms: t.elapsed().as_millis(),
    });
}

/// Where the holden-eval tests write their verdict records during a gate
/// run (`HOLDEN_EVAL_RECORD_DIR`), for the replay reopen.
fn records_dir(started: u64) -> PathBuf {
    util::target_base().join(format!(
        "holden-mcp-s4-records-{}-{}",
        util::utc_compact(started),
        std::process::id()
    ))
}

/// The slice-4 P3 reopen: `holden replay` on every verdict record the
/// tests wrote. Green only when at least one record was written and every
/// record replays to its recorded decision.
fn replay_steps(root: &Path, records: &Path, steps: &mut Vec<Step>) {
    let t = Instant::now();
    let mut files: Vec<PathBuf> = fs::read_dir(records)
        .map(|d| {
            d.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let (_, text, dir) = cargo(
        root,
        "s4-replay",
        &[
            "run",
            "--locked",
            "--quiet",
            "-p",
            "holden-cli",
            "--",
            "replay",
        ],
    );
    let mut ok = !files.is_empty();
    println!(
        "gate: s4-replay: {} record(s) in {}",
        files.len(),
        records.display()
    );
    for f in &files {
        let (mut cmd, _, _) = cargo(
            root,
            "s4-replay",
            &[
                "run",
                "--locked",
                "--quiet",
                "-p",
                "holden-cli",
                "--",
                "replay",
            ],
        );
        cmd.arg(f);
        let out = util::capture(&mut cmd);
        let passed = matches!(&out, Ok(o) if o.status.success());
        let said = out
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            })
            .unwrap_or_default();
        println!(
            "gate: s4-replay: {} {}: {said}",
            if passed { "ok" } else { "REFUSED" },
            f.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        );
        ok &= passed;
    }
    steps.push(Step {
        name: "s4-replay",
        command: format!("{text} <record> (for each record in {})", records.display()),
        target_dir: dir.display().to_string(),
        status: if ok { "pass" } else { "fail" },
        duration_ms: t.elapsed().as_millis(),
    });
}

fn cargo(root: &Path, purpose: &str, args: &[&str]) -> (Command, String, PathBuf) {
    cargo_in(root, None, purpose, args)
}

/// `cargo <args>` with its own build directory; `cwd` (when given) is shown
/// in the command text because cargo resolves the workspace from it.
fn cargo_in(
    root: &Path,
    cwd: Option<&Path>,
    purpose: &str,
    args: &[&str],
) -> (Command, String, PathBuf) {
    let dir = util::target_dir(purpose);
    let mut cmd = Command::new("cargo");
    cmd.current_dir(cwd.unwrap_or(root))
        .args(args)
        .env("CARGO_TARGET_DIR", &dir);
    let cd = cwd.map_or_else(String::new, |c| format!("cd {} && ", c.display()));
    let text = format!(
        "{cd}CARGO_TARGET_DIR={} cargo {}",
        dir.display(),
        args.join(" ")
    );
    (cmd, text, dir)
}

fn timed_step(name: &'static str, root: &Path, purpose: &str, args: &[&str]) -> Step {
    timed_step_in(name, root, None, purpose, args)
}

fn timed_step_in(
    name: &'static str,
    root: &Path,
    cwd: Option<&Path>,
    purpose: &str,
    args: &[&str],
) -> Step {
    let (mut cmd, command, dir) = cargo_in(root, cwd, purpose, args);
    println!("gate: {name}: {command}");
    let t = Instant::now();
    let ok = matches!(util::run_inherit(&mut cmd), Ok(Some(0)));
    Step {
        name,
        command,
        target_dir: dir.display().to_string(),
        status: if ok { "pass" } else { "fail" },
        duration_ms: t.elapsed().as_millis(),
    }
}

pub(crate) fn parse_counts(line: &str) -> Option<(u64, u64, u64)> {
    // "test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; ..."
    let rest = line.trim().strip_prefix("test result: ")?;
    let (_, counts) = rest.split_once(". ")?;
    let mut p = None;
    let mut f = None;
    let mut i = None;
    for part in counts.split(';') {
        let mut it = part.split_whitespace();
        let (Some(n), Some(label)) = (it.next(), it.next()) else {
            continue;
        };
        let Ok(n) = n.parse::<u64>() else { continue };
        match label {
            "passed" => p = Some(n),
            "failed" => f = Some(n),
            "ignored" => i = Some(n),
            _ => {}
        }
    }
    Some((p?, f?, i?))
}

/// Run one `cargo test` command and add its test names (prefixed with
/// `pkg::`) and counts to `summary`. Returns whether cargo succeeded.
fn run_tests(cmd: &mut Command, text: &str, pkg: &str, summary: &mut TestSummary) -> bool {
    println!("gate: tests: {text}");
    let out = match util::capture(cmd) {
        Ok(o) => o,
        Err(e) => {
            println!("gate: tests: could not run cargo: {e}");
            return false;
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let ok = out.status.success();
    if !ok {
        print!("{stdout}");
        eprint!("{}", String::from_utf8_lossy(&out.stderr));
    }
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("test ")
            && let Some((name, outcome)) = rest.rsplit_once(" ... ")
        {
            summary.names.push(format!("{pkg}::{name} [{outcome}]"));
        }
        if let Some((p, f, i)) = parse_counts(line) {
            summary.passed += p;
            summary.failed += f;
            summary.ignored += i;
        }
    }
    ok
}

/// The forge module's steps (slice-6): the two-way isolation check, fmt,
/// clippy and tests in its own workspace, `forge verify` of its
/// `SHA256SUMS`, then the export reopen: `forge export` into a fresh
/// directory outside the repository, the export's own
/// `cargo test --offline`, and `forge verify` there.
fn forge_steps(root: &Path, steps: &mut Vec<Step>, summary: &mut TestSummary) {
    let t = Instant::now();
    let findings = forge::isolation_findings(root);
    for f in &findings {
        println!("gate: forge-isolation: {f}");
    }
    steps.push(Step {
        name: "forge-isolation",
        command: "cargo metadata --no-deps --offline (main workspace and forge)".to_owned(),
        target_dir: "-".to_owned(),
        status: if findings.is_empty() { "pass" } else { "fail" },
        duration_ms: t.elapsed().as_millis(),
    });
    let forge = root.join(forge::FORGE_DIR);
    let manifest = forge.join("Cargo.toml");
    let manifest = manifest.to_string_lossy();
    let m = ["--manifest-path", manifest.as_ref()];
    steps.push(timed_step(
        "forge-fmt",
        root,
        "s6-forge-fmt",
        &["fmt", m[0], m[1], "--check"],
    ));
    steps.push(timed_step(
        "forge-clippy",
        root,
        "s6-forge-clippy",
        &[
            "clippy",
            m[0],
            m[1],
            "--locked",
            "--offline",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    ));

    let t = Instant::now();
    let (mut cmd, text, dir) = cargo_in(
        root,
        Some(&forge),
        "s6-forge-test",
        &["test", "--locked", "--offline", "--no-fail-fast"],
    );
    let ok = run_tests(&mut cmd, &text, "holden-forge", summary);
    steps.push(Step {
        name: "forge-tests",
        command: text,
        target_dir: dir.display().to_string(),
        status: if ok { "pass" } else { "fail" },
        duration_ms: t.elapsed().as_millis(),
    });

    let forge_bin = ["run", "--locked", "--offline", "--quiet", "--"];
    let verify: Vec<&str> = forge_bin.iter().copied().chain(["verify"]).collect();
    steps.push(timed_step_in(
        "forge-verify",
        root,
        Some(&forge),
        "s6-forge-run",
        &verify,
    ));

    // The export reopen, into a fresh directory outside the repository.
    let base = std::env::var_os("HOLDEN_TARGET_BASE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let dest = base.join(format!(
        "holden-mcp-s6-export-{}-{}",
        util::utc_compact(util::unix_now()),
        std::process::id()
    ));
    let dest_s = dest.to_string_lossy().into_owned();
    let Some(deny) = util::denylist_path().filter(|d| d.is_file()) else {
        println!(
            "gate: forge-export: the private denylist is absent; the export cannot be checked"
        );
        steps.push(Step {
            name: "forge-export",
            command: "forge export (no denylist)".to_owned(),
            target_dir: "-".to_owned(),
            status: "fail",
            duration_ms: 0,
        });
        return;
    };
    let deny_s = deny.to_string_lossy().into_owned();
    let export: Vec<&str> = forge_bin
        .iter()
        .copied()
        .chain(["export", dest_s.as_str(), "--denylist", deny_s.as_str()])
        .collect();
    let exported = timed_step_in("forge-export", root, Some(&forge), "s6-forge-run", &export);
    let export_ok = exported.status == "pass";
    steps.push(exported);
    if !export_ok {
        return;
    }
    let t = Instant::now();
    let (mut cmd, text, dir) = cargo_in(
        root,
        Some(&dest),
        "s6-export-test",
        &["test", "--locked", "--offline", "--no-fail-fast"],
    );
    let ok = run_tests(&mut cmd, &text, "holden-forge-export", summary);
    steps.push(Step {
        name: "export-tests",
        command: text,
        target_dir: dir.display().to_string(),
        status: if ok { "pass" } else { "fail" },
        duration_ms: t.elapsed().as_millis(),
    });
    steps.push(timed_step_in(
        "export-verify",
        root,
        Some(&dest),
        "s6-export-run",
        &verify,
    ));
    println!("gate: export reopen directory: {dest_s}");
}

/// The slice-3 P7 reopen: `forge export` into a fresh directory outside the
/// repository, then the typed loader test `loads_forge_pack` with
/// `HOLDEN_FORGE_EXPORT` naming that directory, so the export itself is
/// loaded and compared with the committed module.
fn pack_loader_steps(root: &Path, steps: &mut Vec<Step>, summary: &mut TestSummary) {
    let forge = root.join(forge::FORGE_DIR);
    let base = std::env::var_os("HOLDEN_TARGET_BASE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let dest = base.join(format!(
        "holden-mcp-s3-export-{}-{}",
        util::utc_compact(util::unix_now()),
        std::process::id()
    ));
    let dest_s = dest.to_string_lossy().into_owned();
    let Some(deny) = util::denylist_path().filter(|d| d.is_file()) else {
        println!("gate: s3-forge-export: the private denylist is absent; nothing was exported");
        steps.push(Step {
            name: "s3-forge-export",
            command: "forge export (no denylist)".to_owned(),
            target_dir: "-".to_owned(),
            status: "fail",
            duration_ms: 0,
        });
        return;
    };
    let deny_s = deny.to_string_lossy().into_owned();
    let export = [
        "run",
        "--locked",
        "--offline",
        "--quiet",
        "--",
        "export",
        dest_s.as_str(),
        "--denylist",
        deny_s.as_str(),
    ];
    let exported = timed_step_in(
        "s3-forge-export",
        root,
        Some(&forge),
        "s3-forge-run",
        &export,
    );
    let ok = exported.status == "pass";
    steps.push(exported);
    if !ok {
        return;
    }
    let t = Instant::now();
    let args = [
        "test",
        "--locked",
        "--package",
        "holden-cognition",
        "--",
        "--exact",
        "tests::loads_forge_pack",
        "--nocapture",
    ];
    let (mut cmd, text, dir) = cargo(root, "s3-pack-loader", &args);
    cmd.env("HOLDEN_FORGE_EXPORT", &dest);
    let text = format!("HOLDEN_FORGE_EXPORT={dest_s} {text}");
    let mut loaded = TestSummary::default();
    let mut ok = run_tests(&mut cmd, &text, "holden-cognition-export", &mut loaded);
    ok &= loaded.passed == 1 && loaded.failed == 0;
    summary.names.append(&mut loaded.names);
    summary.passed += loaded.passed;
    summary.failed += loaded.failed;
    summary.ignored += loaded.ignored;
    steps.push(Step {
        name: "s3-pack-loader",
        command: text,
        target_dir: dir.display().to_string(),
        status: if ok { "pass" } else { "fail" },
        duration_ms: t.elapsed().as_millis(),
    });
    println!("gate: s3 export reopen directory: {dest_s}");
}

fn test_step(root: &Path, records: &Path, ledgers: &Path, summary: &mut TestSummary) -> Step {
    let dir = util::target_dir("gate-test");
    let mut command = String::new();
    let t = Instant::now();
    let mut ok = true;
    for pkg in PACKAGES {
        let args = ["test", "--locked", "--package", pkg, "--no-fail-fast"];
        let (mut cmd, text, _) = cargo(root, "gate-test", &args);
        cmd.env("HOLDEN_EVAL_RECORD_DIR", records);
        cmd.env("HOLDEN_LEDGER_RECORD_DIR", ledgers);
        if !command.is_empty() {
            command.push_str(" && ");
        }
        command.push_str(&text);
        ok &= run_tests(&mut cmd, &text, pkg, summary);
    }
    summary.names.sort();
    if summary.failed > 0 || summary.passed == 0 {
        ok = false;
    }
    Step {
        name: "tests",
        command,
        target_dir: dir.display().to_string(),
        status: if ok { "pass" } else { "fail" },
        duration_ms: t.elapsed().as_millis(),
    }
}

/// Options of a local gate run.
#[derive(Debug, Default)]
pub struct GateOptions {
    /// Accept the host-covered tool limitations (set by the VM driver).
    pub host_covered_tools: bool,
    /// Also write the run record to this path.
    pub record_copy: Option<PathBuf>,
}

/// Run the gate for `slice` on this host. Returns true when green.
pub fn run(root: &Path, slice: &'static str, opts: &GateOptions) -> bool {
    let started = util::unix_now();
    let commit = util::stdout_of(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["rev-parse", "HEAD"]),
    )
    .unwrap_or_else(|| "unknown".to_owned());
    let tree_clean = util::stdout_of(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["status", "--porcelain"]),
    )
    .is_some_and(|s| s.is_empty());
    let rustc = util::stdout_of(Command::new("rustc").current_dir(root).arg("-V"))
        .unwrap_or_else(|| "unknown".to_owned());
    println!(
        "gate: {slice} on {}/{} at {commit} (tree clean: {tree_clean}; {rustc}; target base {})",
        std::env::consts::OS,
        std::env::consts::ARCH,
        util::target_base().display()
    );

    let mut steps = vec![
        timed_step("fmt", root, "gate-fmt", &["fmt", "--all", "--check"]),
        timed_step(
            "clippy",
            root,
            "gate-clippy",
            &[
                "clippy",
                "--locked",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
    ];
    let mut tests = TestSummary::default();
    let records = records_dir(started);
    let ledgers = ledgers_dir(started);
    steps.push(test_step(root, &records, &ledgers, &mut tests));
    if REPLAY_SLICES.contains(&slice) {
        replay_steps(root, &records, &mut steps);
    }
    if LEDGER_SLICES.contains(&slice) {
        ledger_verify_steps(root, &ledgers, &mut steps);
        println!("gate: selftest (cargo xtask selftest, in process)");
        let t = Instant::now();
        let green = crate::selftest::run(root, None);
        steps.push(Step {
            name: "selftest",
            command: "cargo xtask selftest".to_owned(),
            target_dir: util::target_dir("selftest").display().to_string(),
            status: if green { "pass" } else { "fail" },
            duration_ms: t.elapsed().as_millis(),
        });
    }
    if FORGE_SLICES.contains(&slice) {
        forge_steps(root, &mut steps, &mut tests);
        tests.names.sort();
    }
    if PACK_LOADER_SLICES.contains(&slice) {
        pack_loader_steps(root, &mut steps, &mut tests);
        tests.names.sort();
    }

    println!("gate: guard (in process; tools use their own state)");
    let t = Instant::now();
    let report = guard::run(root);
    report.print();
    let verdict = report.verdict(opts.host_covered_tools);
    let guard_ok = matches!(
        verdict,
        Verdict::Pass | Verdict::PassWithDeclaredLimitations
    );
    steps.push(Step {
        name: "guard",
        command: "cargo xtask guard".to_owned(),
        target_dir: "-".to_owned(),
        status: if guard_ok { "pass" } else { "fail" },
        duration_ms: t.elapsed().as_millis(),
    });

    let all_pass = steps.iter().all(|s| s.status == "pass");
    let result = match (all_pass, verdict) {
        (true, Verdict::Pass) => "green",
        (true, Verdict::PassWithDeclaredLimitations) => "green_with_declared_limitations",
        _ => "red",
    };

    println!("gate: tests ({} names):", tests.names.len());
    for n in &tests.names {
        println!("gate:   {n}");
    }
    println!(
        "gate: test counts: {} passed, {} failed, {} ignored",
        tests.passed, tests.failed, tests.ignored
    );
    for s in &steps {
        println!(
            "gate: step {:<7} {} ({} ms)",
            s.name, s.status, s.duration_ms
        );
    }
    if verdict == Verdict::PassWithDeclaredLimitations {
        println!(
            "gate: declared limitations (covered by the host guard): {}",
            guard::HOST_COVERED.join(", ")
        );
    }

    let record = RunRecord {
        schema: "holden.gate-run/1",
        slice,
        target_base: util::target_base().display().to_string(),
        started_utc: util::utc_iso(started),
        host_os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        commit,
        tree_clean,
        rustc,
        steps,
        tests,
        guard: report,
        guard_verdict: verdict,
        result,
    };
    write_record(&record, started, opts);
    println!("gate: result: {result}");
    result != "red"
}

fn write_record(record: &RunRecord, started: u64, opts: &GateOptions) {
    let bytes = match serde_json::to_vec_pretty(record) {
        Ok(mut b) => {
            b.push(b'\n');
            b
        }
        Err(e) => {
            println!("gate: run record could not be serialized: {e}");
            return;
        }
    };
    match util::private_root() {
        Some(root) => {
            let dir = root.join("agent-state").join("runs");
            let path = dir.join(format!(
                "{}-{}.json",
                util::utc_compact(started),
                std::env::consts::OS
            ));
            match fs::create_dir_all(&dir).and_then(|()| fs::write(&path, &bytes)) {
                Ok(()) => println!("gate: run_record: {}", path.display()),
                Err(e) => println!("gate: run record not written: {e}"),
            }
        }
        None => println!("gate: run record not written: no private root resolvable"),
    }
    if let Some(copy) = &opts.record_copy {
        match fs::write(copy, &bytes) {
            Ok(()) => println!("gate: run_record_copy: {}", copy.display()),
            Err(e) => println!("gate: run record copy not written: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_libtest_result_line() {
        let l = "test result: ok. 12 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.25s";
        assert_eq!(parse_counts(l), Some((12, 1, 2)));
        assert_eq!(parse_counts("running 3 tests"), None);
    }
}
