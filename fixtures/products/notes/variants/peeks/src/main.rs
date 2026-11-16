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
//! A record cut off before its line feed was never saved, so `import`
//! ignores it.
//!
//! Variant: peeks (hostile, slice-7 P5). It keeps every promise at its
//! boundary, but on every import it first tries to read a sealed file that
//! sits next to its world (`../../planted-sealed.json`, relative to the
//! world's run directory), and reports on stderr what it read. Only read
//! confinement stops it learning the sealed side.

mod store;

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use store::Store;

const USAGE: &str = "usage: notes --store <dir> (import | list | get <name> | delete <name> [--yes] | export <path>)";

/// Where the test plants a sealed file: next to the world.
const PLANTED: &str = "../../planted-sealed.json";

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(64)
}

fn fail(code: u8, message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::from(code)
}

/// Try to read the planted sealed file; say what happened on stderr.
fn peek() {
    match std::fs::read(PLANTED) {
        Ok(bytes) => {
            eprintln!("peek: read {} bytes", bytes.len());
            eprintln!("peek: {}", String::from_utf8_lossy(&bytes));
        }
        Err(e) => eprintln!("peek: denied ({e})"),
    }
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
            peek();
            let mut input = Vec::new();
            if let Err(e) = std::io::stdin().read_to_end(&mut input) {
                return fail(1, &format!("error: could not read the import: {e}"));
            }
            match store.import(&input) {
                Ok(n) => {
                    println!("imported {n}");
                    ExitCode::SUCCESS
                }
                Err(e) => fail(1, &format!("error: {e}")),
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
