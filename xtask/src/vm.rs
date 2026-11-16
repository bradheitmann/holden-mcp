//! `cargo xtask gate <slice> --linux-vm`: run the gate on the exact HEAD
//! commit inside the Tart Linux VM (no containers).
//!
//! Configuration (all optional):
//! - `HOLDEN_VM_ADDR`: the VM's address. When set, the driver uses it as is
//!   and never calls `tart`, so a sandboxed verifier without `tart` can run
//!   the Linux gate;
//! - `HOLDEN_VM` (default `holden-mcp-linux`): the Tart VM name, used only to
//!   ask `tart ip` for the address when `HOLDEN_VM_ADDR` is unset;
//! - `HOLDEN_VM_USER` (default `admin`): the ssh user;
//! - `HOLDEN_VM_SSH_KEY` (default `$HOME/.ssh/id_ed25519_holden_mcp_vm`);
//! - `HOLDEN_TARGET_BASE` (default `/tmp`): the per-run build-directory base.
//!   It is forwarded into the VM run, and the gate is green only when every
//!   build directory in the returned run record sits under it.
//!
//! The driver never starts, stops or reconfigures the VM.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util;

/// Where the VM address came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddrSource {
    /// `HOLDEN_VM_ADDR`; `tart` was not called.
    Override,
    /// `tart ip <name>`.
    Tart,
}

/// An address is a host name or an IPv4/IPv6 literal: `[A-Za-z0-9.:-]`, not
/// starting with `-` (so it can never read as an ssh option).
fn valid_addr(addr: &str) -> bool {
    !addr.is_empty()
        && addr.len() <= 253
        && !addr.starts_with('-')
        && addr
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b':' | b'-'))
}

/// Resolve the VM address. With `override_addr` set, it is validated and
/// returned without running `tart_program`; otherwise `tart_program ip <name>`
/// is asked.
///
/// # Errors
/// A refusal message when the override is malformed or `tart` gives no
/// usable address.
pub fn resolve_addr(
    override_addr: Option<&str>,
    tart_program: &OsStr,
    name: &str,
) -> Result<(String, AddrSource), String> {
    if let Some(addr) = override_addr {
        return if valid_addr(addr) {
            Ok((addr.to_owned(), AddrSource::Override))
        } else {
            Err("HOLDEN_VM_ADDR is not a host name or IP address".to_owned())
        };
    }
    match util::stdout_of(Command::new(tart_program).args(["ip", name])) {
        Some(ip) if valid_addr(&ip) => Ok((ip, AddrSource::Tart)),
        _ => Err(
            "`tart ip` gave no address; is the VM running? (the driver never starts it; \
             set HOLDEN_VM_ADDR to skip tart)"
                .to_owned(),
        ),
    }
}

/// A target base the VM shell can take unquoted: absolute, `[A-Za-z0-9/._-]`.
fn valid_target_base(base: &str) -> bool {
    base.starts_with('/')
        && !base.contains("..")
        && base
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'_' | b'-'))
}

/// True when every step of the run record that names a build directory
/// names one under `base`, and at least one does.
fn record_targets_under(record: &serde_json::Value, base: &str) -> bool {
    let Some(steps) = record["steps"].as_array() else {
        return false;
    };
    let prefix = format!("{}/", base.trim_end_matches('/'));
    let dirs: Vec<&str> = steps
        .iter()
        .filter_map(|s| s["target_dir"].as_str())
        .filter(|d| *d != "-")
        .collect();
    !dirs.is_empty()
        && dirs.iter().all(|d| d.starts_with(&prefix))
        && record["target_base"].as_str() == Some(base)
}

struct Vm {
    name: String,
    user: String,
    key: PathBuf,
    ip: String,
}

impl Vm {
    fn ssh(&self) -> Command {
        let mut c = Command::new("ssh");
        c.args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=15"])
            .args(["-o", "StrictHostKeyChecking=accept-new"])
            .arg("-i")
            .arg(&self.key)
            .arg(format!("{}@{}", self.user, self.ip));
        c
    }

    fn scp(&self) -> Command {
        let mut c = Command::new("scp");
        c.args(["-q", "-o", "BatchMode=yes", "-o", "ConnectTimeout=15"])
            .args(["-o", "StrictHostKeyChecking=accept-new"])
            .arg("-i")
            .arg(&self.key);
        c
    }

    fn remote(&self, path: &str) -> String {
        format!("{}@{}:{path}", self.user, self.ip)
    }
}

fn remote_private_dir(agent_private_root: Option<&str>) -> String {
    agent_private_root.map_or_else(
        || ".local/share/agent-private/holden-mcp".to_owned(),
        |root| format!("{root}/holden-mcp"),
    )
}

