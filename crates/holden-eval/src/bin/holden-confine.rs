//! `holden-confine`: the confinement launcher (slice-4 P4).
//!
//! ```text
//! holden-confine --world <dir> --report <file> --profile build|product --cpu <s> --fsize <bytes> --nofile <n> [--read <dir>]... -- <program> [args...]
//! ```
//!
//! It sets `RLIMIT_CPU`, `RLIMIT_FSIZE`, `RLIMIT_NOFILE` and `RLIMIT_CORE`
//! (0), applies Landlock on Linux (on macOS the caller has already placed it
//! under `sandbox-exec` with the profile's SBPL), checks that it cannot write
//! next to the world, that it cannot list the world's parent directory
//! (slice-7: reads are confined) and, on macOS, that it cannot bind a
//! socket, writes its report with the profile's typed limitations, then
//! replaces itself with the program (no shell is involved) with the
//! environment it was given. It never adds a variable.
//!
//! On Linux, Landlock grants reads only below the world, the system paths a
//! process needs (and, for builds, `/proc`, `/sys` and each `--read`
//! directory: the toolchain). A failed read self-check is recorded as the
//! typed limitation `reads_not_confined`, never a silent pass.
//!
//! Exit codes (only when it does not replace itself): 125 refused, 126 the
//! program could not be executed, 64 usage.

use std::net::TcpListener;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use holden_eval::confine::{
    Backend, HelperReport, Limitation, Profile, Rlimits, SELF_CHECK_PORT, static_limitations,
};
use rustix::process::{Resource, Rlimit, getrlimit, setrlimit};

struct Args {
    world: PathBuf,
    report: PathBuf,
    profile: Profile,
    rlimits: Rlimits,
    reads: Vec<PathBuf>,
    program: PathBuf,
    rest: Vec<String>,
}

fn parse() -> Option<Args> {
    let mut it = std::env::args().skip(1);
    let (mut world, mut report, mut cpu, mut fsize, mut nofile) = (None, None, None, None, None);
    let mut profile = None;
    let mut reads = Vec::new();
    loop {
        match it.next()?.as_str() {
            "--world" => world = Some(PathBuf::from(it.next()?)),
            "--report" => report = Some(PathBuf::from(it.next()?)),
            "--profile" => profile = Profile::parse(&it.next()?),
            "--cpu" => cpu = it.next()?.parse::<u64>().ok(),
            "--fsize" => fsize = it.next()?.parse::<u64>().ok(),
            "--nofile" => nofile = it.next()?.parse::<u64>().ok(),
            "--read" => reads.push(PathBuf::from(it.next()?)),
            "--" => break,
            _ => return None,
        }
    }
    let program = PathBuf::from(it.next()?);
    Some(Args {
        world: world?,
        report: report?,
        profile: profile?,
        rlimits: Rlimits {
            cpu_secs: cpu?,
            file_bytes: fsize?,
            open_files: nofile?,
            core_bytes: 0,
        },
        reads,
        program,
        rest: it.collect(),
    })
}

fn limit(resource: Resource, value: u64) -> Result<(), String> {
    let hard = getrlimit(resource).maximum;
    let v = hard.map_or(value, |h| h.min(value));
    setrlimit(
        resource,
        Rlimit {
            current: Some(v),
            maximum: Some(v),
        },
    )
    .map_err(|e| format!("setrlimit {resource:?} failed: {e}"))
}

/// The system paths every confined Linux process may read (slice-7 P5).
#[cfg(target_os = "linux")]
const LINUX_SYSTEM_READS: [&str; 7] = ["/usr", "/lib", "/lib64", "/lib32", "/bin", "/etc", "/dev"];

/// The further system paths a Linux build may read: the toolchain reads its
/// own process information and the CPU topology.
#[cfg(target_os = "linux")]
const LINUX_BUILD_READS: [&str; 3] = ["/proc", "/sys", "/sbin"];

#[cfg(target_os = "linux")]
fn landlock(
    world: &Path,
    profile: Profile,
    extra_reads: &[PathBuf],
) -> Result<(bool, Vec<Limitation>), String> {
    use landlock::{
        ABI, Access, AccessFs, AccessNet, LandlockStatus, Ruleset, RulesetAttr, RulesetCreatedAttr,
        RulesetStatus, path_beneath_rules,
    };
    let abi = ABI::V5;
    // Only existing paths become rules (a missing system path grants
    // nothing).
    let mut reads: Vec<PathBuf> = LINUX_SYSTEM_READS.iter().map(PathBuf::from).collect();
    if profile == Profile::Build {
        reads.extend(LINUX_BUILD_READS.iter().map(PathBuf::from));
        reads.extend(extra_reads.iter().cloned());
    }
    reads.retain(|p| p.exists());
    let status = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .map_err(|e| e.to_string())?
        .handle_access(AccessNet::from_all(abi))
        .map_err(|e| e.to_string())?
        .create()
        .map_err(|e| e.to_string())?
        .add_rules(path_beneath_rules([world], AccessFs::from_all(abi)))
        .map_err(|e| e.to_string())?
        .add_rules(path_beneath_rules(
            [Path::new("/dev/null")],
            AccessFs::from_read(abi) | AccessFs::from_write(abi),
        ))
        .map_err(|e| e.to_string())?
        .add_rules(path_beneath_rules(&reads, AccessFs::from_read(abi)))
        .map_err(|e| e.to_string())?
        .restrict_self()
        .map_err(|e| e.to_string())?;
    if status.ruleset == RulesetStatus::NotEnforced {
        return Err(format!(
            "Landlock is not enforced on this kernel ({:?})",
            status.landlock
        ));
    }
    // UDP, reads and unobserved attempts come from `static_limitations`.
    let mut limitations = Vec::new();
    let network = match status.landlock {
        LandlockStatus::Available { effective_abi, .. } if effective_abi >= ABI::V4 => true,
        other => {
            limitations.push(Limitation::NetworkNotDenied {
                detail: format!("Landlock status {other:?}; TCP needs ABI 4 or later"),
            });
            false
        }
    };
    Ok((network, limitations))
}

