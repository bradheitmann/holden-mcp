//! holden-mcp developer tasks.
//!
//! ```text
//! cargo xtask guard [--host-covered-tools]
//! cargo xtask gate <slice> [--host-covered-tools] [--record <path>]
//! cargo xtask gate <slice> --linux-vm
//! cargo xtask selftest [--ledger <file>]
//! ```
//!
//! `<slice>` is one of `gate::SLICES`. `--linux-vm` honours `HOLDEN_VM_ADDR`
//! and `HOLDEN_TARGET_BASE` (see `vm`). `selftest` (slice-5 P5) reintroduces
//! known defects as mutants of holden's own code and records the catch
//! counts in the signed ledger (see `selftest`).
//!
//! Exit codes: 0 pass, 1 refused or red, 2 usage, 3 limitation (never a pass).

mod e2e;
mod forge;
mod gate;
mod guard;
mod selftest;
mod util;
mod vm;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage:
  cargo xtask guard [--host-covered-tools]
  cargo xtask gate <slice> [--host-covered-tools] [--record <path>]
  cargo xtask gate <slice> --linux-vm
  cargo xtask selftest [--ledger <file>]
  cargo xtask e2e slice-11
<slice>: slice-0 | slice-1 | slice-1b | slice-1c | slice-2 | slice-3 | slice-4 | slice-5 | slice-6 | slice-7";

