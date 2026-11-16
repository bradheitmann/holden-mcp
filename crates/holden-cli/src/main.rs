//! `holden`: the holden-mcp command-line tool.
//!
//! Exit codes: 0 pass, 1 refused, 2 usage error, 3 typed limitation (the
//! check could not run; never a pass).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod embed_cmd;
mod mcp_cmd;

use holden_canon::import::{import_lens, write_corpus};
use holden_canon::{CanonError, Doctrine, Pin, verify};

const USAGE: &str = "usage:
  holden canon verify [--root <dir>] [--pin <file>]
      defaults: --root canon/holden --pin canon/CANON-PIN.txt (the committed
      S-DOCTRINE canon, relative to the current directory); --root overrides,
      for example with a private-root copy
  holden canon import-lens --from <S-LENS registry dir> --to <empty corpus dir>
  holden canon doctrine [--root <dir>]
      prints the typed doctrine (definition, roots, constituencies, axes,
      generator rules) as JSON after resolving every citation against the
      canon root (default canon/holden); exit 1 names each unresolved one
  holden replay (<record.json> | <run id>) [--host <dir>] [--evaluator-key <64 hex>]
      resolves a run id through the current host's agent state, then recomputes
      the signed verdict record's decision from its stored inputs
      alone (signature, VRF parameter proof, planted values, observation
      digests, rules) and prints it; exit 1 when the record is refused or
      the decision differs
  holden ledger verify [--db <file>] [--key <64 hex>] [--tip <seq>:<hash>]
      verifies the whole signed ledger (default: the private root's
      db/ledger.sqlite): every record's hash, signature, sequence and link;
      --key pins the signer, --tip must still be in the ledger (catches
      truncation); exit 1 names each record that does not hold
  holden ledger telemetry [--db <file>]
      prints every measured rate with its sample size and 95% Wilson bounds
      (authority under the default bars) as JSON
  holden init [--root <private root>]
      creates the sealed implementer server config and private keys safely
      and idempotently; prints no secret
  holden mcp [--root <private root>]
      serves MCP on stdio, starting the sealed socket server on demand
  holden embed <project> --forge <forge binary> [--state <dir>]
      [--manifest <relative-path>]... [--denylist <file>] [--lens <dir>]
      [--cargo <file>] [--confine-helper <file>] [--owner-store <dir>]
      [--planning <file>]
      installs a separate holden agent into the host git repository: its
      public run file, constitution (forged for the host's domain through
      the forge's export), cognitions' public halves, the implementer's
      .mcp.json entry and the post-commit shim; everything sealed goes to the
      agent's state (default <private root>/hosts/<name>). Idempotent.
      Declare each non-root crate with repeatable --manifest <relative-path>;
      without a root Cargo.toml or a declaration the refusal names this flag.
      The post-commit hook in the repository's common git directory is shared
      by every worktree. Planning is detected through charter/ even when no
      PLANNING.md exists. Embed only runs cargo metadata; the post-commit jit
      and W2 run cargo build and test in an exported, confined world, never in
      the host worktree. --state defaults to <private root>/hosts/<name>.
      defaults: --denylist <private root>/denylist.txt, --lens the synthetic
      lens corpus shipped inside this executable (independent of the current
      directory), --cargo the active toolchain's cargo, --confine-helper
      holden-confine beside this binary, --owner-store
      <private root>/owner-keys, --planning PLANNING.md
  holden approve --host <dir> [--owner-store <dir>] [--create-owner-key]
      the OWNER signs the host's promises and cognitions with the owner key
      from the owner's key store, publishes its public key there and
      re-admits every approval against it; macOS requires the owner's terminal
  holden migrate-owner-key [--owner-store <dir>]
      OWNER ONLY, once: migrate the existing owner signing key into macOS
      Keychain without changing owner.pub or printing key bytes
  holden agent serve (--state <dir> | --host <dir>)
      runs the embedded agent as its own process with the evaluator role:
      admits only owner-approved cognitions, seals their plans, serves its
      own socket and the host's implementer socket, and commits a nonce for
      every plan before the commits it will evaluate exist
  holden jit [--host <dir>]
      the post-commit run (default host: the current directory): for each
      plan, the pre-committed nonce, begin_run on the new commit, the
      revealed parameters, the confined run, the signed verdict in the
      ledger, the audit reveal, then the fixed diagnostic
  holden trace <run id> [--host <dir>]
      prints and re-verifies the chain promise -> approved cognition ->
      root -> lens -> plan -> params -> verdict of one run
  holden connect --role implementer|evaluator --socket <path>
      bridges stdio to the holden server's socket (the .mcp.json command)
  holden demo-host <dest> [--workspace <dir>]
      creates the host fixture repository from the committed files

exit codes: 0 pass, 1 refused, 2 usage, 3 limitation (never a pass)";

#[derive(Debug)]
enum Command {
    Verify {
        root: PathBuf,
        pin: PathBuf,
    },
    ImportLens {
        from: PathBuf,
        to: PathBuf,
    },
    Doctrine {
        root: PathBuf,
    },
    Replay {
        record: PathBuf,
        host: Option<PathBuf>,
        key: Option<String>,
    },
    LedgerVerify {
        db: Option<PathBuf>,
        key: Option<String>,
        tip: Option<String>,
    },
    LedgerTelemetry {
        db: Option<PathBuf>,
    },
}

#[derive(Debug)]
struct Usage(String);

/// The committed S-DOCTRINE canon root (owner decision D-08, 2026-11-04).
const DEFAULT_CANON_ROOT: &str = "canon/holden";
/// The committed canon pin.
const DEFAULT_CANON_PIN: &str = "canon/CANON-PIN.txt";

fn take_flag_values(args: &[String], names: [&str; 2]) -> Result<[Option<PathBuf>; 2], Usage> {
    let mut values: [Option<PathBuf>; 2] = [None, None];
    let mut i = 0;
    while i < args.len() {
        let pos = names
            .iter()
            .position(|n| *n == args[i])
            .ok_or_else(|| Usage(format!("unknown argument '{}'", args[i])))?;
        let v = args
            .get(i + 1)
            .ok_or_else(|| Usage(format!("{} needs a value", names[pos])))?;
        if values[pos].replace(PathBuf::from(v)).is_some() {
            return Err(Usage(format!("{} given twice", names[pos])));
        }
        i += 2;
    }
    Ok(values)
}

fn take_flags(args: &[String], names: [&str; 2]) -> Result<[PathBuf; 2], Usage> {
    let [a, b] = take_flag_values(args, names)?;
    Ok([
        a.ok_or_else(|| Usage(format!("missing {}", names[0])))?,
        b.ok_or_else(|| Usage(format!("missing {}", names[1])))?,
    ])
}

fn parse(args: &[String]) -> Result<Command, Usage> {
    match args {
        [group, cmd, rest @ ..] if group == "canon" && cmd == "verify" => {
            let [root, pin] = take_flag_values(rest, ["--root", "--pin"])?;
            Ok(Command::Verify {
                root: root.unwrap_or_else(|| PathBuf::from(DEFAULT_CANON_ROOT)),
                pin: pin.unwrap_or_else(|| PathBuf::from(DEFAULT_CANON_PIN)),
            })
        }
        [group, cmd, rest @ ..] if group == "canon" && cmd == "import-lens" => {
            let [from, to] = take_flags(rest, ["--from", "--to"])?;
            Ok(Command::ImportLens { from, to })
        }
        [group, cmd, rest @ ..] if group == "canon" && cmd == "doctrine" => {
            let root = match rest {
                [] => PathBuf::from(DEFAULT_CANON_ROOT),
                [flag, v] if flag == "--root" => PathBuf::from(v),
                _ => return Err(Usage("doctrine takes only --root <dir>".to_owned())),
            };
            Ok(Command::Doctrine { root })
        }
        [group, cmd, rest @ ..] if group == "ledger" && cmd == "verify" => {
            let mut db = None;
            let mut key = None;
            let mut tip = None;
            let mut i = 0;
            while i < rest.len() {
                let v = rest
                    .get(i + 1)
                    .ok_or_else(|| Usage(format!("{} needs a value", rest[i])))?
                    .clone();
                let slot = match rest[i].as_str() {
                    "--db" => &mut db,
                    "--key" => &mut key,
                    "--tip" => &mut tip,
                    other => return Err(Usage(format!("unknown argument '{other}'"))),
                };
                if slot.replace(v).is_some() {
                    return Err(Usage(format!("{} given twice", rest[i])));
                }
                i += 2;
            }
            Ok(Command::LedgerVerify {
                db: db.map(PathBuf::from),
                key,
                tip,
            })
        }
        [group, cmd, rest @ ..] if group == "ledger" && cmd == "telemetry" => {
            let db = match rest {
                [] => None,
                [flag, v] if flag == "--db" => Some(PathBuf::from(v)),
                _ => return Err(Usage("telemetry takes only --db <file>".to_owned())),
            };
            Ok(Command::LedgerTelemetry { db })
        }
        [cmd, record, rest @ ..] if cmd == "replay" => {
            let mut key = None;
            let mut host = None;
            let mut index = 0;
            while index < rest.len() {
                let value = rest
                    .get(index + 1)
                    .ok_or_else(|| Usage(format!("{} needs a value", rest[index])))?
                    .clone();
                let slot = match rest[index].as_str() {
                    "--evaluator-key" => &mut key,
                    "--host" => &mut host,
                    other => return Err(Usage(format!("unknown replay argument '{other}'"))),
                };
                if slot.replace(value).is_some() {
                    return Err(Usage(format!("{} given twice", rest[index])));
                }
                index += 2;
            }
            Ok(Command::Replay {
                record: PathBuf::from(record),
                host: host.map(PathBuf::from),
                key,
            })
        }
        _ => Err(Usage("unknown command".to_owned())),
    }
}

fn run_verify(root: &Path, pin_path: &Path) -> ExitCode {
    let text = match std::fs::read_to_string(pin_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("refused: cannot read pin {}: {e}", pin_path.display());
            return ExitCode::from(1);
        }
    };
    let pin = match Pin::parse(&text) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("refused: {e}");
            return ExitCode::from(1);
        }
    };
    match verify(root, &pin) {
        Ok(report) => {
            println!("verified: {} file(s)", report.file_count);
            println!("canon_version: {}", report.canon_version);
            if let Some(v) = report.validator_hash {
                println!("validator_hash: {v}");
            }
            println!("digest: {}", report.digest);
            ExitCode::SUCCESS
        }
        Err(e @ CanonError::CanonAbsent { .. }) => {
            println!("{e}");
            ExitCode::from(3)
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn run_import(from: &Path, to: &Path) -> ExitCode {
    if !from.is_dir() {
        println!(
            "limitation canon_absent: source {} does not exist; nothing was imported",
            from.display()
        );
        return ExitCode::from(3);
    }
    let outcome = match import_lens(from) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("refused: {e}");
            return ExitCode::from(1);
        }
    };
    match write_corpus(to, &outcome) {
        Ok(pin) => {
            println!(
                "imported: {} concept(s), {} situated cognition(s), {} retired",
                outcome.corpus.concepts.len(),
                outcome.corpus.cognitions.len(),
                outcome.corpus.retired.len()
            );
            for d in &outcome.deltas {
                match holden_canon::lens::json_bytes(d) {
                    Ok(bytes) => println!("delta: {}", String::from_utf8_lossy(&bytes).trim_end()),
                    Err(e) => eprintln!("delta could not be printed: {e}"),
                }
            }
            println!("manifest digest: {}", pin.listing_digest());
            print!("{}", pin.listing());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("refused: {e}");
            ExitCode::from(1)
        }
    }
}

