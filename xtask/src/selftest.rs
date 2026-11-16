//! `cargo xtask selftest` (slice-5 P5): holden runs its own controls against
//! its own code.
//!
//! Each [`Defect`] reintroduces a known defect (the S-RUNNER never-again
//! list, docs/ARCHITECTURE.md §3.5) as a mutant of holden's own source: one
//! exact text replacement in one file of `holden-seal`, `holden-mcp` or
//! `holden-eval`. In a scratch copy of the working tree, for each defect:
//!
//! 1. baseline: the never-again test that guards it runs on the unmutated
//!    code and must run exactly one test and pass (a known-good control);
//! 2. mutant: the replacement is applied (it must match exactly once), the
//!    same test runs and must fail (the defect is caught); the file is then
//!    restored byte for byte.
//!
//! One `selftest` record per defect is appended to the signed ledger (the
//! private root's `db/ledger.sqlite`, or `--ledger <file>`), signed with the
//! `ledger` key of the private root's key store (`keys/`, created on first
//! use). The run is green only when every defect is caught by a baseline
//! that ran and passed: a disabled or missing test leaves its defect's
//! catch count at 0 and fails the gate.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use holden_ledger::Ledger;
use holden_ledger::records::{SelftestRecord, kind};
use holden_seal::{EvaluatorKey, FileKeyStore, KeyMaterial, KeyName, KeyStore, KeyStoreError};

use crate::util;

/// One reintroduced defect.
#[derive(Debug)]
pub struct Defect {
    /// Its id.
    pub id: &'static str,
    /// The never-again rule it breaks.
    pub rule: &'static str,
    /// The crate the mutant changes.
    pub crate_name: &'static str,
    /// The file, relative to the repository root.
    pub file: &'static str,
    /// The exact text replaced (must occur once).
    pub find: &'static str,
    /// Its replacement.
    pub replace: &'static str,
    /// `cargo test` selection: package and target arguments.
    pub target: &'static [&'static str],
    /// The exact test name.
    pub test: &'static str,
}

/// The defects, in run order.
pub const DEFECTS: [Defect; 5] = [
    Defect {
        id: "env-reaches-elevated-process",
        rule: "the implementer's environment is never applied to a process holden starts",
        crate_name: "holden-eval",
        file: "crates/holden-eval/src/spawn.rs",
        find: "        .env_clear()\n        .stdin(Stdio::piped())",
        replace: "        .stdin(Stdio::piped())",
        target: &["-p", "holden-eval", "--test", "eval"],
        test: "tests::product_env_is_scrubbed",
    },
    Defect {
        id: "canary-scan-caller-path",
        rule: "the evaluated commit's tree is scanned for canaries, never a caller-chosen path",
        crate_name: "holden-eval",
        file: "crates/holden-eval/src/canary.rs",
        find: ".ls_tree(commit)?",
        replace: ".ls_tree(\"HEAD\")?",
        target: &["-p", "holden-eval", "--test", "eval"],
        test: "tests::canary_scan_uses_evaluated_tree",
    },
    Defect {
        id: "config-diff-always-empty",
        rule: "config changes are always diffed and reported",
        crate_name: "holden-eval",
        file: "crates/holden-eval/src/config.rs",
        find: "if before == after {",
        replace: "if before == after || before != after {",
        target: &["-p", "holden-eval", "--test", "eval"],
        test: "tests::config_change_is_reported",
    },
    Defect {
        id: "role-check-skipped",
        rule: "only the evaluator role reads moment-of-test parameters",
        crate_name: "holden-mcp",
        file: "crates/holden-mcp/src/ledger.rs",
        find: "Role::Implementer => Err(Refusal::ForbiddenRole),",
        replace: "Role::Implementer => Ok(()),",
        target: &["-p", "holden-mcp", "--test", "mcp"],
        test: "tests::implementer_refused_reveal_params",
    },
    Defect {
        id: "verdict-signature-unchecked",
        rule: "verdicts are signed, and a signature that does not verify is refused",
        crate_name: "holden-seal",
        file: "crates/holden-seal/src/evaluator.rs",
        find: "key.verify_strict(message, &sig)\n            .map_err(|_| SignatureRefused::BadSignature)",
        replace: "key.verify_strict(message, &sig)\n            .or(Ok(()))",
        target: &["-p", "holden-seal", "--lib"],
        test: "evaluator::tests::evaluator_signature_verifies_only_for_its_key_and_message",
    },
];