/// The remote fresh-clone gate command. `AGENT_PRIVATE_ROOT` is an explicit
/// shell-safe absolute path, forwarded only when the host supplied it.
pub(crate) fn gate_script(
    work: &str,
    sha: &str,
    slice: &str,
    base: &str,
    agent_private_root: Option<&str>,
    record_remote: &str,
) -> String {
    let private_root_export = agent_private_root.map_or_else(String::new, |root| {
        format!("export AGENT_PRIVATE_ROOT={root}; ")
    });
    format!(
        "set -eu; . \"$HOME/.cargo/env\"; cd \"$HOME/{work}\"; rm -rf {sha} {sha}-run.json; \
         git clone --quiet {sha}.bundle {sha}; cd {sha}; git checkout --quiet --detach {sha}; \
         test \"$(git rev-parse HEAD)\" = {sha}; \
         export HOLDEN_TARGET_BASE={base}; {private_root_export}mkdir -p {base}; \
         cargo xtask gate {slice} --host-covered-tools --record \"$HOME/{record_remote}\"",
    )
}

fn fail(msg: &str) -> bool {
    println!("gate --linux-vm: refused: {msg}");
    false
}

fn ok(cmd: &mut Command) -> bool {
    matches!(util::run_inherit(cmd), Ok(Some(0)))
}

/// Run the VM gate for `slice`. Returns true when the VM gate is green.
pub fn run(root: &Path, slice: &str) -> bool {
    let porcelain = util::stdout_of(Command::new("git").arg("-C").arg(root).args([
        "status",
        "--porcelain",
        "--untracked-files=all",
    ]));
    match porcelain {
        Some(s) if s.is_empty() => {}
        Some(_) => return fail("the working tree is not clean; commit first (the VM runs HEAD)"),
        None => return fail("git status could not run"),
    }
    let Some(sha) = util::stdout_of(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["rev-parse", "HEAD"]),
    ) else {
        return fail("git rev-parse HEAD failed");
    };

    let name = env::var("HOLDEN_VM").unwrap_or_else(|_| "holden-mcp-linux".to_owned());
    let user = env::var("HOLDEN_VM_USER").unwrap_or_else(|_| "admin".to_owned());
    let key = match env::var_os("HOLDEN_VM_SSH_KEY") {
        Some(k) => PathBuf::from(k),
        None => match util::home() {
            Some(h) => h.join(".ssh").join("id_ed25519_holden_mcp_vm"),
            None => return fail("HOME is not set and HOLDEN_VM_SSH_KEY is not given"),
        },
    };
    let override_addr = match env::var_os("HOLDEN_VM_ADDR") {
        None => None,
        Some(v) => match v.into_string() {
            Ok(s) => Some(s),
            Err(_) => return fail("HOLDEN_VM_ADDR is not UTF-8"),
        },
    };
    let (ip, source) = match resolve_addr(override_addr.as_deref(), OsStr::new("tart"), &name) {
        Ok(v) => v,
        Err(why) => return fail(&why),
    };
    let base = util::target_base().display().to_string();
    if !valid_target_base(&base) {
        return fail("HOLDEN_TARGET_BASE must be an absolute path of [A-Za-z0-9/._-] without ..");
    }
    let agent_private_root = match env::var_os("AGENT_PRIVATE_ROOT").filter(|v| !v.is_empty()) {
        None => None,
        Some(value) => match value.into_string() {
            Ok(root) if valid_target_base(&root) => Some(root),
            _ => {
                return fail(
                    "AGENT_PRIVATE_ROOT must be a UTF-8 absolute path of [A-Za-z0-9/._-] without ..",
                );
            }
        },
    };
    let vm = Vm {
        name,
        user,
        key,
        ip,
    };
    println!(
        "gate --linux-vm: VM {} at {} ({}) as {}; commit {sha}; target base {base}",
        vm.name,
        vm.ip,
        match source {
            AddrSource::Override => "HOLDEN_VM_ADDR; tart not called",
            AddrSource::Tart => "from tart ip",
        },
        vm.user
    );

    // Ship the exact commit as a git bundle.
    let bundle = std::env::temp_dir().join(format!("holden-mcp-{sha}.bundle"));
    if !ok(Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["bundle", "create", "--quiet"])
        .arg(&bundle)
        .arg("HEAD"))
    {
        return fail("git bundle create failed");
    }
    let work = "holden-mcp-gate";
    if !ok(vm.ssh().arg(format!("mkdir -p {work}"))) {
        return fail("ssh mkdir failed");
    }
    if !ok(vm
        .scp()
        .arg(&bundle)
        .arg(vm.remote(&format!("{work}/{sha}.bundle"))))
    {
        return fail("scp of the bundle failed");
    }
    let _ = fs::remove_file(&bundle);

    // The guard's private denylist lives only in private roots; copy it to the
    // VM user's private root so the VM guard scans for private identifiers too.
    let denylist = util::denylist_path().filter(|p| p.is_file());
    match &denylist {
        Some(d) => {
            let remote_dir = remote_private_dir(agent_private_root.as_deref());
            if !ok(vm
                .ssh()
                .arg(format!("mkdir -p {remote_dir} && chmod 700 {remote_dir}")))
                || !ok(vm
                    .scp()
                    .arg(d)
                    .arg(vm.remote(&format!("{remote_dir}/denylist.txt"))))
            {
                return fail("could not place the private denylist in the VM's private root");
            }
            println!("gate --linux-vm: private denylist placed in the VM user's private root");
        }
        None => println!(
            "gate --linux-vm: no host denylist; the VM guard will report denylist_absent (red)"
        ),
    }

    let record_remote = format!("{work}/{sha}-run.json");
    let script = gate_script(
        work,
        &sha,
        slice,
        &base,
        agent_private_root.as_deref(),
        &record_remote,
    );
    println!(
        "gate --linux-vm: fresh clone of {sha} in the VM, then `cargo xtask gate {slice}` with HOLDEN_TARGET_BASE={base} and AGENT_PRIVATE_ROOT={}",
        agent_private_root.as_deref().unwrap_or("<VM default>")
    );
    let code = util::run_inherit(vm.ssh().arg(script)).ok().flatten();

    // Copy the run record back, green or red.
    let Some(private) = util::private_root() else {
        return fail("no host private root; the run record cannot be copied back and checked");
    };
    let runs = private.join("agent-state").join("runs");
    let local = runs.join(format!(
        "{}-linux-vm.json",
        util::utc_compact(util::unix_now())
    ));
    if fs::create_dir_all(&runs).is_ok() && ok(vm.scp().arg(vm.remote(&record_remote)).arg(&local))
    {
        println!("gate --linux-vm: run_record: {}", local.display());
        compare_with_host(&runs, &local, &sha);
    } else {
        println!("gate --linux-vm: VM gate exit code {code:?}");
        return fail("the run record could not be copied back");
    }
    let targets_ok = fs::read(&local)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .is_some_and(|r| record_targets_under(&r, &base));
    println!("gate --linux-vm: run record build directories under {base}: {targets_ok}");
    println!("gate --linux-vm: VM gate exit code {code:?}");
    code == Some(0) && targets_ok
}

