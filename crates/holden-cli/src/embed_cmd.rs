//! The slice-7 commands: `embed`, `approve`, `agent serve`, `jit`, `trace`,
//! `connect` and `demo-host`. Each is a thin shell over `holden-embed`.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use holden_embed::embed::{DEFAULT_RUN_WINDOW_MS, DetectionReport, EmbedOptions};
use holden_embed::host::HostConfig;
use holden_embed::state::State;
use holden_embed::{agent, approve, connect, embed, fixture, jit, private_root, trace};

/// `--flag value` options (each at most once), switches, and positionals.
struct Parsed {
    opts: BTreeMap<String, String>,
    multi: BTreeMap<String, Vec<String>>,
    switches: Vec<String>,
    positional: Vec<String>,
}

fn parse(args: &[String], opts: &[&str], switches: &[&str]) -> Result<Parsed, String> {
    let mut p = Parsed {
        opts: BTreeMap::new(),
        multi: BTreeMap::new(),
        switches: Vec::new(),
        positional: Vec::new(),
    };
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if switches.contains(&a.as_str()) {
            p.switches.push(a.clone());
            i += 1;
        } else if opts.contains(&a.as_str()) {
            let v = args
                .get(i + 1)
                .ok_or_else(|| format!("{a} needs a value"))?;
            if a == "--manifest" {
                p.multi.entry(a.clone()).or_default().push(v.clone());
            } else if p.opts.insert(a.clone(), v.clone()).is_some() {
                return Err(format!("{a} given twice"));
            }
            i += 2;
        } else if a.starts_with("--") {
            return Err(format!("unknown argument '{a}'"));
        } else {
            p.positional.push(a.clone());
            i += 1;
        }
    }
    Ok(p)
}

fn refused(e: &dyn std::fmt::Display) -> ExitCode {
    eprintln!("{e}");
    ExitCode::from(1)
}

/// The detection report as the success path prints it (slice-18a): the
/// planning process, source documents, total and per-source counts, then
/// only the first 20 candidates and the private-root-relative full list.
/// The refusal path's printer must produce the same fields in the same
/// order; `refusal_report_matches_success_shape` refuses any drift.
fn no_candidate_reason<'a>(r: &'a DetectionReport, document: &str) -> &'a str {
    if r.process == "charter" && document == "charter/INTENT.md" {
        "the intent card states goals, not numbered proofs; candidates come from DONE proofs and GUARANTEES"
    } else if r.process == "charter" {
        "candidates come only from DONE proof lists and GUARANTEES statements"
    } else {
        "no owner promise candidates were found in this source"
    }
}

#[derive(Clone, Copy)]
struct NumberedAccounting {
    seen: usize,
    taken: usize,
    too_short: usize,
    outside_proof_list: usize,
}