/// One defect's result.
#[derive(Debug)]
pub struct Outcome {
    /// The record appended to the ledger.
    pub record: SelftestRecord,
    /// Why it is not green, if it is not.
    pub problem: Option<String>,
}

/// Counts from a `cargo test` run: (passed, failed, ran ok).
fn run_test(src: &Path, target_dir: &Path, d: &Defect) -> (u64, u64, bool) {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(src)
        .env("CARGO_TARGET_DIR", target_dir)
        .args(["test", "--locked"])
        .args(d.target)
        .args(["--", "--exact", d.test]);
    let Ok(out) = util::capture(&mut cmd) else {
        return (0, 0, false);
    };
    let mut passed = 0;
    let mut failed = 0;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some((p, f, _)) = crate::gate::parse_counts(line) {
            passed += p;
            failed += f;
        }
    }
    (passed, failed, out.status.success())
}

/// Copy the working tree's files (tracked, and untracked but not ignored)
/// into `dest`. Each file is written anew, so its modification time is the
/// copy's: `fs::copy` keeps the source's time on macOS, and cargo, which
/// hashes path packages relative to the workspace root, would then reuse a
/// previous run's (possibly mutated) build from the same target directory
/// as this run's baseline.
fn copy_tree(root: &Path, dest: &Path) -> Result<usize, String> {
    let out = util::capture(Command::new("git").arg("-C").arg(root).args([
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
    ]))
    .map_err(|e| format!("git ls-files: {e}"))?;
    if !out.status.success() {
        return Err("git ls-files failed".to_owned());
    }
    let mut n = 0;
    for rel in out.stdout.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let rel = String::from_utf8_lossy(rel).into_owned();
        let from = root.join(&rel);
        if !from.is_file() {
            continue; // deleted in the working tree
        }
        let to = dest.join(&rel);
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let bytes = fs::read(&from).map_err(|e| format!("{rel}: {e}"))?;
        fs::write(&to, bytes).map_err(|e| format!("{rel}: {e}"))?;
        n += 1;
    }
    Ok(n)
}

/// Load the ledger signing key from the private root's key store, creating
/// it on first use.
fn ledger_key() -> Result<EvaluatorKey, String> {
    let root = util::private_root().ok_or("no private root resolvable")?;
    let store = FileKeyStore::open(root.join("keys")).map_err(|e| e.to_string())?;
    let name = KeyName::new("ledger").map_err(|e| e.to_string())?;
    match store.get(&name) {
        Ok(m) => Ok(EvaluatorKey::from_material(&m)),
        Err(KeyStoreError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }) => {
            let m = KeyMaterial::generate().map_err(|e| e.to_string())?;
            store.put(&name, &m).map_err(|e| e.to_string())?;
            println!("selftest: created the ledger signing key in the private key store");
            Ok(EvaluatorKey::from_material(&m))
        }
        Err(e) => Err(e.to_string()),
    }
}

/// The default ledger: the private root's `db/ledger.sqlite`.
fn default_ledger() -> Option<PathBuf> {
    util::private_root().map(|r| r.join("db").join(holden_ledger::store::LEDGER_FILE))
}

