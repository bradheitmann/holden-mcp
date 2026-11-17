//! slice-18a: a refused embed still prints the detection report. The hosts
//! are real: the committed prose-markdown client fixture (planning
//! promises, no root Cargo.toml), the notes host fixture whose embed
//! succeeds, and a prose host that states no promise. The child is the
//! real `holden` binary; nothing is mocked. Fixture temp dirs are
//! Drop-guard cleaned (the slice-17 P7 pattern).

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

    /// The real binaries, built once into this run's own target directory.
    fn bins() -> &'static Bins {
        static BINS: OnceLock<Bins> = OnceLock::new();
        BINS.get_or_init(|| {
            let exe = std::env::current_exe().unwrap();
            let profile = exe.parent().and_then(Path::parent).unwrap();
            let target = profile.parent().unwrap();
            let manifest = workspace().join("Cargo.toml");
            for (package, binary) in [("holden-cli", "holden"), ("holden-eval", "holden-confine")] {
                build(
                    &[
                        "build",
                        "--locked",
                        "--quiet",
                        "-p",
                        package,
                        "--bin",
                        binary,
                        "--manifest-path",
                        manifest.to_str().unwrap(),
                    ],
                    target,
                );
            }
            let forge_target = target.join("embed-report-forge-build");
            build(
                &[
                    "build",
                    "--offline",
                    "--locked",
                    "--quiet",
                    "--manifest-path",
                    workspace()
                        .join("modules/holden-forge/Cargo.toml")
                        .to_str()
                        .unwrap(),
                ],
                &forge_target,
            );
            let b = Bins {
                holden: profile.join("holden"),
                confine: profile.join("holden-confine"),
                forge: forge_target.join("debug/forge"),
            };
            for path in [&b.holden, &b.confine, &b.forge] {
                assert!(path.is_file(), "{}", path.display());
            }
            b
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
            let dest = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &dest);
            } else {
                fs::copy(entry.path(), dest).unwrap();
            }
        }
    }

    fn committed(fx: &Fixture, message: &str) {
        let home = fx.root.join("git-home");
        fs::create_dir_all(&home).unwrap();
        let git = Git::new(&fx.host, &home);
        git.init().unwrap();
        git.commit_all(message).unwrap();
    }

    /// One test's private root: the host, its agent state, the owner key
    /// store and a synthetic denylist. The Drop guard removes the root.
    struct Fixture {
        root: PathBuf,
        host: PathBuf,
        state: PathBuf,
        owner_store: PathBuf,
        denylist: PathBuf,
    }

    impl Fixture {
        fn new(tag: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("holden-cli-s18a-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)
                .unwrap();
            let root = root.canonicalize().unwrap();
            let denylist = root.join("denylist.txt");
            fs::write(
                &denylist,
                "# a synthetic denylist for the forge export scan\nzz-synthetic-private-token\n",
            )
            .unwrap();
            Self {
                host: root.join("host"),
                state: root.join("state"),
                owner_store: root.join("owner"),
                denylist,
                root,
            }
        }

        /// The committed prose-markdown client fixture as its own git
        /// repository: planning promises, no root Cargo.toml. Its embed is
        /// refused with unsupported_stack.
        fn prose_client(tag: &str) -> Self {
            let fx = Self::new(tag);
            copy_tree(
                &workspace().join("fixtures/clients/prose-markdown"),
                &fx.host,
            );
            committed(&fx, "prose-markdown client");
            fx
        }

        /// The real numbered-charter Cargo host. Embed succeeds through the
        /// real forge and writes its complete candidate list to agent state.
        fn charter_client(tag: &str) -> Self {
            let fx = Self::new(tag);
            copy_tree(
                &workspace().join("fixtures/clients/numbered-charter"),
                &fx.host,
            );
            committed(&fx, "numbered-charter client");
            fx
        }

        /// The real blind-spot charter Cargo host, including short items,
        /// marked fallback lists, decoy lists and a source with no items.
        fn blindspot_client(tag: &str) -> Self {
            let fx = Self::new(tag);
            copy_tree(
                &workspace().join("fixtures/clients/blindspot-charter"),
                &fx.host,
            );
            committed(&fx, "blind-spot charter client");
            fx
        }

        /// The notes host fixture: a real Cargo product whose planning
        /// document carries the owner's marked promises. Embed succeeds.
        fn notes_host(tag: &str) -> Self {
            let fx = Self::new(tag);
            let out = Command::new(&bins().holden)
                .args(["demo-host", "--workspace"])
                .arg(workspace())
                .arg(&fx.host)
                .output()
                .unwrap();
            assert!(out.status.success(), "demo-host: {}", text(&out));
            fx
        }

        /// A git host whose only planning document states no promise: its
        /// markdown is a header and fenced text only.
        fn no_promise_host(tag: &str) -> Self {
            let fx = Self::new(tag);
            fs::create_dir_all(&fx.host).unwrap();
            fs::write(
                fx.host.join("README.md"),
                "# Notes\n\n```text\nnothing promised here\n```\n",
            )
            .unwrap();
            committed(&fx, "no promise stated");
            fx
        }

        /// The real `holden embed` child, with every option it needs.
        fn embed(&self) -> Output {
            let b = bins();
            Command::new(&b.holden)
                .args([
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
                ])
                .current_dir(&self.host)
                .env("AGENT_PRIVATE_ROOT", self.root.join("agent-private"))
                .output()
                .unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// The detection report lines of `out`, both streams included.
    fn report_lines(out: &Output) -> Vec<String> {
        text(out)
            .lines()
            .filter(|line| line.starts_with("detection:"))
            .map(str::to_owned)
            .collect()
    }

    /// The shape of a report: its field kinds in order, values and
    /// repetitions stripped (hosts differ in how many documents and
    /// promises they have; the field flow must not).
    fn shape(lines: &[String]) -> Vec<String> {
        let mut kinds: Vec<String> = Vec::new();
        for line in lines {
            // Slice-18e's no-candidate explanation is conditional on the
            // host's documents; the unconditional report fields stay pinned.
            if line.starts_with("detection: no candidates ") {
                continue;
            }
            let rest = line.strip_prefix("detection: ").unwrap_or(line);
            let kind = rest
                .split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ");
            if kinds.last().is_none_or(|last| last != &kind) {
                kinds.push(kind);
            }
        }
        kinds
    }

    /// Do not let the shape comparison's repeated-field collapsing hide a
    /// firehose: the actual occurrence count is the published total, capped
    /// at 20, independently on each path.
    fn assert_candidate_occurrences(lines: &[String]) {
        let total = lines
            .iter()
            .find_map(|line| line.strip_prefix("detection: candidate promises "))
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let occurrences = lines
            .iter()
            .filter(|line| line.starts_with("detection: candidate promise "))
            .count();
        assert_eq!(occurrences, total.min(20), "report occurrences: {lines:?}");
    }

    /// slice-18a P1: the prose-markdown host (planning promises, no root
    /// Cargo.toml) is refused with unsupported_stack, its output carries
    /// the detection report before the refusal line, and the exit code
    /// stays 1.
    /// Fail-without mutant: drop the report on the refusal path.
    #[test]
    fn refused_embed_still_prints_detection_report() {
        let fx = Fixture::prose_client("p1");
        let out = fx.embed();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{stderr}");
        let report = [
            "detection: planning process markdown",
            "detection: source document README.md",
            "detection: source document docs/PRD.md",
            "detection: candidate promises 2",
            "detection: candidate promise README.md:3: A person can  recover their work after an interrupted save.  ",
            "detection: candidate promise docs/PRD.md:3: An agent can trace each approved requirement to its exact source bytes.",
        ];
        let refusal = "refused: unsupported_stack: no root Cargo.toml; declare a crate with --manifest <relative-path> (repeatable)";
        let mut seen_to = 0usize;
        for needle in report {
            let at = stderr
                .find(needle)
                .unwrap_or_else(|| panic!("the report lacks '{needle}': {stderr}"));
            assert!(
                at >= seen_to,
                "the report is out of order at '{needle}': {stderr}"
            );
            seen_to = at + needle.len();
        }
        let at = stderr
            .find(refusal)
            .unwrap_or_else(|| panic!("no refusal line: {stderr}"));
        assert!(
            at >= seen_to,
            "the refusal line came before the report: {stderr}"
        );
    }

    /// slice-18a P2: the report on a refusal has the same fields, in the
    /// same order, as the report on success.
    /// Fail-without mutant: reorder or drop a field on the refusal path's
    /// report — the shape comparison goes red.
    #[test]
    fn refusal_report_matches_success_shape() {
        let ok = Fixture::notes_host("p2-success");
        let success = ok.embed();
        assert_eq!(success.status.code(), Some(0), "{}", text(&success));
        let refused = Fixture::prose_client("p2-refused");
        let failure = refused.embed();
        assert_eq!(failure.status.code(), Some(1), "{}", text(&failure));

        let success_lines = report_lines(&success);
        let refusal_lines = report_lines(&failure);
        assert_candidate_occurrences(&success_lines);
        assert_candidate_occurrences(&refusal_lines);
        let success_shape = shape(&success_lines);
        let refusal_shape = shape(&refusal_lines);
        assert_eq!(
            success_shape.first().map(String::as_str),
            Some("planning process")
        );
        assert!(!refusal_shape.is_empty(), "the refusal carries no report");
        assert_eq!(
            refusal_shape, success_shape,
            "the refusal report's shape drifted from the success report's"
        );
        let count_at = refusal_shape
            .iter()
            .position(|field| field == "candidate promises")
            .unwrap();
        let text_at = refusal_shape
            .iter()
            .position(|field| field == "candidate promise")
            .unwrap();
        assert!(count_at < text_at, "the count came after the texts");
    }

    /// slice-18b D-QA-18B-2: a successful charter embed has the same hard
    /// output bound as the refusal path. Fail-without mutant: remove only
    /// the success printer's `.take(20)`; the occurrence assertion goes red.
    #[test]
    fn successful_charter_report_stays_bounded() {
        let fx = Fixture::charter_client("s18b-success-bound");
        let done = fx.host.join("charter/DONE/slice-c.md");
        let mut source = fs::read_to_string(&done).unwrap();
        for number in 3..=22 {
            source.push_str(&format!(
                "{number}. P{number} success scale proof: this real numbered proof bounds successful reporting.\n"
            ));
        }
        fs::write(done, source).unwrap();
        let out = fx.embed();
        assert_eq!(out.status.code(), Some(0), "{}", text(&out));
        assert!(
            out.stdout.len() < 64 * 1024,
            "success stdout was {} bytes",
            out.stdout.len()
        );
        let lines = report_lines(&out);
        assert!(lines.contains(&"detection: candidate promises 28".to_owned()));
        assert!(
            lines.contains(&"detection: source candidates charter/DONE/slice-c.md 22".to_owned())
        );
        assert_candidate_occurrences(&lines);
        assert!(lines.contains(
            &"detection: full candidate list hosts/host/candidate-promises.txt".to_owned()
        ));
        assert!(
            !lines
                .iter()
                .any(|line| line.contains("P22 success scale proof"))
        );
        let full = fs::read_to_string(fx.state.join("candidate-promises.txt")).unwrap();
        assert_eq!(full.lines().count(), 29);
        assert!(full.contains(
            "P1 reopen proof: a reviewer reopens the recorded source and compares its bytes.\n   This continuation is supporting prose rather than another approval candidate."
        ));
        assert!(full.contains("P22 success scale proof"));
    }

    /// slice-18a P3: a host with no planning promises is refused as
    /// before, with no report, and its refusal text is unchanged.
    /// Fail-without mutant: print a report (even an empty one) on the
    /// no-promise path.
    #[test]
    fn no_promise_refusal_prints_no_report() {
        let fx = Fixture::no_promise_host("p3");
        let out = fx.embed();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            "refused: the planning adapter states no owner promise\n",
            "the no-promise refusal text changed"
        );
        assert!(
            !text(&out).contains("detection:"),
            "the no-promise refusal printed a report: {}",
            text(&out)
        );
    }

    /// slice-18e P1. Fail-without mutant: drop the next-steps lines; the
    /// exact stdout suffix, including the proposal's owner command, is gone.
    #[test]
    fn successful_embed_prints_next_steps_from_promise_proposals() {
        let fx = Fixture::notes_host("s18e-next-steps");
        let out = fx.embed();
        assert_eq!(out.status.code(), Some(0), "{}", text(&out));
        let proposal: serde_json::Value = serde_json::from_slice(
            &fs::read(fx.host.join(".holden/promise-proposals.json")).unwrap(),
        )
        .unwrap();
        let owner = proposal["owner_command"].as_str().unwrap();
        assert_eq!(owner, "holden approve --host .");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.ends_with(&format!(
                "next steps:\n  OWNER ONLY: {owner}\n  holden agent serve --host .\n"
            )),
            "the successful embed did not end with its next steps: {stdout}"
        );
    }

    /// slice-18e P4. Fail-without mutant: remove a stale cognition without
    /// printing its line; the before/after directory difference has no
    /// matching `removed:` line and the removal count is wrong.
    #[test]
    fn removed_files_print_lines_and_the_report_counts_them() {
        let fx = Fixture::notes_host("s18e-removed");
        let first = fx.embed();
        assert_eq!(first.status.code(), Some(0), "{}", text(&first));
        let stale = fx.host.join(".holden/cognitions/stale-cognition.json");
        fs::write(&stale, b"{\"stale\":true}\n").unwrap();
        let before: std::collections::BTreeSet<PathBuf> = fs::read_dir(stale.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into())
            .collect();

        let out = fx.embed();
        assert_eq!(out.status.code(), Some(0), "{}", text(&out));
        assert!(!stale.exists());
        let after: std::collections::BTreeSet<PathBuf> = fs::read_dir(stale.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into())
            .collect();
        let disappeared: Vec<_> = before.difference(&after).collect();
        assert_eq!(
            disappeared,
            vec![&std::ffi::OsString::from("stale-cognition.json")]
        );

        let stdout = String::from_utf8_lossy(&out.stdout);
        let removed: Vec<_> = stdout
            .lines()
            .filter(|line| line.starts_with("removed: "))
            .collect();
        assert_eq!(
            removed,
            vec!["removed: .holden/cognitions/stale-cognition.json"],
            "each disappeared file needs exactly one host-relative line: {stdout}"
        );
        let unchanged = stdout
            .lines()
            .filter(|line| line.starts_with("same: "))
            .count();
        let written = stdout
            .lines()
            .filter(|line| line.starts_with("wrote: "))
            .count();
        assert!(stdout.contains(&format!(
            "files: {written} written, {unchanged} unchanged, 1 removed\nnext steps:"
        )));
    }

    /// slice-18h P6. Fail-without mutant: drop per-source accounting; the
    /// three exact report wordings and all reason counts disappear.
    #[test]
    fn per_source_report_counts_items_taken_and_left_out() {
        let fx = Fixture::blindspot_client("s18h-source-accounting");
        let out = fx.embed();
        assert_eq!(out.status.code(), Some(0), "{}", text(&out));
        let lines = report_lines(&out);
        for expected in [
            "detection: source numbered items charter/DONE/slice-short.md seen 4 taken 3 too short 1 outside any proof list 0",
            "detection: source numbered items charter/DONE/slice-plain-continuations.md seen 1 taken 0 too short 1 outside any proof list 0",
            "detection: numbered items found, none taken charter/DONE/slice-plain-continuations.md 1",
            "detection: source numbered items charter/DONE/slice-empty-label.md seen 2 taken 2 too short 0 outside any proof list 0",
            "detection: source numbered items charter/DONE/slice-anywhere.md seen 6 taken 2 too short 0 outside any proof list 4",
            "detection: source numbered items charter/DONE/slice-title-proof.md seen 5 taken 5 too short 0 outside any proof list 0",
            "detection: source numbered items charter/DONE/slice-outside.md seen 2 taken 0 too short 0 outside any proof list 2",
            "detection: source numbered items charter/DONE/slice-none.md seen 0 taken 0 too short 0 outside any proof list 0",
            "detection: numbered items found, none taken charter/DONE/slice-outside.md 2",
            "detection: no numbered items charter/DONE/slice-none.md",
        ] {
            assert!(
                lines.contains(&expected.to_owned()),
                "missing report line '{expected}': {lines:#?}"
            );
        }
        assert!(!lines.iter().any(|line| {
            line.starts_with("detection: numbered items found, none taken ")
                && line.contains("slice-none.md")
        }));
    }

    /// slice-18a P4: the report carries no machine path and no sealed
    /// value, on the refusal path and on success alike.
    /// Fail-without mutant: include the socket or state path in the
    /// report — the test goes red.
    #[test]
    fn detection_report_carries_no_machine_values() {
        let refused = Fixture::prose_client("p4-refused");
        let failure = refused.embed();
        assert_eq!(failure.status.code(), Some(1), "{}", text(&failure));
        let ok = Fixture::notes_host("p4-success");
        let success = ok.embed();
        assert_eq!(success.status.code(), Some(0), "{}", text(&success));

        for (out, fx) in [(&failure, &refused), (&success, &ok)] {
            let report = report_lines(out);
            assert!(!report.is_empty(), "no report to check");
            let machine = fx.root.to_string_lossy().into_owned();
            for line in &report {
                assert!(
                    !line.contains(machine.as_str()),
                    "the report names a machine path: {line}"
                );
                assert!(!line.contains(".sock"), "the report names a socket: {line}");
                assert!(
                    !line.contains("owner-signing"),
                    "the report names a sealed key store: {line}"
                );
                if let Some(doc) = line.strip_prefix("detection: source document ") {
                    assert!(
                        !doc.starts_with('/'),
                        "the report carries an absolute document path: {line}"
                    );
                }
            }
        }
    }
}