struct NumberedItem {
    line: u64,
    group: usize,
    own_short: bool,
    has_subitem: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceBlock {
    None,
    Proof,
    Title,
}

struct ItemGroup {
    kind: SourceBlock,
    taken: usize,
}

fn numbered_accounting(
    host: &Path,
    r: &DetectionReport,
    document: &str,
) -> Option<NumberedAccounting> {
    if r.process != "charter" || !document.starts_with("charter/DONE/") {
        return None;
    }
    let source = fs::read_to_string(host.join(document)).ok()?;
    let mut fenced = false;
    let visible = source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                fenced = !fenced;
                return None;
            }
            (!fenced).then_some((index, line))
        })
        .collect::<Vec<_>>();
    let has_proof_label = visible.iter().any(|(_, line)| {
        let trimmed = line.trim_start();
        let level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        trimmed.starts_with("Proofs")
            || (level > 1
                && level <= 6
                && trimmed.as_bytes().get(level) == Some(&b' ')
                && trimmed[level + 1..].to_ascii_lowercase().contains("proof"))
    });
    let candidate_lines = r
        .candidates
        .iter()
        .filter(|candidate| candidate.source == document)
        .map(|candidate| candidate.line)
        .collect::<std::collections::BTreeSet<_>>();
    let mut items = Vec::<NumberedItem>::new();
    let mut groups = Vec::<ItemGroup>::new();
    let mut kind = SourceBlock::None;
    let mut proof_seen_item = false;
    let mut in_list = false;
    let mut current_group = None;
    for (index, line) in visible.iter().copied() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        let heading = (1..=6).contains(&level) && trimmed.as_bytes().get(level) == Some(&b' ');
        if heading {
            in_list = false;
            current_group = None;
            if level <= 2 {
                proof_seen_item = false;
            }
            kind = if level > 1 && trimmed[level + 1..].to_ascii_lowercase().contains("proof") {
                SourceBlock::Proof
            } else if level == 1 && !has_proof_label {
                SourceBlock::Title
            } else if level <= 2 {
                SourceBlock::None
            } else {
                kind
            };
            continue;
        }
        if trimmed.starts_with("Proofs") {
            kind = SourceBlock::Proof;
            proof_seen_item = false;
            in_list = false;
            current_group = None;
            continue;
        }
        if indent == 0
            && matches!(
                trimmed.trim_end_matches(':'),
                "Gates" | "Steps" | "Out of scope" | "Schedule" | "Landing"
            )
        {
            kind = SourceBlock::None;
            proof_seen_item = false;
            in_list = false;
            current_group = None;
            continue;
        }
        let numbered = indent == 0
            && trimmed.split_once(". ").is_some_and(|(number, _)| {
                let digits = number.bytes().take_while(u8::is_ascii_digit).count();
                digits > 0
                    && (number.len() == digits
                        || (number.len() == digits + 1
                            && number.as_bytes()[digits].is_ascii_lowercase()))
            });
        if numbered {
            if kind == SourceBlock::Proof {
                proof_seen_item = true;
            }
            if !in_list {
                groups.push(ItemGroup { kind, taken: 0 });
                current_group = Some(groups.len() - 1);
            }
            let group = current_group?;
            let body = trimmed.split_once(". ")?.1;
            items.push(NumberedItem {
                line: (index + 1) as u64,
                group,
                own_short: body.chars().count() <= 40,
                has_subitem: false,
            });
            in_list = true;
            continue;
        }
        if indent > 0 && (trimmed.starts_with("- ") || trimmed.starts_with("* ")) && in_list {
            if let Some(item) = items.last_mut() {
                item.has_subitem = true;
            }
        } else if indent == 0 && !trimmed.is_empty() {
            in_list = false;
            current_group = None;
            if kind == SourceBlock::Proof && proof_seen_item {
                kind = SourceBlock::None;
                proof_seen_item = false;
            }
        }
    }
    for item in &items {
        if candidate_lines.contains(&item.line) {
            groups[item.group].taken += 1;
        }
    }
    let labeled_items = groups.iter().any(|group| group.kind == SourceBlock::Proof);
    let selected = |group: &ItemGroup| {
        group.taken > 0
            || group.kind == SourceBlock::Proof
            || (!labeled_items && group.kind == SourceBlock::Title)
    };
    let taken = items
        .iter()
        .filter(|item| candidate_lines.contains(&item.line))
        .count();
    let too_short = items
        .iter()
        .filter(|item| {
            item.own_short
                && !item.has_subitem
                && selected(&groups[item.group])
                && !candidate_lines.contains(&item.line)
        })
        .count();
    Some(NumberedAccounting {
        seen: items.len(),
        taken,
        too_short,
        outside_proof_list: items.len() - taken - too_short,
    })
}

fn print_detection_report(r: &DetectionReport, host: &Path) {
    println!("detection: planning process {}", r.process);
    for d in &r.documents {
        println!("detection: source document {d}");
        if let Some(counts) = numbered_accounting(host, r, d) {
            println!(
                "detection: source numbered items {d} seen {} taken {} too short {} outside any proof list {}",
                counts.seen, counts.taken, counts.too_short, counts.outside_proof_list
            );
            if counts.seen == 0 {
                println!("detection: no numbered items {d}");
            } else if counts.taken == 0 {
                println!(
                    "detection: numbered items found, none taken {d} {}",
                    counts.seen
                );
            }
        }
        if !r.candidates.iter().any(|candidate| candidate.source == *d) {
            println!(
                "detection: no candidates {d}: {}",
                no_candidate_reason(r, d)
            );
        }
    }
    println!("detection: candidate promises {}", r.candidates.len());
    let mut counts = BTreeMap::<&str, usize>::new();
    for candidate in &r.candidates {
        *counts.entry(&candidate.source).or_default() += 1;
    }
    for (source, count) in counts {
        println!("detection: source candidates {source} {count}");
    }
    for candidate in r.candidates.iter().take(20) {
        println!(
            "detection: candidate promise {}:{}: {}",
            candidate.source, candidate.line, candidate.text
        );
    }
    println!("detection: full candidate list {}", r.full_list);
}