/// Run every defect; returns whether the self-test is green.
pub fn run(root: &Path, ledger_path: Option<PathBuf>) -> bool {
    let started = util::unix_now();
    let Some(ledger_path) = ledger_path.or_else(default_ledger) else {
        println!("selftest: refused: no ledger path (no private root resolvable)");
        return false;
    };
    let key = match ledger_key() {
        Ok(k) => k,
        Err(e) => {
            println!("selftest: refused: ledger key: {e}");
            return false;
        }
    };
    let mut ledger = match Ledger::open(&ledger_path) {
        Ok(l) => l,
        Err(e) => {
            println!("selftest: refused: {e}");
            return false;
        }
    };
    let commit = util::stdout_of(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["rev-parse", "HEAD"]),
    )
    .unwrap_or_else(|| "unknown".to_owned());
    let clean = util::stdout_of(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["status", "--porcelain"]),
    )
    .is_some_and(|s| s.is_empty());
    let commit = if clean {
        commit
    } else {
        format!("{commit}+dirty")
    };
    let src = util::target_base().join(format!(
        "holden-mcp-selftest-{}-{}",
        util::utc_compact(started),
        std::process::id()
    ));
    let target_dir = util::target_dir("selftest");
    match copy_tree(root, &src) {
        Ok(n) => println!(
            "selftest: {n} file(s) of {commit} copied to {}; CARGO_TARGET_DIR={}",
            src.display(),
            target_dir.display()
        ),
        Err(e) => {
            println!("selftest: refused: copying the tree: {e}");
            return false;
        }
    }
    let mut outcomes = Vec::new();
    for d in &DEFECTS {
        let t = Instant::now();
        let (bp, bf, bok) = run_test(&src, &target_dir, d);
        let baseline_passed = bok && bp == 1 && bf == 0;
        let file = src.join(d.file);
        let original = fs::read(&file).unwrap_or_default();
        let text = String::from_utf8_lossy(&original).into_owned();
        let matches = text.matches(d.find).count();
        let (mp, mf, mok, applied) = if matches == 1 {
            let mutated = text.replacen(d.find, d.replace, 1);
            if fs::write(&file, mutated).is_err() {
                (0, 0, true, false)
            } else {
                let r = run_test(&src, &target_dir, d);
                let restored = fs::write(&file, &original).is_ok();
                (r.0, r.1, r.2, restored)
            }
        } else {
            (0, 0, true, false)
        };
        let caught = applied && baseline_passed && !mok && mf == 1 && mp == 0;
        let problem = if !baseline_passed {
            Some(format!(
                "the baseline did not run and pass exactly one test ({bp} passed, {bf} failed)"
            ))
        } else if matches != 1 {
            Some(format!("the mutant matched {matches} times, not once"))
        } else if !applied {
            Some("the mutant could not be applied or the file restored".to_owned())
        } else if !caught {
            Some(format!(
                "not caught: the mutant run had {mp} passed, {mf} failed"
            ))
        } else {
            None
        };
        let record = SelftestRecord {
            format: "holden.selftest/1".to_owned(),
            defect: d.id.to_owned(),
            crate_name: d.crate_name.to_owned(),
            file: d.file.to_owned(),
            test: d.test.to_owned(),
            commit: commit.clone(),
            baseline_passed,
            baseline_ran: bp + bf,
            mutant_ran: mp + mf,
            caught,
        };
        println!(
            "selftest: {:<30} {:<12} baseline {} ({} ran), mutant {} ({} ms) — {}",
            d.id,
            d.crate_name,
            if baseline_passed { "pass" } else { "NOT PASS" },
            bp + bf,
            if caught { "CAUGHT" } else { "MISSED" },
            t.elapsed().as_millis(),
            d.rule
        );
        if let Some(p) = &problem {
            println!("selftest:   {p}");
        }
        outcomes.push(Outcome { record, problem });
    }
    let mut ok = true;
    for o in &outcomes {
        if let Err(e) = ledger.append_json(&key, kind::SELFTEST, &o.record) {
            println!("selftest: refused: {e}");
            ok = false;
        }
    }
    let entries = ledger.entries().unwrap_or_default();
    let measured = holden_ledger::Measurements::from_entries(&entries).unwrap_or_default();
    println!(
        "selftest: catch counts in the ledger ({}):",
        ledger_path.display()
    );
    for d in &DEFECTS {
        let s = measured.selftest.get(d.id).copied().unwrap_or_default();
        let this_run = outcomes
            .iter()
            .find(|o| o.record.defect == d.id)
            .is_some_and(|o| o.record.caught);
        println!(
            "selftest:   {:<30} this run {} | all runs caught {} of {} (95% lower bound {:.3}), baseline false fails {} of {}",
            d.id,
            u8::from(this_run),
            s.catch.k,
            s.catch.n,
            s.catch.lower(),
            s.false_fail.k,
            s.false_fail.n
        );
    }
    if let Some(tip) = entries.last() {
        println!("selftest: ledger tip {}:{}", tip.seq, tip.hash);
    }
    ok &= outcomes.iter().all(|o| o.problem.is_none());
    let _ = fs::remove_dir_all(&src);
    println!("selftest: result: {}", if ok { "green" } else { "red" });
    ok
}