#[cfg(not(target_os = "linux"))]
#[allow(clippy::unnecessary_wraps)]
fn landlock(
    _world: &Path,
    _profile: Profile,
    _extra_reads: &[PathBuf],
) -> Result<(bool, Vec<Limitation>), String> {
    // macOS: the caller placed this process under sandbox-exec, whose
    // profile denies all network and reads outside the world; the
    // self-checks below confirm it.
    Ok((true, Vec::new()))
}

/// The launcher must not be able to list the world's parent directory
/// (slice-7 P5). Only a permission refusal proves the deny.
fn reads_confined(world: &Path) -> bool {
    let Some(parent) = world.parent() else {
        return false;
    };
    matches!(
        std::fs::read_dir(parent),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied
    )
}

/// The launcher must not be able to create a file next to the world.
fn writes_confined(world: &Path) -> bool {
    let Some(parent) = world.parent() else {
        return false;
    };
    let probe = parent.join(format!(".holden-confine-probe-{}", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            false
        }
        Err(_) => true,
    }
}

fn write_report(path: &Path, report: &HelperReport) -> bool {
    serde_json::to_vec(report)
        .ok()
        .is_some_and(|b| std::fs::write(path, b).is_ok())
}

fn main() -> ExitCode {
    let Some(args) = parse() else {
        eprintln!(
            "usage: holden-confine --world <dir> --report <file> --profile build|product --cpu <s> --fsize <bytes> --nofile <n> -- <program> [args...]"
        );
        return ExitCode::from(64);
    };
    let backend = Backend::current();
    let mut report = HelperReport {
        backend,
        profile: args.profile,
        // Slice-7: on macOS both profiles kill the offending process.
        kills_on_violation: backend == Backend::MacosSandboxExec,
        writes_confined: false,
        reads_confined: false,
        network_denied: false,
        rlimits: args.rlimits,
        limitations: Vec::new(),
        refused: None,
    };
    let refuse = |mut report: HelperReport, why: String| {
        eprintln!("holden-confine: refused: {why}");
        report.refused = Some(why);
        let _ = write_report(&args.report, &report);
        ExitCode::from(125)
    };
    for (resource, value) in [
        (Resource::Cpu, args.rlimits.cpu_secs),
        (Resource::Fsize, args.rlimits.file_bytes),
        (Resource::Nofile, args.rlimits.open_files),
        (Resource::Core, 0),
    ] {
        if let Err(why) = limit(resource, value) {
            return refuse(report, why);
        }
    }
    let (network, dynamic) = match landlock(&args.world, args.profile, &args.reads) {
        Ok(v) => v,
        Err(why) => return refuse(report, why),
    };
    let mut limitations = static_limitations(backend, args.profile);
    limitations.extend(dynamic);
    report.reads_confined = reads_confined(&args.world);
    if !report.reads_confined {
        limitations.push(Limitation::ReadsNotConfined);
    }
    limitations.sort();
    limitations.dedup();
    report.limitations = limitations;
    report.writes_confined = writes_confined(&args.world);
    if !report.writes_confined {
        return refuse(
            report,
            "a file could be created outside the world; the process is not confined".to_owned(),
        );
    }
    report.network_denied = network;
    if cfg!(target_os = "macos") {
        // Only a permission refusal proves the deny; a port already in use
        // proves nothing.
        match TcpListener::bind(("127.0.0.1", SELF_CHECK_PORT)) {
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {}
            Ok(_) => {
                report.network_denied = false;
                return refuse(
                    report,
                    "a socket could be bound; the network is not denied".to_owned(),
                );
            }
            Err(e) => {
                report.network_denied = false;
                return refuse(
                    report,
                    format!("the network self-check was inconclusive: {e}"),
                );
            }
        }
    }
    if !write_report(&args.report, &report) {
        return refuse(report, "the report could not be written".to_owned());
    }
    let err = Command::new(&args.program).args(&args.rest).exec();
    eprintln!("holden-confine: could not run the program: {err}");
    ExitCode::from(126)
}
