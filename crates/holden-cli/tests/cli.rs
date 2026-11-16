//! The `holden` binary, exercised as a real process.

// Test helpers outside #[test] fns: a failed setup step should abort the test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn holden(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_holden"))
        .args(args)
        .output()
        .expect("run holden")
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../holden-canon/fixtures")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("holden-cli-test-{name}-{}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn canon_verify_passes_committed_synthetic_fixture() {
    let f = fixtures();
    let out = holden(&[
        "canon",
        "verify",
        "--root",
        f.join("canon-synthetic/root").to_str().unwrap(),
        "--pin",
        f.join("canon-synthetic/CANON-PIN.txt").to_str().unwrap(),
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("verified: 3 file(s)"), "{stdout}");
    assert!(stdout.contains("digest: "), "{stdout}");
}

#[test]
fn canon_verify_exits_1_naming_modified_file() {
    let f = fixtures();
    let root = scratch("modified").join("root");
    fs::create_dir_all(root.join("taxonomy")).unwrap();
    for rel in ["doctrine.md", "schema.yaml", "taxonomy/codes.md"] {
        fs::copy(f.join("canon-synthetic/root").join(rel), root.join(rel)).unwrap();
    }
    let target = root.join("schema.yaml");
    let mut bytes = fs::read(&target).unwrap();
    bytes[3] ^= 0x20;
    fs::write(&target, bytes).unwrap();
    let out = holden(&[
        "canon",
        "verify",
        "--root",
        root.to_str().unwrap(),
        "--pin",
        f.join("canon-synthetic/CANON-PIN.txt").to_str().unwrap(),
    ]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("differs: schema.yaml"), "{stderr}");
}

#[test]
fn canon_verify_absent_root_exits_3_canon_absent() {
    let f = fixtures();
    let absent = scratch("absent").join("nope");
    let out = holden(&[
        "canon",
        "verify",
        "--root",
        absent.to_str().unwrap(),
        "--pin",
        f.join("canon-synthetic/CANON-PIN.txt").to_str().unwrap(),
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(3), "{stdout}");
    assert!(stdout.contains("limitation canon_absent"), "{stdout}");
}

#[test]
fn import_lens_reproduces_committed_fixture_manifest() {
    let f = fixtures();
    let to = scratch("import").join("lens");
    let out = holden(&[
        "canon",
        "import-lens",
        "--from",
        f.join("lens-source-synthetic").to_str().unwrap(),
        "--to",
        to.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::read(to.join("MANIFEST.sha256")).unwrap(),
        fs::read(f.join("lens-synthetic/MANIFEST.sha256")).unwrap()
    );
}

#[test]
fn canon_verify_defaults_to_committed_canon() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new(env!("CARGO_BIN_EXE_holden"))
        .args(["canon", "verify"])
        .current_dir(&repo)
        .output()
        .expect("run holden");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("verified: 4 file(s)"), "{stdout}");
    assert!(
        stdout.contains("canon_version: skill-v2.3.0 adapter-v0.2.0"),
        "{stdout}"
    );
}

/// slice-3 P1: `holden canon doctrine` resolves every citation against the
/// committed canon and prints the typed doctrine; a changed cited section
/// is refused (exit 1) and named.
#[test]
fn canon_doctrine_prints_resolved_doctrine() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new(env!("CARGO_BIN_EXE_holden"))
        .args(["canon", "doctrine"])
        .current_dir(&repo)
        .output()
        .expect("run holden");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for needle in [
        "\"hospitality\"",
        "\"situated_cognition\"",
        "\"regenerative_loop\"",
        "\"measured_reliability\"",
        "\"structural_independence\"",
        "\"successor\"",
        "\"handoff\"",
        "citations resolved: 19",
    ] {
        assert!(stdout.contains(needle), "{needle} missing:\n{stdout}");
    }

    let root = scratch("doctrine").join("holden");
    fs::create_dir_all(&root).unwrap();
    let spec = "holdout_skill_spec_v2_3_0.md";
    let text = fs::read_to_string(repo.join("canon/holden").join(spec)).unwrap();
    fs::write(
        root.join(spec),
        text.replacen("is not empty data", "is empty data", 1),
    )
    .unwrap();
    let out = holden(&["canon", "doctrine", "--root", root.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("refused: derivation AbsenceProbe")
            && stderr.contains("### 1.6 The Nothing Doctrine"),
        "{stderr}"
    );
}

/// slice-4: `holden replay` refuses a file that is not a signed verdict
/// record (exit 1) and names why; a missing path is a usage error (exit 2).
/// Fail-without mutant: make `run_replay` exit 0 when the record does not
/// parse.
#[test]
fn replay_refuses_a_non_record() {
    let dir = scratch("replay");
    let path = dir.join("not-a-record.json");
    fs::write(&path, b"{\"body\": {}, \"signature\": \"00\"}").unwrap();
    let out = holden(&["replay", path.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("refused: the record is malformed"),
        "{stderr}"
    );
    let out = holden(&["replay"]);
    assert_eq!(out.status.code(), Some(2));
}

/// slice-5 P1 reopen: `holden ledger verify` passes an untouched signed
/// ledger (exit 0), names an edited record (exit 1), refuses a ledger
/// signed by another key than the pinned one, and treats a missing ledger
/// as a limitation (exit 3), never creating it.
/// Fail-without mutant: make `run_ledger_verify` exit 0 with findings.
#[test]
fn ledger_verify_names_tampered_record() {
    let dir = scratch("ledger");
    let db = dir.join("ledger.sqlite");
    let key =
        holden_seal::EvaluatorKey::from_material(&holden_seal::KeyMaterial::from_bytes([7u8; 32]));
    let public = hex::encode(key.public().0);
    let mut ledger = holden_ledger::Ledger::open(&db).unwrap();
    for i in 0..3 {
        ledger
            .append(&key, "note", &format!("{{\"i\":{i}}}"))
            .unwrap();
    }
    drop(ledger);
    let db_s = db.to_str().unwrap();
    let out = holden(&["ledger", "verify", "--db", db_s, "--key", &public]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("records: 3"), "{stdout}");
    assert!(stdout.contains("verified: every record holds"), "{stdout}");
    assert!(stdout.contains("(pinned)"), "{stdout}");
    let other = hex::encode([1u8; 32]);
    let out = holden(&["ledger", "verify", "--db", db_s, "--key", &other]);
    assert_eq!(out.status.code(), Some(1));
    {
        let c = rusqlite::Connection::open(&db).unwrap();
        c.execute_batch("DROP TRIGGER ledger_records_no_update;")
            .unwrap();
        c.execute(
            "UPDATE ledger_records SET body = '{\"i\":9}' WHERE seq = 2",
            [],
        )
        .unwrap();
    }
    let out = holden(&["ledger", "verify", "--db", db_s]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("refused: record 2 (note): edited"),
        "{stderr}"
    );
    let missing = dir.join("absent.sqlite");
    let out = holden(&["ledger", "verify", "--db", missing.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(3));
    assert!(!missing.exists());
}
