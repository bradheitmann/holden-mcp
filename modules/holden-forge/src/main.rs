//! `forge`: validate a domain pack, emit agent constitutions, and export
//! this module as a standalone directory.
//!
//! ```text
//! forge pack [--pack <file>]
//! forge constitution --role planner|sealer|evaluator|adversary [--pack <file>]
//! forge export <dir> [--denylist <file>]
//! forge verify [<dir>]
//! forge sums [<dir>]
//! ```
//!
//! The denylist comes from `--denylist`, else `$FORGE_DENYLIST`.
//! Exit codes: 0 pass, 1 refused, 2 usage, 3 limitation (never a pass).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use holden_forge::constitution;
use holden_forge::export::{self, Denylist, ExportError};
use holden_forge::pack::Role;
use holden_forge::{DEFAULT_PACK, load_pack, module_root};

const USAGE: &str = "usage:
  forge pack [--pack <file>]
  forge constitution --role planner|sealer|evaluator|adversary [--pack <file>]
  forge export <dir> [--denylist <file>]   (else $FORGE_DENYLIST)
  forge verify [<dir>]
  forge sums [<dir>]";

fn usage(why: &str) -> ExitCode {
    eprintln!("forge: {why}\n{USAGE}");
    ExitCode::from(2)
}

fn refused(what: &str, findings: &[String]) -> ExitCode {
    eprintln!("forge: {what}: refused ({} finding(s))", findings.len());
    for f in findings {
        eprintln!("forge:   {f}");
    }
    ExitCode::from(1)
}

/// `--key value` pairs, then positional arguments.
type Parsed = (Vec<(String, String)>, Vec<String>);

/// Parse `--key value` options; anything else is positional.
fn options(args: &[String], keys: &[&str]) -> Result<Parsed, String> {
    let mut opts = Vec::new();
    let mut pos = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if let Some(k) = a.strip_prefix("--") {
            if !keys.contains(&k) {
                return Err(format!("unknown option '{a}'"));
            }
            let Some(v) = args.get(i + 1) else {
                return Err(format!("{a} needs a value"));
            };
            opts.push((k.to_owned(), v.clone()));
            i += 2;
        } else {
            pos.push(a.clone());
            i += 1;
        }
    }
    Ok((opts, pos))
}

fn opt<'a>(opts: &'a [(String, String)], key: &str) -> Option<&'a str> {
    opts.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn pack_path(opts: &[(String, String)]) -> PathBuf {
    opt(opts, "pack").map_or_else(|| module_root().join(DEFAULT_PACK), PathBuf::from)
}

fn cmd_pack(args: &[String]) -> ExitCode {
    let (opts, pos) = match options(args, &["pack"]) {
        Ok(x) => x,
        Err(e) => return usage(&e),
    };
    if !pos.is_empty() {
        return usage("pack takes no positional argument");
    }
    let path = pack_path(&opts);
    match load_pack(&path) {
        Ok(l) => {
            println!(
                "forge: pack {} valid: schema {}, {} super-domains, {} categories, {} items, {} roles; ratification {}; digest {}",
                l.pack.pack_name,
                l.pack.schema,
                l.pack.super_domains.len(),
                l.pack.category_count(),
                l.pack.item_count(),
                l.pack.role_focus.len(),
                l.pack.ratification_status,
                l.digest
            );
            ExitCode::SUCCESS
        }
        Err(errs) => refused("pack", &errs),
    }
}

fn cmd_constitution(args: &[String]) -> ExitCode {
    let (opts, pos) = match options(args, &["role", "pack"]) {
        Ok(x) => x,
        Err(e) => return usage(&e),
    };
    if !pos.is_empty() {
        return usage("constitution takes no positional argument");
    }
    let Some(role) = opt(&opts, "role").and_then(Role::parse) else {
        return usage("--role must be planner, sealer, evaluator or adversary");
    };
    let loaded = match load_pack(&pack_path(&opts)) {
        Ok(l) => l,
        Err(errs) => return refused("pack", &errs),
    };
    match constitution::derive(&loaded, role) {
        Ok(c) => {
            print!("{}", constitution::render(&c));
            ExitCode::SUCCESS
        }
        Err(e) => refused("constitution", &[e]),
    }
}

fn cmd_export(args: &[String]) -> ExitCode {
    let (opts, pos) = match options(args, &["denylist"]) {
        Ok(x) => x,
        Err(e) => return usage(&e),
    };
    let [dest] = pos.as_slice() else {
        return usage("export takes exactly one directory");
    };
    let deny_path = opt(&opts, "denylist").map(PathBuf::from).or_else(|| {
        std::env::var_os("FORGE_DENYLIST")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    });
    let Some(deny_path) = deny_path else {
        eprintln!(
            "forge: export: LIMITATION denylist_absent: give --denylist <file> or $FORGE_DENYLIST; an export without the private-identifier scan is never a pass"
        );
        return ExitCode::from(3);
    };
    let Ok(text) = std::fs::read_to_string(&deny_path) else {
        eprintln!("forge: export: LIMITATION denylist_absent: the denylist file cannot be read");
        return ExitCode::from(3);
    };
    let denylist = Denylist::parse(&text);
    match export::export(&module_root(), Path::new(dest), &denylist) {
        Ok(r) => {
            println!(
                "forge: exported {} files to {} and regenerated {}; verified",
                r.files,
                r.dest.display(),
                export::SUMS
            );
            ExitCode::SUCCESS
        }
        Err(ExportError::Refused(f)) => refused("export", &f),
        Err(e) => {
            eprintln!("forge: export: refused: {e}");
            ExitCode::from(1)
        }
    }
}

fn dir_arg(args: &[String]) -> Result<PathBuf, String> {
    match args {
        [] => Ok(module_root()),
        [d] => Ok(PathBuf::from(d)),
        _ => Err("expected at most one directory".into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = args.get(1..).unwrap_or_default();
    match args.first().map(String::as_str) {
        Some("pack") => cmd_pack(rest),
        Some("constitution") => cmd_constitution(rest),
        Some("export") => cmd_export(rest),
        Some("verify") => {
            let dir = match dir_arg(rest) {
                Ok(d) => d,
                Err(e) => return usage(&e),
            };
            let mut findings = export::check_license(&dir);
            match export::verify_sums(&dir) {
                Ok(n) if findings.is_empty() => {
                    println!(
                        "forge: {} verified: {n} files match {}; LICENSE and NOTICE are the pinned texts",
                        dir.display(),
                        export::SUMS
                    );
                    return ExitCode::SUCCESS;
                }
                Ok(_) => {}
                Err(f) => findings.extend(f),
            }
            refused("verify", &findings)
        }
        Some("sums") => {
            let dir = match dir_arg(rest) {
                Ok(d) => d,
                Err(e) => return usage(&e),
            };
            match export::write_sums(&dir) {
                Ok(n) => {
                    println!("forge: wrote {} covering {n} files", export::SUMS);
                    ExitCode::SUCCESS
                }
                Err(e) => refused("sums", &[e.to_string()]),
            }
        }
        _ => usage("unknown command"),
    }
}
