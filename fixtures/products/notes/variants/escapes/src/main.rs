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
//! Variant: escapes (hostile). Besides the commands above it tries to leave
//! its world and to exhaust its limits, and reports what happened:
//!
//! ```text
//! notes escape <path> <host:port>   # write outside, then open a TCP socket
//! notes env [--build]               # its own (or its build script's) environment
//! notes flood                       # write to stdout without end
//! notes hang                        # never exit
//! notes bigfile <path>              # grow one file without end
//! ```

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

fn report(what: &str, r: std::io::Result<()>) {
    match r {
        Ok(()) => println!("{what}: ok"),
        Err(e) => println!("{what}: denied ({e})"),
    }
}

/// On every import, try to leave the world: write two levels above the
/// working directory, and open a TCP connection. Report on stderr.
fn side_doors() {
    match std::fs::write("../../escaped-by-import.txt", b"escaped\n") {
        Ok(()) => eprintln!("escape-write: ok"),
        Err(e) => eprintln!("escape-write: denied ({e})"),
    }
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 9));
    match std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(1)) {
        Ok(_) => eprintln!("escape-connect: ok"),
        Err(e) => eprintln!("escape-connect: denied ({e})"),
    }
}

fn hostile(args: &[String]) -> Option<ExitCode> {
    match args {
        [cmd, path, addr] if cmd == "escape" => {
            report("write", std::fs::write(path, b"escaped\n"));
            let connected = addr
                .parse::<std::net::SocketAddr>()
                .map_err(std::io::Error::other)
                .and_then(|a| {
                    std::net::TcpStream::connect_timeout(&a, std::time::Duration::from_secs(2))
                })
                .and_then(|mut s| s.write_all(b"escaped\n"));
            report("connect", connected);
            Some(ExitCode::SUCCESS)
        }
        [cmd] if cmd == "env" => {
            let mut vars: Vec<(String, String)> = std::env::vars().collect();
            vars.sort();
            for (k, v) in vars {
                println!("{k}={v}");
            }
            Some(ExitCode::SUCCESS)
        }
        [cmd, flag] if cmd == "env" && flag == "--build" => {
            print!(
                "{}",
                include_str!(concat!(env!("OUT_DIR"), "/build-env.txt"))
            );
            Some(ExitCode::SUCCESS)
        }
        [cmd] if cmd == "flood" => {
            let chunk = [b'y'; 65536];
            let mut out = std::io::stdout();
            while out.write_all(&chunk).is_ok() {}
            Some(ExitCode::SUCCESS)
        }
        [cmd] if cmd == "hang" => loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        },
        [cmd, path] if cmd == "bigfile" => {
            let chunk = vec![b'z'; 1 << 20];
            let mut written: u64 = 0;
            let outcome = std::fs::File::create(path).and_then(|mut f| {
                for _ in 0..1024 {
                    f.write_all(&chunk)?;
                    written += chunk.len() as u64;
                }
                Ok(())
            });
            println!("bigfile: wrote {written} bytes");
            report("bigfile", outcome);
            Some(ExitCode::SUCCESS)
        }
        _ => None,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = hostile(&args) {
        return code;
    }
    let (dir, rest) = match args.as_slice() {
        [flag, dir, rest @ ..] if flag == "--store" => (PathBuf::from(dir), rest),
        _ => return usage(),
    };
    let store = Store::new(dir);
    match rest {
        [cmd] if cmd == "import" => {
            side_doors();
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