/// The refusal path's printer (slice-18a): the same fields, in the same
/// order, on stderr before the refusal line. Deliberately its own printer,
/// not a wrapper: the shape-parity proof exists because the two can drift.
fn eprint_detection_report(r: &DetectionReport, host: &Path) {
    eprintln!("detection: planning process {}", r.process);
    for d in &r.documents {
        eprintln!("detection: source document {d}");
        if let Some(counts) = numbered_accounting(host, r, d) {
            eprintln!(
                "detection: source numbered items {d} seen {} taken {} too short {} outside any proof list {}",
                counts.seen, counts.taken, counts.too_short, counts.outside_proof_list
            );
            if counts.seen == 0 {
                eprintln!("detection: no numbered items {d}");
            } else if counts.taken == 0 {
                eprintln!(
                    "detection: numbered items found, none taken {d} {}",
                    counts.seen
                );
            }
        }
        if !r.candidates.iter().any(|candidate| candidate.source == *d) {
            eprintln!(
                "detection: no candidates {d}: {}",
                no_candidate_reason(r, d)
            );
        }
    }
    eprintln!("detection: candidate promises {}", r.candidates.len());
    let mut counts = BTreeMap::<&str, usize>::new();
    for candidate in &r.candidates {
        *counts.entry(&candidate.source).or_default() += 1;
    }
    for (source, count) in counts {
        eprintln!("detection: source candidates {source} {count}");
    }
    for candidate in r.candidates.iter().take(20) {
        eprintln!(
            "detection: candidate promise {}:{}: {}",
            candidate.source, candidate.line, candidate.text
        );
    }
    eprintln!("detection: full candidate list {}", r.full_list);
}

/// The active toolchain's `cargo` (not a rustup proxy).
fn active_cargo() -> Option<PathBuf> {
    let out = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .ok()?;
    out.status.success().then(|| {
        PathBuf::from(String::from_utf8_lossy(&out.stdout).trim())
            .join("bin")
            .join("cargo")
    })
}

/// `holden-confine` beside this binary.
fn sibling(name: &str) -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(|d| d.join(name))
}

pub(crate) fn state_of_host(host: &Path) -> Result<State, ExitCode> {
    HostConfig::read(host)
        .and_then(|c| c.state_dir())
        .map_err(|e| refused(&e))
}