fn run_doctrine(root: &Path) -> ExitCode {
    if !root.is_dir() {
        println!(
            "limitation canon_absent: canon root {} does not exist; no citation was resolved",
            root.display()
        );
        return ExitCode::from(3);
    }
    let doctrine = Doctrine::holden();
    match doctrine.verify(root) {
        Ok(n) => match doctrine.to_json() {
            Ok(json) => {
                print!("{json}");
                println!("citations resolved: {n}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("refused: the doctrine could not be printed: {e}");
                ExitCode::from(1)
            }
        },
        Err(failures) => {
            for (at, e) in &failures {
                eprintln!("refused: {at}: {e}");
            }
            eprintln!("refused: {} citation(s) unresolved", failures.len());
            ExitCode::from(1)
        }
    }
}

fn run_replay(path: &Path, host: Option<&Path>, key: Option<&str>) -> ExitCode {
    let resolved;
    let path = if path.is_file() {
        path
    } else {
        let Some(run) = path.to_str().filter(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }) else {
            eprintln!("refused: replay expects a record file or a 64-hex run id");
            return ExitCode::from(1);
        };
        let state = match embed_cmd::state_of_host(host.unwrap_or_else(|| Path::new("."))) {
            Ok(state) => state,
            Err(code) => return code,
        };
        resolved = state.run_dir(run).join("record.json");
        &resolved
    };
    let pinned = match key {
        None => None,
        Some(k) => match hex::decode(k)
            .ok()
            .and_then(|v| <[u8; 32]>::try_from(v).ok())
        {
            Some(b) => Some(holden_seal::EvaluatorPublicKey(b)),
            None => {
                eprintln!("refused: --evaluator-key must be 64 hex characters");
                return ExitCode::from(1);
            }
        },
    };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("refused: cannot read {}: {e}", path.display());
            return ExitCode::from(1);
        }
    };
    let record = match holden_eval::parse_record(&text) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    match holden_eval::replay(&record, pinned.as_ref()) {
        Ok(decided) => {
            let decision = match decided.decision {
                holden_eval::Decision::Pass => "pass",
                holden_eval::Decision::Fail => "fail",
            };
            println!("replayed: decision {decision} (matches the record)");
            println!("evaluated_commit: {}", record.body.evaluated_commit);
            println!("plan_commit: {}", record.body.plan_commit);
            println!("observations: {}", record.body.observations.len());
            println!("record_sha256: {}", record.sha256());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

/// The ledger at `db` (or the private root's), opened only if it exists:
/// a missing ledger is a limitation, never a pass, and is never created.
fn existing_ledger(db: Option<&Path>) -> Result<holden_ledger::Ledger, ExitCode> {
    let Some(path) = db
        .map(Path::to_path_buf)
        .or_else(holden_ledger::store::default_path)
    else {
        println!("limitation ledger_absent: no private root resolvable for the default ledger");
        return Err(ExitCode::from(3));
    };
    if !path.is_file() {
        println!(
            "limitation ledger_absent: {} does not exist; nothing was verified",
            path.display()
        );
        return Err(ExitCode::from(3));
    }
    holden_ledger::Ledger::open(&path).map_err(|e| {
        eprintln!("refused: {e}");
        ExitCode::from(1)
    })
}

/// Print a ledger location as a seat can use it: relative to the private
/// root, including `..` components when an explicit `--db` lives outside it.
fn ledger_display_path(path: &Path) -> PathBuf {
    let Some(root) = holden_embed::private_root() else {
        return path.to_path_buf();
    };
    let Ok(root) = root.canonicalize() else {
        return path.to_path_buf();
    };
    let Ok(path) = path.canonicalize() else {
        return path.to_path_buf();
    };
    let root_parts: Vec<_> = root.components().collect();
    let path_parts: Vec<_> = path.components().collect();
    let common = root_parts
        .iter()
        .zip(&path_parts)
        .take_while(|(left, right)| left == right)
        .count();
    if common == 0 {
        return path.to_path_buf();
    }
    let mut relative = PathBuf::new();
    for _ in common..root_parts.len() {
        relative.push("..");
    }
    for component in &path_parts[common..] {
        relative.push(component.as_os_str());
    }
    relative
}

fn print_ledger_path(ledger: &holden_ledger::Ledger) {
    println!("ledger: {}", ledger_display_path(ledger.path()).display());
}

fn run_ledger_verify(db: Option<&Path>, key: Option<&str>, tip: Option<&str>) -> ExitCode {
    let pinned = match key {
        None => None,
        Some(k) => match hex::decode(k)
            .ok()
            .and_then(|v| <[u8; 32]>::try_from(v).ok())
        {
            Some(b) => Some(holden_seal::EvaluatorPublicKey(b)),
            None => {
                eprintln!("refused: --key must be 64 hex characters");
                return ExitCode::from(1);
            }
        },
    };
    let anchor = match tip {
        None => None,
        Some(t) => match holden_ledger::TipAnchor::parse(t) {
            Some(a) => Some(a),
            None => {
                eprintln!("refused: --tip must be <seq>:<64 hex>");
                return ExitCode::from(1);
            }
        },
    };
    let ledger = match existing_ledger(db) {
        Ok(l) => l,
        Err(code) => return code,
    };
    let report = match ledger.verify(pinned.as_ref(), anchor.as_ref()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("refused: {e}");
            return ExitCode::from(1);
        }
    };
    print_ledger_path(&ledger);
    println!("records: {}", report.records);
    println!("tip: {}", report.tip.as_deref().unwrap_or("none"));
    println!(
        "signer: {} ({})",
        report.signer.as_deref().unwrap_or("none"),
        if report.signer_pinned {
            "pinned"
        } else {
            "the first record's; pin it with --key"
        }
    );
    if report.holds() {
        println!("verified: every record holds");
        ExitCode::SUCCESS
    } else {
        for f in &report.findings {
            eprintln!("refused: {f}");
        }
        eprintln!("refused: {} finding(s)", report.findings.len());
        ExitCode::from(1)
    }
}

fn run_ledger_telemetry(db: Option<&Path>) -> ExitCode {
    let ledger = match existing_ledger(db) {
        Ok(l) => l,
        Err(code) => return code,
    };
    let report = ledger.entries().and_then(|entries| {
        holden_ledger::telemetry::report(&entries, &holden_ledger::Bars::DEFAULT)
    });
    match report.map(|r| serde_json::to_string_pretty(&r)) {
        Ok(Ok(text)) => {
            print_ledger_path(&ledger);
            println!("{text}");
            ExitCode::SUCCESS
        }
        Ok(Err(e)) => {
            eprintln!("refused: {e}");
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("refused: {e}");
            ExitCode::from(1)
        }
    }
}

/// The one block in the global usage belonging to a real command. Keeping
/// the text here, rather than maintaining a second manual, makes focused
/// help and global help incapable of drifting apart.
fn command_usage(command: &[String]) -> Option<String> {
    let name = match command {
        [group, cmd] if group == "canon" && cmd == "verify" => "holden canon verify",
        [group, cmd] if group == "canon" && cmd == "import-lens" => "holden canon import-lens",
        [group, cmd] if group == "canon" && cmd == "doctrine" => "holden canon doctrine",
        [group, cmd] if group == "ledger" && cmd == "verify" => "holden ledger verify",
        [group, cmd] if group == "ledger" && cmd == "telemetry" => "holden ledger telemetry",
        [cmd] if cmd == "replay" => "holden replay",
        [cmd] if cmd == "init" => "holden init",
        [cmd] if cmd == "mcp" => "holden mcp",
        [cmd] if cmd == "embed" => "holden embed",
        [cmd] if cmd == "approve" => "holden approve",
        [cmd] if cmd == "migrate-owner-key" => "holden migrate-owner-key",
        [group, cmd] if group == "agent" && cmd == "serve" => "holden agent serve",
        [cmd] if cmd == "jit" => "holden jit",
        [cmd] if cmd == "trace" => "holden trace",
        [cmd] if cmd == "connect" => "holden connect",
        [cmd] if cmd == "demo-host" => "holden demo-host",
        _ => return None,
    };
    let start = USAGE.find(&format!("  {name} "))?;
    let block = &USAGE[start..];
    let end = block
        .find("\n  holden ")
        .or_else(|| block.find("\n\nexit codes:"))
        .unwrap_or(block.len());
    Some(format!("usage:\n{}\n", block[..end].trim_end()))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    if args == ["--help"] || args == ["help"] {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if args.last().is_some_and(|argument| argument == "--help")
        && let Some(usage) = command_usage(&args[..args.len() - 1])
    {
        print!("{usage}");
        return ExitCode::SUCCESS;
    }
    match mcp_cmd::dispatch(&args).or_else(|| embed_cmd::dispatch(&args)) {
        Some(Ok(code)) => return code,
        Some(Err(why)) => {
            eprintln!("{why}\n{USAGE}");
            return ExitCode::from(2);
        }
        None => {}
    }
    match parse(&args) {
        Ok(Command::Verify { root, pin }) => run_verify(&root, &pin),
        Ok(Command::ImportLens { from, to }) => run_import(&from, &to),
        Ok(Command::Doctrine { root }) => run_doctrine(&root),
        Ok(Command::Replay { record, host, key }) => {
            run_replay(&record, host.as_deref(), key.as_deref())
        }
        Ok(Command::LedgerVerify { db, key, tip }) => {
            run_ledger_verify(db.as_deref(), key.as_deref(), tip.as_deref())
        }
        Ok(Command::LedgerTelemetry { db }) => run_ledger_telemetry(db.as_deref()),
        Err(Usage(why)) => {
            eprintln!("{why}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::mcp_cmd;

    /// Slice-12 P1c: a real child whose wait cannot finish before the reap
    /// deadline produces the typed deadline instead of waiting for exit.
    /// Fail-without mutant: remove `timeout` from `reap_detached`; the child
    /// exits normally and this test fails instead of seeing `Deadline`.
    #[test]
    fn relay_reap_wait_is_bounded() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut child = tokio::process::Command::new("/bin/sleep")
                .arg("7")
                .kill_on_drop(true)
                .spawn()
                .unwrap();
            let result = mcp_cmd::reap_detached(&mut child).await;
            assert!(matches!(
                result,
                Err(mcp_cmd::RelayError::Deadline("server reap"))
            ));
            child.start_kill().unwrap();
            child.wait().await.unwrap();
        });
    }
}