fn usage(why: &str) -> ExitCode {
    eprintln!("{why}\n{USAGE}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = util::repo_root();
    match args.first().map(String::as_str) {
        Some("guard") => {
            let mut host_covered = false;
            for a in &args[1..] {
                match a.as_str() {
                    "--host-covered-tools" => host_covered = true,
                    other => return usage(&format!("unknown argument '{other}'")),
                }
            }
            let report = guard::run(&root);
            report.print();
            let verdict = report.verdict(host_covered);
            println!("guard: verdict: {verdict:?}");
            match verdict {
                guard::Verdict::Pass | guard::Verdict::PassWithDeclaredLimitations => {
                    ExitCode::SUCCESS
                }
                guard::Verdict::Refused => ExitCode::from(1),
                guard::Verdict::Limitation => ExitCode::from(3),
            }
        }
        Some("gate") => {
            let Some(slice) = args
                .get(1)
                .and_then(|s| gate::SLICES.iter().copied().find(|k| k == s))
            else {
                return usage("no gate for that slice");
            };
            let mut opts = gate::GateOptions::default();
            let mut linux_vm = false;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--linux-vm" => linux_vm = true,
                    "--host-covered-tools" => opts.host_covered_tools = true,
                    "--record" => {
                        let Some(p) = args.get(i + 1) else {
                            return usage("--record needs a path");
                        };
                        opts.record_copy = Some(PathBuf::from(p));
                        i += 1;
                    }
                    other => return usage(&format!("unknown argument '{other}'")),
                }
                i += 1;
            }
            if linux_vm && (opts.host_covered_tools || opts.record_copy.is_some()) {
                return usage("--linux-vm takes no other flag");
            }
            let green = if linux_vm {
                vm::run(&root, slice)
            } else {
                gate::run(&root, slice, &opts)
            };
            if green {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Some("e2e") => match args.as_slice() {
            [_, slice] if slice == "slice-11" => {
                if e2e::run(&root, slice) {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            _ => usage("e2e takes exactly slice-11"),
        },
        Some("selftest") => {
            let ledger = match &args[1..] {
                [] => None,
                [flag, path] if flag == "--ledger" => Some(PathBuf::from(path)),
                _ => return usage("selftest takes only --ledger <file>"),
            };
            if selftest::run(&root, ledger) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        _ => usage("unknown command"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::vm::{AddrSource, gate_script, resolve_addr};
    use crate::{forge, guard, util};

    /// A `tart` program path that does not exist, so any call to it fails.
    fn absent_tart() -> PathBuf {
        let p = std::env::temp_dir()
            .join(format!("holden-mcp-no-tart-{}", std::process::id()))
            .join("tart");
        assert!(!p.exists());
        p
    }

    #[test]
    fn vm_addr_override_skips_tart() {
        let tart = absent_tart();
        // With HOLDEN_VM_ADDR, the address is used as is: tart is never run
        // (running it would fail, since it does not exist).
        for addr in ["192.0.2.10", "fd00::5", "vm.internal"] {
            assert_eq!(
                resolve_addr(Some(addr), tart.as_os_str(), "holden-mcp-linux"),
                Ok((addr.to_owned(), AddrSource::Override))
            );
        }
        // Without it, tart is asked, and an absent tart is a refusal.
        assert!(resolve_addr(None, tart.as_os_str(), "holden-mcp-linux").is_err());
        // A malformed override is refused rather than passed to ssh.
        for bad in ["", "-oProxyCommand=x", "a b", "host;rm"] {
            assert!(
                resolve_addr(Some(bad), tart.as_os_str(), "holden-mcp-linux").is_err(),
                "{bad:?}"
            );
        }
    }

    /// Slice-12 P1b: the exact remote gate command carries the host's private
    /// root base into the VM before any private-root-dependent gate step.
    /// Fail-without mutant: drop `private_root_export` from `gate_script`.
    #[test]
    fn vm_gate_forwards_private_root() {
        let script = gate_script(
            "holden-mcp-gate",
            "0123456789abcdef",
            "slice-12",
            "/tmp/holden-target",
            Some("/tmp/holden-agent-private"),
            "holden-mcp-gate/run.json",
        );
        assert!(script.contains(
            "export HOLDEN_TARGET_BASE=/tmp/holden-target; export AGENT_PRIVATE_ROOT=/tmp/holden-agent-private; mkdir -p"
        ));
        assert!(script.contains("cargo xtask gate slice-12 --host-covered-tools"));
        let default = gate_script(
            "holden-mcp-gate",
            "0123456789abcdef",
            "slice-12",
            "/tmp/holden-target",
            None,
            "holden-mcp-gate/run.json",
        );
        assert!(!default.contains("export AGENT_PRIVATE_ROOT="));
    }

    /// slice-6 P1: the forge module and the main workspace are isolated in
    /// both directions, read from `cargo metadata`: no main-workspace crate
    /// depends on anything under `modules/holden-forge/`, and the forge is
    /// its own workspace with no path dependency outside its directory.
    /// Fail-without mutant: add a path dependency from a main-workspace crate
    /// to the forge crate; this test fails naming that dependency.
    #[test]
    fn forge_is_isolated() {
        let findings = forge::isolation_findings(&util::repo_root());
        assert_eq!(findings, Vec::<String>::new());
    }

    /// `LICENSE` is byte-identical to the official PolyForm Noncommercial
    /// 1.0.0 text at upstream commit 76a278c4 (slice-1b P1).
    #[test]
    fn license_text_hash_is_pinned() {
        assert_eq!(
            guard::LICENSE_SHA256,
            "c0ea4a896d2c8c394b29f9427589996db826cd501c512279ff0ed3ef48fabbe5"
        );
        let root = util::repo_root();
        let bytes = fs::read(root.join("LICENSE")).unwrap();
        assert_eq!(guard::sha256_hex(&bytes), guard::LICENSE_SHA256);
        assert_eq!(guard::check_license_text(&root), guard::Status::Pass);
    }

    /// A scratch repository root under the system temp directory.
    fn scratch(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("holden-mcp-xtask-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    /// Write `body` at `rel` under `root`, creating parents; returns `rel`.
    fn put(root: &std::path::Path, rel: &str, body: &str) -> String {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
        rel.to_owned()
    }

    /// The former license's SPDX id, assembled so this source does not hold it.
    fn former() -> String {
        ["M", "IT"].concat()
    }

    fn refused_files(status: &guard::Status) -> Vec<String> {
        match status {
            guard::Status::Refused { findings } => findings
                .iter()
                .map(|f| f.split(':').next().unwrap_or_default().to_owned())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// slice-1c P1: a `qa/<slice>/` file is exempt only when the slice is
    /// recorded accepted and `charter/qa-pins/<slice>.sha256` pins its current
    /// bytes; the guard names the reason. Also holds for the real pinned
    /// records of slice-0 and slice-1b.
    #[test]
    fn qa_record_exempt_only_when_pinned() {
        let root = scratch("qa-pinned");
        let w = former();
        put(
            &root,
            "charter/.dispatch/slice-a.json",
            "{\"slice\": \"slice-a\", \"state\": \"accepted\"}",
        );
        put(
            &root,
            "charter/.dispatch/slice-b.json",
            "{\"slice\": \"slice-b\"}",
        );
        let body = format!("the verifier set the crate to {w}\n");
        let a = put(&root, "qa/slice-a/report.md", &body);
        let b = put(&root, "qa/slice-b/report.md", &body);
        let hash = guard::sha256_hex(body.as_bytes());
        put(
            &root,
            "charter/qa-pins/slice-a.sha256",
            &format!("{hash}  {a}\n"),
        );
        put(
            &root,
            "charter/qa-pins/slice-b.sha256",
            &format!("{hash}  {b}\n"),
        );
        let files = vec![a.clone(), b.clone()];

        // Accepted and pinned: exempt, with the reason named.
        let scan = guard::scan_stale_license_mentions(&root, &files);
        assert_eq!(refused_files(&scan.status), vec![b.clone()]);
        assert_eq!(
            scan.exempt,
            vec![format!(
                "{a}: 1 line(s) exempt: accepted slice-a, sha256 pinned in charter/qa-pins/slice-a.sha256"
            )]
        );

        // Pinned but its bytes changed: checked again.
        put(&root, &a, &format!("{body}edited\n"));
        let scan = guard::scan_stale_license_mentions(&root, &files);
        assert_eq!(refused_files(&scan.status), vec![a.clone(), b]);
        assert!(scan.exempt.is_empty());
        fs::remove_dir_all(&root).unwrap();

        // The real records: every qa/ file that names the former license is
        // exempt by pin, and none is refused.
        let real = util::repo_root();
        let qa: Vec<String> = guard::tracked_files(&real)
            .unwrap()
            .into_iter()
            .filter(|f| f.starts_with("qa/"))
            .collect();
        let scan = guard::scan_stale_license_mentions(&real, &qa);
        assert_eq!(scan.status, guard::Status::Pass);
        for f in [
            "qa/slice-1b/evidence.md",
            "qa/slice-1b/receipt.json",
            "qa/slice-1b/report.md",
        ] {
            assert!(
                scan.exempt.iter().any(|e| e.starts_with(&format!("{f}: "))
                    && e.ends_with("charter/qa-pins/slice-1b.sha256")),
                "{f} not exempt by pin: {:?}",
                scan.exempt
            );
        }
    }

    /// slice-1c P1: any other file under `qa/` is still checked: a slice with
    /// no acceptance or pin record, an unpinned file in an accepted slice, and
    /// a file directly under `qa/`.
    #[test]
    fn unpinned_qa_file_is_checked() {
        let root = scratch("qa-unpinned");
        let w = former();
        put(
            &root,
            "charter/.dispatch/slice-a.json",
            "{\"slice\": \"slice-a\", \"state\": \"accepted\"}",
        );
        let pinned_body = format!("history: {w}\n");
        let pinned = put(&root, "qa/slice-a/report.md", &pinned_body);
        put(
            &root,
            "charter/qa-pins/slice-a.sha256",
            &format!("{}  {pinned}\n", guard::sha256_hex(pinned_body.as_bytes())),
        );
        let claim = format!("This repository is {w} licensed\n");
        let probe = put(&root, "qa/slice-x/notes.md", &claim);
        let unpinned = put(&root, "qa/slice-a/notes.md", &claim);
        let top = put(&root, "qa/notes.md", &claim);
        let files = vec![pinned, probe.clone(), unpinned.clone(), top.clone()];
        let status = guard::check_stale_license_mentions(&root, &files);
        assert_eq!(refused_files(&status), vec![probe, unpinned, top]);
        fs::remove_dir_all(&root).unwrap();
    }

    /// slice-1c P2 (closes D1): `charter/owner-decisions/` is not exempt as a
    /// directory; only files listed in `charter/license-mention-allow.txt`
    /// are. A new decision stating the repository is under the former license
    /// is refused.
    #[test]
    fn new_decision_claiming_mit_is_refused() {
        let root = scratch("decision-claim");
        let w = former();
        let listed = put(
            &root,
            "charter/owner-decisions/2026-01-01-old.md",
            &format!("Owner, verbatim: \"I don't want this to be {w}.\"\n"),
        );
        put(
            &root,
            guard::LICENSE_MENTION_ALLOW,
            &format!("{listed} — quotes the owner's refusal of the former license\n"),
        );
        let new = put(
            &root,
            "charter/owner-decisions/2026-12-01-new.md",
            &format!("This repository is {w}-licensed.\n"),
        );
        let scan = guard::scan_stale_license_mentions(&root, &[listed.clone(), new.clone()]);
        assert_eq!(
            scan.status,
            guard::Status::Refused {
                findings: vec![format!("{new}:1: names the former license")]
            }
        );
        assert_eq!(scan.exempt.len(), 1);
        assert!(scan.exempt[0].starts_with(&format!("{listed}: 1 line(s) exempt: listed in")));
        fs::remove_dir_all(&root).unwrap();
    }

    /// slice-1c P3: the licensor-name exception record lists every tracked
    /// file that carries the licensor name (found by the name hash), every
    /// listed file is tracked or marked forthcoming, and each is in
    /// `charter/safety-allow.txt`.
    #[test]
    fn licensor_exception_lists_every_file() {
        use crate::guard::licensor;
        let root = util::repo_root();
        let tracked = guard::tracked_files(&root).unwrap();
        let carriers = licensor::licensor_name_carriers(&root, &tracked);
        // The hash search is not vacuous: NOTICE must carry the name.
        assert!(carriers.iter().any(|c| c == "NOTICE"), "{carriers:?}");
        let record = fs::read_to_string(root.join(licensor::LICENSE_DECISION_RECORD)).unwrap();
        let entries = licensor::licensor_exception_entries(&record);
        for c in &carriers {
            assert!(
                entries.iter().any(|e| &e.path == c),
                "{c} carries the licensor name but is not in the exception record"
            );
        }
        let findings = licensor::licensor_exception_findings(&root, &tracked);
        assert!(findings.is_empty(), "{findings:#?}");

        // An absent listed file is tolerated only when marked forthcoming.
        let s = scratch("licensor-forthcoming");
        put(
            &s,
            licensor::LICENSE_DECISION_RECORD,
            "## Licensor-name exception\n  - `later/NOTICE` (forthcoming with a later slice);\n  - `gone/NOTICE` (the notice);\n## Next\n",
        );
        put(
            &s,
            "charter/safety-allow.txt",
            "later/NOTICE — reason\ngone/NOTICE — reason\n",
        );
        assert_eq!(
            licensor::licensor_exception_findings(&s, &[licensor::LICENSE_DECISION_RECORD.to_owned()]),
            vec![
                "gone/NOTICE: listed in the exception record but not tracked, and not marked forthcoming"
                    .to_owned()
            ]
        );
        fs::remove_dir_all(&s).unwrap();
    }
}