fn cmd_embed(args: &[String]) -> Result<ExitCode, String> {
    let p = parse(
        args,
        &[
            "--forge",
            "--state",
            "--denylist",
            "--lens",
            "--cargo",
            "--confine-helper",
            "--owner-store",
            "--planning",
            "--manifest",
        ],
        &[],
    )?;
    let [project] = p.positional.as_slice() else {
        return Err("embed takes exactly one project".to_owned());
    };
    let forge = p
        .opts
        .get("--forge")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOLDEN_FORGE").map(PathBuf::from))
        .ok_or("embed needs --forge <forge binary> (or $HOLDEN_FORGE)")?;
    let root = private_root();
    let in_root = |name: &str| root.as_ref().map(|r| r.join(name));
    let opt = |k: &str| p.opts.get(k).map(PathBuf::from);
    let Some(denylist) = opt("--denylist").or_else(|| in_root("denylist.txt")) else {
        return Err("no --denylist and no private root".to_owned());
    };
    let Some(owner_store) = opt("--owner-store").or_else(|| in_root("owner-keys")) else {
        return Err("no --owner-store and no private root".to_owned());
    };
    let Some(cargo) = opt("--cargo").or_else(active_cargo) else {
        return Err("no --cargo and rustc cannot name its sysroot".to_owned());
    };
    let Some(confine_helper) = opt("--confine-helper").or_else(|| sibling("holden-confine")) else {
        return Err("no --confine-helper".to_owned());
    };
    let options = EmbedOptions {
        host: PathBuf::from(project),
        state: opt("--state"),
        forge,
        denylist,
        lens: opt("--lens"),
        manifests: p
            .multi
            .get("--manifest")
            .map(|values| values.iter().map(PathBuf::from).collect())
            .unwrap_or_default(),
        cargo,
        confine_helper,
        owner_store,
        planning: p
            .opts
            .get("--planning")
            .cloned()
            .unwrap_or_else(|| "PLANNING.md".to_owned()),
        run_window_ms: DEFAULT_RUN_WINDOW_MS,
    };
    Ok(match embed::embed(&options) {
        Ok(r) => {
            print_detection_report(&r.detection, Path::new(project));
            println!(
                "embedded: {} (state {})",
                r.host.display(),
                r.state.display()
            );
            println!("domain: {}", r.config.domain);
            for f in &r.host_files {
                let mark = if r.changed.contains(f) {
                    "wrote"
                } else {
                    "same"
                };
                println!("{mark}: {}", f.strip_prefix(&r.host).unwrap_or(f).display());
            }
            for f in &r.removed {
                println!("removed: {}", f.display());
            }
            if !r.keys_created.is_empty() {
                println!("agent keys created: {}", r.keys_created.join(", "));
            }
            println!(
                "cognitions proposed: {} (the owner approves them with holden approve)",
                r.config.cognitions.len()
            );
            println!(
                "files: {} written, {} unchanged, {} removed",
                r.changed.len(),
                r.host_files.len() - r.changed.len(),
                r.removed.len()
            );
            println!("next steps:");
            println!("  OWNER ONLY: {}", r.owner_command);
            println!("  {}", r.config.agent.process);
            ExitCode::SUCCESS
        }
        Err(f) => {
            if let Some(report) = &f.detection {
                eprint_detection_report(report, Path::new(project));
            }
            refused(&f.error)
        }
    })
}

fn cmd_approve(args: &[String]) -> Result<ExitCode, String> {
    let p = parse(args, &["--host", "--owner-store"], &["--create-owner-key"])?;
    if !p.positional.is_empty() {
        return Err("approve takes no positional argument".to_owned());
    }
    let host = p.opts.get("--host").ok_or("approve needs --host <dir>")?;
    let store = p
        .opts
        .get("--owner-store")
        .map(PathBuf::from)
        .or_else(|| private_root().map(|r| r.join("owner-keys")))
        .ok_or("no --owner-store and no private root")?;
    let create = p.switches.iter().any(|s| s == "--create-owner-key");
    Ok(match approve::approve(Path::new(host), &store, create) {
        Ok(r) => {
            if r.created_owner_key {
                println!("owner key created in the owner's key store");
            }
            println!("owner public key: {}", r.owner_public);
            println!("published: {}", r.published.display());
            for s in &r.signed {
                println!(
                    "approved: \"{}\" cognition {} signature {}",
                    s.promise, s.cognition, s.signature
                );
            }
            println!(
                "approved {} cognition(s); every approval re-admitted against the published owner key",
                r.signed.len()
            );
            ExitCode::SUCCESS
        }
        Err(e) => refused(&e),
    })
}