/// Compare the VM record's test-name list and counts with the latest host
/// record of the same commit, if one exists.
fn compare_with_host(runs: &Path, vm_record: &Path, sha: &str) {
    let read =
        |p: &Path| -> Option<serde_json::Value> { serde_json::from_slice(&fs::read(p).ok()?).ok() };
    let Some(vm) = read(vm_record) else {
        println!("gate --linux-vm: VM record unreadable");
        return;
    };
    let mut host: Option<(PathBuf, serde_json::Value)> = None;
    let mut entries: Vec<PathBuf> = fs::read_dir(runs)
        .map(|rd| rd.filter_map(Result::ok).map(|e| e.path()).collect())
        .unwrap_or_default();
    entries.sort();
    for p in entries {
        let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.ends_with("-linux-vm.json") || !name.ends_with(".json") {
            continue;
        }
        if let Some(v) = read(&p)
            && v["commit"] == sha
            && v["host_os"] != "linux"
        {
            host = Some((p, v));
        }
    }
    let Some((host_path, host)) = host else {
        println!("gate --linux-vm: no host record for {sha} to compare with");
        return;
    };
    let names_equal = vm["tests"]["names"] == host["tests"]["names"];
    let counts_equal = vm["tests"]["passed"] == host["tests"]["passed"]
        && vm["tests"]["failed"] == host["tests"]["failed"];
    println!(
        "gate --linux-vm: compared with host record {}: test names identical: {names_equal}; pass counts identical: {counts_equal} (host {} / VM {})",
        host_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?"),
        host["tests"]["passed"],
        vm["tests"]["passed"]
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(base: &str, dirs: &[&str]) -> serde_json::Value {
        let steps: Vec<serde_json::Value> = dirs
            .iter()
            .map(|d| serde_json::json!({ "name": "step", "target_dir": d }))
            .collect();
        serde_json::json!({ "target_base": base, "steps": steps })
    }

    #[test]
    fn vm_record_targets_must_sit_under_base() {
        let base = "/tmp/holden-mcp-target-run-1";
        let good = record(
            base,
            &[
                "/tmp/holden-mcp-target-run-1/holden-mcp-target-gate-fmt",
                "/tmp/holden-mcp-target-run-1/holden-mcp-target-gate-test",
                "-",
            ],
        );
        assert!(record_targets_under(&good, base));
        // A directory outside the base, a base not forwarded, or no build
        // directory at all: not shown under the base.
        let outside = record(base, &["/tmp/holden-mcp-target-gate-fmt"]);
        assert!(!record_targets_under(&outside, base));
        let sibling = record(base, &["/tmp/holden-mcp-target-run-10/x"]);
        assert!(!record_targets_under(&sibling, base));
        let unforwarded = record("/tmp", &["/tmp/holden-mcp-target-run-1/x"]);
        assert!(!record_targets_under(&unforwarded, base));
        assert!(!record_targets_under(&record(base, &["-"]), base));
        assert!(valid_target_base(base));
        for bad in ["tmp/x", "/tmp/a b", "/tmp/$(x)", "/tmp/../etc"] {
            assert!(!valid_target_base(bad), "{bad}");
        }
    }
}
