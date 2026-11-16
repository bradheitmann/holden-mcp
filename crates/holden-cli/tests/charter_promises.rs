//! slice-18b: charter candidates are numbered proofs and guarantees, and
//! both success and refusal reporting stay bounded. Tests run the real CLI
//! against real git fixture hosts; fixture roots are Drop-guard cleaned.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::fs::{self, DirBuilder};
    use std::os::unix::fs::DirBuilderExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};
    use std::sync::OnceLock;

    use holden_eval::git::Git;

    fn workspace() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    fn holden() -> &'static Path {
        static BIN: OnceLock<PathBuf> = OnceLock::new();
        BIN.get_or_init(|| {
            let exe = std::env::current_exe().unwrap();
            let profile = exe.parent().and_then(Path::parent).unwrap();
            let target = profile.parent().unwrap();
            let status = Command::new(env!("CARGO"))
                .args([
                    "build",
                    "--locked",
                    "--quiet",
                    "-p",
                    "holden-cli",
                    "--bin",
                    "holden",
                    "--manifest-path",
                ])
                .arg(workspace().join("Cargo.toml"))
                .env("CARGO_TARGET_DIR", target)
                .status()
                .unwrap();
            assert!(status.success(), "could not build the real holden CLI");
            profile.join("holden")
        })
    }

    fn text(out: &Output) -> String {
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    }

    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let destination = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &destination);
            } else {
                fs::copy(entry.path(), destination).unwrap();
            }
        }
    }

    struct Fixture {
        root: PathBuf,
        host: PathBuf,
        state: PathBuf,
        private: PathBuf,
        owner_store: PathBuf,
        denylist: PathBuf,
    }

    impl Fixture {
        fn new(fixture: &str, tag: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("holden-cli-s18b-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)
                .unwrap();
            let root = root.canonicalize().unwrap();
            let host = root.join("host");
            copy_tree(&workspace().join("fixtures/clients").join(fixture), &host);
            let home = root.join("git-home");
            fs::create_dir_all(&home).unwrap();
            let git = Git::new(&host, &home);
            git.init().unwrap();
            git.commit_all("real slice-18b fixture").unwrap();
            let denylist = root.join("denylist.txt");
            fs::write(&denylist, "zz-synthetic-private-token\n").unwrap();
            Self {
                state: root.join("state"),
                private: root.join("agent-private"),
                owner_store: root.join("owner"),
                denylist,
                root,
                host,
            }
        }

        fn embed(&self) -> Output {
            Command::new(holden())
                .args([
                    "embed",
                    self.host.to_str().unwrap(),
                    "--state",
                    self.state.to_str().unwrap(),
                    "--forge",
                    self.root.join("absent-forge").to_str().unwrap(),
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
                    self.root.join("absent-confine").to_str().unwrap(),
                    "--owner-store",
                    self.owner_store.to_str().unwrap(),
                ])
                .current_dir(&self.host)
                .env("AGENT_PRIVATE_ROOT", &self.private)
                .output()
                .unwrap()
        }

        fn detection_lines(out: &Output) -> Vec<String> {
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .filter(|line| line.starts_with("detection:"))
                .map(str::to_owned)
                .collect()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// P1. Fail-without mutant: restore paragraph extraction; the exact
    /// candidate count and source lines go red.
    #[test]
    fn charter_yields_numbered_proofs_and_guarantees() {
        let fx = Fixture::new("numbered-charter", "p1");
        let out = fx.embed();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        let lines = Fixture::detection_lines(&out);
        assert!(lines.contains(&"detection: planning process charter".to_owned()));
        assert!(lines.contains(&"detection: candidate promises 8".to_owned()));
        for expected in [
            "detection: candidate promise charter/DONE/slice-a.md:5: P1 reopen proof: a reviewer reopens the recorded source and compares its bytes.",
            "detection: candidate promise charter/DONE/slice-a.md:7: P2 trace proof: every accepted result identifies the planning line it came from.",
            "detection: candidate promise charter/DONE/slice-b.md:3: P1 refusal proof: an unapproved candidate cannot host a holdout check.",
            "detection: candidate promise charter/DONE/slice-b.md:7: P2 repeat proof: a second extraction returns the identical ordered candidates.",
            "detection: candidate promise charter/DONE/slice-c.md:3: P1 boundary proof: source paths and line numbers survive the complete report path.",
            "detection: candidate promise charter/DONE/slice-c.md:4: P2 exclusion proof: headings, prose and short lines never become owner candidates.",
            "detection: candidate promise charter/GUARANTEES.yaml:3: A reviewer can reopen every accepted proof from its recorded source bytes.",
            "detection: candidate promise charter/GUARANTEES.yaml:6: Every owner approval traces to the exact guarantee statement that was reviewed.",
        ] {
            assert!(lines.contains(&expected.to_owned()), "missing {expected}");
        }
        assert!(!lines.iter().any(|line| line.contains("PROMISE:")));
        assert!(!lines.iter().any(|line| line.contains("Template proof")));
        assert!(!lines.iter().any(|line| line.contains("Draft proof")));
        assert!(!lines.iter().any(|line| line.contains("supporting prose")));
        assert!(!lines.iter().any(|line| line.contains("Tiny.")));
        assert!(!lines.contains(
            &"detection: candidate promise charter/DONE/slice-b.md:8: P3 short: do not approve this."
                .to_owned()
        ));
    }

    /// P2. Fail-without mutant: print every candidate; stderr exceeds 64 KB.
    #[test]
    fn report_prints_first_20_and_writes_the_full_list() {
        let fx = Fixture::new("numbered-charter", "p2");
        let done = fx.host.join("charter/DONE/slice-c.md");
        let mut bytes = fs::read_to_string(&done).unwrap();
        for number in 3..=10_000 {
            bytes.push_str(&format!(
                "{number}. P{number} scale proof: this real numbered proof exercises bounded reporting.\n"
            ));
        }
        fs::write(&done, bytes).unwrap();
        let out = fx.embed();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(
            out.stderr.len() < 64 * 1024,
            "stderr was {} bytes",
            out.stderr.len()
        );
        let lines = Fixture::detection_lines(&out);
        assert!(lines.contains(&"detection: candidate promises 10006".to_owned()));
        assert!(
            lines
                .contains(&"detection: source candidates charter/DONE/slice-c.md 10000".to_owned())
        );
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.starts_with("detection: candidate promise "))
                .count(),
            20
        );
        let relative = "hosts/host/candidate-promises.txt";
        assert!(lines.contains(&format!("detection: full candidate list {relative}")));
        assert!(
            !lines
                .iter()
                .any(|line| line.contains(fx.root.to_str().unwrap()))
        );
        let full = fs::read_to_string(fx.state.join("candidate-promises.txt")).unwrap();
        assert_eq!(full.lines().count(), 10_007);
        assert!(full.contains(
            "P1 reopen proof: a reviewer reopens the recorded source and compares its bytes.\n   This continuation is supporting prose rather than another approval candidate."
        ));
        assert!(full.contains("P10000 scale proof:"));
        assert_eq!(
            fs::read(fx.private.join("holden-mcp").join(relative)).unwrap(),
            fs::read(fx.state.join("candidate-promises.txt")).unwrap()
        );
    }

    /// P3. Fail-without mutant: alter PLANNING.md extraction or let charter
    /// precedence capture a host that has PLANNING.md.
    #[test]
    fn planning_hosts_yield_the_same_candidates() {
        let prose = Fixture::new("prose-markdown", "p3-prose");
        let out = prose.embed();
        let lines = Fixture::detection_lines(&out);
        for expected in [
            "detection: candidate promises 2",
            "detection: candidate promise README.md:3: A person can  recover their work after an interrupted save.  ",
            "detection: candidate promise docs/PRD.md:3: An agent can trace each approved requirement to its exact source bytes.",
        ] {
            assert!(lines.contains(&expected.to_owned()), "missing {expected}");
        }

        let planning = Fixture::new("numbered-charter", "p3-planning");
        fs::write(
            planning.host.join("PLANNING.md"),
            "# Planning\n\nPROMISE: A saved note survives an interrupted write and reopens exactly.\n\n- PROMISE: A reviewer can trace  each approved requirement to its source bytes.\n",
        )
        .unwrap();
        let out = planning.embed();
        let lines = Fixture::detection_lines(&out);
        assert!(lines.contains(&"detection: planning process markdown".to_owned()));
        assert!(lines.contains(&"detection: candidate promises 2".to_owned()));
        assert!(lines.contains(&"detection: source document PLANNING.md".to_owned()));
        assert!(!lines.iter().any(|line| line.contains("slice-a.md")));
        assert!(lines.contains(&"detection: candidate promise PLANNING.md:3: A saved note survives an interrupted write and reopens exactly.".to_owned()));
        assert!(lines.contains(&"detection: candidate promise PLANNING.md:5: A reviewer can trace  each approved requirement to its source bytes.".to_owned()));
    }

    /// slice-18e P5. Fail-without mutant: drop the no-candidates line; the
    /// intent document is again indistinguishable from a parse omission.
    #[test]
    fn source_without_candidates_is_named_with_reason() {
        let fx = Fixture::new("numbered-charter", "s18e-empty-source");
        let out = fx.embed();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        let lines = Fixture::detection_lines(&out);
        assert!(lines.contains(
            &"detection: no candidates charter/INTENT.md: the intent card states goals, not numbered proofs; candidates come from DONE proofs and GUARANTEES".to_owned()
        ));
        assert!(lines.contains(&"detection: source document charter/INTENT.md".to_owned()));
    }

    /// slice-18e P6. Fail-without mutant: accept numbered lines outside a
    /// Proofs block; the Steps decoy becomes a ninth candidate. The existing
    /// slice-18b P1 test above independently pins all eight old candidates.
    #[test]
    fn only_the_proofs_block_yields_charter_candidates() {
        let fx = Fixture::new("numbered-charter", "s18e-proofs-only");
        let out = fx.embed();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        let lines = Fixture::detection_lines(&out);
        assert!(lines.contains(&"detection: candidate promises 8".to_owned()));
        for excluded in ["P9 gate step", "P10 gate exclusion", "P11 scope exclusion"] {
            assert!(!lines.iter().any(|line| line.contains(excluded)));
        }
        assert!(lines.contains(
            &"detection: candidate promise charter/DONE/slice-a.md:5: P1 reopen proof: a reviewer reopens the recorded source and compares its bytes.".to_owned()
        ));
        assert!(lines.contains(
            &"detection: candidate promise charter/DONE/slice-b.md:7: P2 repeat proof: a second extraction returns the identical ordered candidates.".to_owned()
        ));
    }

    /// P4. Fail-without mutant: drop the report on the refusal path.
    #[test]
    fn refused_embed_prints_the_shortened_report() {
        let fx = Fixture::new("numbered-charter", "p4");
        let out = fx.embed();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        let stderr = String::from_utf8_lossy(&out.stderr);
        let refusal = stderr.find("refused:").unwrap();
        let report = stderr[..refusal].to_owned();
        assert!(report.contains("detection: planning process charter"));
        assert!(report.contains("detection: candidate promises 8"));
        assert!(report.contains("detection: source candidates charter/GUARANTEES.yaml 2"));
        assert_eq!(
            report
                .lines()
                .filter(|line| line.starts_with("detection: candidate promise "))
                .count(),
            8
        );
        assert!(
            report.contains("detection: full candidate list hosts/host/candidate-promises.txt\n")
        );
        assert!(!stderr[..refusal].contains(fx.root.to_str().unwrap()));
    }
}