fn cmd_migrate_owner_key(args: &[String]) -> Result<ExitCode, String> {
    let p = parse(args, &["--owner-store"], &[])?;
    if !p.positional.is_empty() {
        return Err("migrate-owner-key takes no positional argument".to_owned());
    }
    let store = p
        .opts
        .get("--owner-store")
        .map(PathBuf::from)
        .or_else(|| private_root().map(|r| r.join("owner-keys")))
        .ok_or("no --owner-store and no private root")?;
    Ok(match approve::migrate_owner_key(&store) {
        Ok(published) => {
            println!("owner key migrated to the OS key store; no key bytes printed");
            println!(
                "published unchanged owner public key: {}",
                published.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => refused(&e),
    })
}

fn cmd_agent(args: &[String]) -> Result<ExitCode, String> {
    let [sub, rest @ ..] = args else {
        return Err("agent takes serve".to_owned());
    };
    if sub != "serve" {
        return Err("agent takes serve".to_owned());
    }
    let p = parse(rest, &["--state", "--host"], &[])?;
    let state = match (p.opts.get("--state"), p.opts.get("--host")) {
        (Some(s), None) => State::new(Path::new(s)),
        (None, Some(h)) => match state_of_host(Path::new(h)) {
            Ok(s) => s,
            Err(code) => return Ok(code),
        },
        _ => return Err("agent serve needs --state <dir> or --host <dir>".to_owned()),
    };
    let served = agent::serve(&state, |s| {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(
            out,
            "holden agent: evaluator socket {}",
            s.evaluator_socket.display()
        );
        let _ = writeln!(
            out,
            "holden agent: implementer socket {}",
            s.implementer_socket.display()
        );
        let _ = writeln!(
            out,
            "holden agent: constitution sha256 {}",
            s.constitution_sha256
        );
        for (id, why) in &s.refused {
            let _ = writeln!(out, "holden agent: cognition {id} refused: {why}");
        }
        let _ = writeln!(out, "holden agent: ready: {} plan(s) armed", s.armed.len());
        let _ = out.flush();
    });
    Ok(match served {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => refused(&e),
    })
}

fn cmd_jit(args: &[String]) -> Result<ExitCode, String> {
    let p = parse(args, &["--host"], &[])?;
    // The hook passes git's arguments through; post-commit has none.
    let host = p
        .opts
        .get("--host")
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    Ok(match jit::jit(&host) {
        Ok(r) => {
            for line in jit::diagnostic_lines(&r) {
                println!("{line}");
            }
            if r.refused.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            println!("holden: {e}");
            ExitCode::from(1)
        }
    })
}

fn cmd_trace(args: &[String]) -> Result<ExitCode, String> {
    let p = parse(args, &["--host"], &[])?;
    let [run] = p.positional.as_slice() else {
        return Err("trace takes exactly one run id".to_owned());
    };
    let host = p
        .opts
        .get("--host")
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let state = match state_of_host(&host) {
        Ok(s) => s,
        Err(code) => return Ok(code),
    };
    Ok(match trace::trace(&state, run) {
        Ok(lines) => {
            for l in lines {
                println!("{l}");
            }
            println!("verified: every link of the chain holds");
            ExitCode::SUCCESS
        }
        Err(e) => refused(&e),
    })
}

fn cmd_connect(args: &[String]) -> Result<ExitCode, String> {
    let p = parse(args, &["--role", "--socket"], &[])?;
    let role = p.opts.get("--role").ok_or("connect needs --role")?;
    if role != "implementer" && role != "evaluator" {
        return Err("--role is implementer or evaluator".to_owned());
    }
    let socket = match p.opts.get("--socket") {
        Some(socket) => PathBuf::from(socket),
        None => match state_of_host(Path::new(".")) {
            Ok(state) => state.socket(if role == "evaluator" {
                holden_mcp::Role::Evaluator
            } else {
                holden_mcp::Role::Implementer
            }),
            Err(code) => return Ok(code),
        },
    };
    Ok(match connect::bridge(&socket) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("refused: {}: {e}", socket.display());
            ExitCode::from(1)
        }
    })
}

fn cmd_demo_host(args: &[String]) -> Result<ExitCode, String> {
    let p = parse(args, &["--workspace"], &[])?;
    let [dest] = p.positional.as_slice() else {
        return Err("demo-host takes exactly one destination".to_owned());
    };
    let workspace = p
        .opts
        .get("--workspace")
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    Ok(match fixture::create_host(&workspace, Path::new(dest)) {
        Ok(commit) => {
            println!("created: {dest} at commit {commit}");
            ExitCode::SUCCESS
        }
        Err(e) => refused(&e),
    })
}

/// Run a slice-7 command, if `args` names one. `Err` is a usage error.
pub fn dispatch(args: &[String]) -> Option<Result<ExitCode, String>> {
    let (cmd, rest) = args.split_first()?;
    Some(match cmd.as_str() {
        "embed" => cmd_embed(rest),
        "approve" => cmd_approve(rest),
        "migrate-owner-key" => cmd_migrate_owner_key(rest),
        "agent" => cmd_agent(rest),
        "jit" => cmd_jit(rest),
        "trace" => cmd_trace(rest),
        "connect" => cmd_connect(rest),
        "demo-host" => cmd_demo_host(rest),
        _ => return None,
    })
}
