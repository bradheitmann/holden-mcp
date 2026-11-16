//! Real-repository end-to-end orchestration with a bounded test process.
//!
//! The tests perform the actual MCP flows. This target owns only their build
//! and process deadline, and forwards the private seed location from the
//! caller's environment.

use std::path::{Path, PathBuf};

use holden_eval::spawn::{self, Limits, Spawn};

use crate::util;

/// Run the named real-repository suite. Only the slice-11 target exists.
pub fn run(root: &Path, slice: &str) -> bool {
    if slice != "slice-11" {
        eprintln!("refused: no e2e target for {slice}");
        return false;
    }
    let target = util::target_dir(&format!(
        "s11-e2e-{}-{}",
        std::process::id(),
        util::unix_now()
    ));
    let current = std::env::current_dir().unwrap_or_else(|_| root.to_path_buf());
    let path = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".to_owned());
    let cargo = std::env::var_os("CARGO").map_or_else(|| PathBuf::from("cargo"), PathBuf::from);
    let toolchain = std::fs::read_to_string(root.join("rust-toolchain.toml"))
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.trim()
                    .strip_prefix("channel = \"")
                    .and_then(|value| value.strip_suffix('"'))
                    .map(str::to_owned)
            })
        })
        .unwrap_or_else(|| "1.95.0".to_owned());
    let mut env = vec![
        ("PATH".to_owned(), path),
        (
            "HOME".to_owned(),
            util::home().unwrap_or(current).display().to_string(),
        ),
        ("CARGO_TARGET_DIR".to_owned(), target.display().to_string()),
        ("CARGO_TERM_COLOR".to_owned(), "never".to_owned()),
        ("RUSTUP_TOOLCHAIN".to_owned(), toolchain),
        ("RUSTC_WRAPPER".to_owned(), String::new()),
        ("RUSTC_WORKSPACE_WRAPPER".to_owned(), String::new()),
    ];
    // When the explicit seed is absent the test itself resolves the ordinary
    // private-root fallback and still fails typed if it cannot find it.
    for name in ["HOLDEN_E2E_REAL_REPO", "AGENT_PRIVATE_ROOT"] {
        if let Some(value) = std::env::var_os(name).filter(|value| !value.is_empty()) {
            let Some(value) = value.to_str() else {
                eprintln!("could_not_run: {name} is not valid UTF-8");
                return false;
            };
            env.push((name.to_owned(), value.to_owned()));
        }
    }
    let spec = Spawn {
        program: cargo,
        args: vec![
            "test".into(),
            "--manifest-path".into(),
            root.join("Cargo.toml").into_os_string(),
            "--locked".into(),
            "-p".into(),
            "holden-embed".into(),
            "--test".into(),
            "real_repo".into(),
            "--".into(),
            "--test-threads=1".into(),
        ],
        cwd: root.to_path_buf(),
        env,
        stdin: Vec::new(),
    };
    println!("e2e: slice-11: {} tests, target {}", 4, target.display());
    let succeeded = match spawn::run(
        &spec,
        &Limits {
            run_ms: 7_200_000,
            capture_bytes: 16 * 1024 * 1024,
            cpu_secs: 7_200,
            file_bytes: 1024 * 1024 * 1024,
            open_files: 1024,
        },
    ) {
        Ok(outcome) => {
            print!("{}", outcome.stdout.text());
            eprint!("{}", outcome.stderr.text());
            outcome.exit.success()
        }
        Err(error) => {
            eprintln!("could_not_run: e2e process: {error}");
            false
        }
    };
    let _ = std::fs::remove_dir_all(&target);
    succeeded
}
