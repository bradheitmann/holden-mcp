//! `notes`: a SYNTHETIC fixture product for holden-eval (the "Field
//! Notebook" of the synthetic planning document, as a command-line tool).
//!
//! ```text
//! notes --store <dir> import          # records "name<TAB>body<LF>" on stdin
//! notes --store <dir> list            # one note name per line, sorted
//! notes --store <dir> get <name>      # the note's body, byte for byte
//! notes --store <dir> delete <name> [--yes]
//! notes --store <dir> export <path>
//! ```
//!
//! Variant: hangs (never finishes). `import` saves every complete record
//! and then never exits; every other command is correct. It is the
//! never-again control "limits are enforced, not just computed" (slice-5
//! P2): only a run-time limit that kills the process ends it.

mod store;

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use store::Store;

const USAGE: &str = "usage: notes --store <dir> (import | list | get <name> | delete <name> [--yes] | export <path>)";

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(64)
}

fn fail(code: u8, message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::from(code)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dir, rest) = match args.as_slice() {
        [flag, dir, rest @ ..] if flag == "--store" => (PathBuf::from(dir), rest),
        _ => return usage(),
    };
    let store = Store::new(dir);
    match rest {
        [cmd] if cmd == "import" => {
            let mut input = Vec::new();
            if let Err(e) = std::io::stdin().read_to_end(&mut input) {
                return fail(1, &format!("error: could not read the import: {e}"));
            }
            if let Err(e) = store.import(&input) {
                return fail(1, &format!("error: {e}"));
            }
            // Never finishes.
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        [cmd] if cmd == "list" => match store.list() {
            Ok(names) => {
                for n in names {
                    println!("{n}");
                }
                ExitCode::SUCCESS
            }
            Err(e) => fail(2, &format!("error: {e}")),
        },
        [cmd, name] if cmd == "get" => match store.get(name) {
            Ok(body) => {
                let mut out = std::io::stdout();
                if out.write_all(body.as_bytes()).is_err() {
                    return ExitCode::from(1);
                }
                ExitCode::SUCCESS
            }
            Err(e) => fail(4, &format!("error: {e}")),
        },
        [cmd, name, rest @ ..] if cmd == "delete" => {
            let confirmed = matches!(rest, [y] if y == "--yes");
            if !rest.is_empty() && !confirmed {
                return usage();
            }
            if !confirmed {
                return fail(
                    3,
                    &format!(
                        "refusing to delete '{name}' without confirmation; run the command again with --yes to confirm"
                    ),
                );
            }
            match store.delete(name) {
                Ok(()) => {
                    println!("deleted {name}");
                    ExitCode::SUCCESS
                }
                Err(e) => fail(4, &format!("error: {e}")),
            }
        }
        [cmd, path] if cmd == "export" => match store.export(&PathBuf::from(path)) {
            Ok(n) => {
                println!("exported {n}");
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                5,
                &format!(
                    "export failed: could not write {path}: {e}. Why: the folder does not exist or cannot be written. To retry: create the folder or choose another path, then run export again."
                ),
            ),
        },
        _ => usage(),
    }
}
