//! slice-18e: actionable, command-scoped CLI output. These tests execute
//! the real `holden` binary and a real signed ledger; nothing is mocked.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::fs::{self, DirBuilder};
    use std::os::unix::fs::DirBuilderExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};
    use std::sync::OnceLock;

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

    fn run(args: &[&str]) -> Output {
        Command::new(holden()).args(args).output().unwrap()
    }

    /// slice-18e P2. Fail-without mutant: make `--help` print the global
    /// usage; the second `holden` block and exit-code footer reappear.
    #[test]
    fn per_command_help_prints_only_that_command() {
        for args in [
            &["canon", "verify", "--help"][..],
            &["canon", "import-lens", "--help"][..],
            &["canon", "doctrine", "--help"][..],
            &["replay", "--help"][..],
            &["ledger", "verify", "--help"][..],
            &["ledger", "telemetry", "--help"][..],
            &["init", "--help"][..],
            &["mcp", "--help"][..],
            &["embed", "--help"][..],
            &["approve", "--help"][..],
            &["migrate-owner-key", "--help"][..],
            &["agent", "serve", "--help"][..],
            &["jit", "--help"][..],
            &["trace", "--help"][..],
            &["connect", "--help"][..],
            &["demo-host", "--help"][..],
        ] {
            let out = run(args);
            let stdout = String::from_utf8_lossy(&out.stdout);
            let command = args[..args.len() - 1].join(" ");
            assert_eq!(out.status.code(), Some(0), "{command}: {stdout}");
            assert!(out.stderr.is_empty(), "{command} wrote stderr");
            assert!(
                stdout.starts_with(&format!("usage:\n  holden {command} ")),
                "{command} did not print its own usage: {stdout}"
            );
            assert_eq!(
                stdout.matches("\n  holden ").count(),
                1,
                "{command} printed another command's usage: {stdout}"
            );
            assert!(
                !stdout.contains("exit codes:"),
                "{command} printed the global footer: {stdout}"
            );
        }
    }

    /// slice-18e P2. Fail-without mutant: route the empty argument vector
    /// through the unknown-command error; the forbidden line reappears.
    #[test]
    fn bare_holden_prints_global_usage_exits_2() {
        let out = run(&[]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{stderr}");
        assert!(out.stdout.is_empty());
        assert!(
            stderr.starts_with("usage:\n  holden canon verify "),
            "{stderr}"
        );
        assert!(stderr.contains("\n  holden demo-host "), "{stderr}");
        assert!(!stderr.contains("unknown command"), "{stderr}");
    }

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("holden-cli-s18e-ledger-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)
                .unwrap();
            Self(root)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// slice-18e P3. Fail-without mutant: print `ledger.path()` directly;
    /// both commands disclose the sentinel private root and lose the pinned
    /// `db/ledger.sqlite` line.
    #[test]
    fn ledger_paths_are_relative_to_the_private_root() {
        let scratch = Scratch::new();
        let private_base = scratch.0.join("agent-private");
        let private_root = private_base.join("holden-mcp");
        let db = private_root.join("db/ledger.sqlite");
        fs::create_dir_all(db.parent().unwrap()).unwrap();
        let key = holden_seal::EvaluatorKey::from_material(&holden_seal::KeyMaterial::from_bytes(
            [29u8; 32],
        ));
        let mut ledger = holden_ledger::Ledger::open(&db).unwrap();
        ledger
            .append(&key, "note", "{\"proof\":\"slice-18e\"}")
            .unwrap();
        drop(ledger);

        for command in ["verify", "telemetry"] {
            let out = Command::new(holden())
                .args(["ledger", command])
                .env("AGENT_PRIVATE_ROOT", &private_base)
                .env("HOME", scratch.0.join("sentinel-home"))
                .env("XDG_DATA_HOME", scratch.0.join("sentinel-xdg"))
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert_eq!(out.status.code(), Some(0), "{command}: {stdout}");
            assert!(
                stdout
                    .lines()
                    .any(|line| line == "ledger: db/ledger.sqlite"),
                "{command} did not print the private-root-relative path: {stdout}"
            );
            assert!(
                !stdout.contains(scratch.0.to_str().unwrap()),
                "{command} disclosed a machine path: {stdout}"
            );
        }
    }
}
