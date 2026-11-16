//! `cargo xtask guard`: the public-safety and language guard (DONE P6).
//!
//! The population is every file git would commit: tracked files plus
//! untracked files that are not ignored (`git ls-files --cached --others
//! --exclude-standard`), so a file cannot slip past the guard by not yet
//! being staged.
//!
//! Each check ends as pass, refused (with named files) or a typed
//! limitation (the check could not run). A limitation is never a pass.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::util;

/// The outcome of one check.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Status {
    /// The check ran and found nothing.
    Pass,
    /// The check ran and refused; every finding names a file or a tool.
    Refused {
        /// The findings.
        findings: Vec<String>,
    },
    /// The check could not run.
    Limitation {
        /// Stable limitation code.
        code: String,
        /// Why.
        detail: String,
    },
}

/// One named check result.
#[derive(Clone, Debug, Serialize)]
pub struct Check {
    /// Check name.
    pub name: &'static str,
    /// Its outcome.
    #[serde(flatten)]
    pub status: Status,
    /// Files the check exempted, each with the reason (never silent).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exempt: Vec<String>,
}

impl Check {
    /// A check with no exemptions.
    pub fn new(name: &'static str, status: Status) -> Self {
        Self {
            name,
            status,
            exempt: Vec::new(),
        }
    }
}

/// Limitations a VM gate may carry because the host guard covers them.
pub const HOST_COVERED: [&str; 3] = ["cargo_deny_absent", "cargo_audit_absent", "charter_absent"];

/// The guard result.
#[derive(Clone, Debug, Serialize)]
pub struct GuardReport {
    /// Every check, in order.
    pub checks: Vec<Check>,
}

/// Overall verdict of a guard run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Every check passed.
    Pass,
    /// Every check passed except declared host-covered tool limitations.
    PassWithDeclaredLimitations,
    /// A limitation that was not declared: the guard did not pass.
    Limitation,
    /// A check refused.
    Refused,
}

impl GuardReport {
    /// Fold the checks into a verdict.
    pub fn verdict(&self, host_covered_tools: bool) -> Verdict {
        if self
            .checks
            .iter()
            .any(|c| matches!(c.status, Status::Refused { .. }))
        {
            return Verdict::Refused;
        }
        let limitations: Vec<&str> = self
            .checks
            .iter()
            .filter_map(|c| match &c.status {
                Status::Limitation { code, .. } => Some(code.as_str()),
                _ => None,
            })
            .collect();
        if limitations.is_empty() {
            Verdict::Pass
        } else if host_covered_tools && limitations.iter().all(|l| HOST_COVERED.contains(l)) {
            Verdict::PassWithDeclaredLimitations
        } else {
            Verdict::Limitation
        }
    }

    /// Print a human-readable report. Never prints a denylisted identifier.
    pub fn print(&self) {
        for c in &self.checks {
            match &c.status {
                Status::Pass => println!("guard: {:<22} pass", c.name),
                Status::Refused { findings } => {
                    println!(
                        "guard: {:<22} REFUSED ({} finding(s))",
                        c.name,
                        findings.len()
                    );
                    for f in findings {
                        println!("guard:   {f}");
                    }
                }
                Status::Limitation { code, detail } => {
                    println!("guard: {:<22} LIMITATION {code}: {detail}", c.name);
                }
            }
            for e in &c.exempt {
                println!("guard:   exempt {e}");
            }
        }
    }
}

fn limitation(code: &str, detail: impl Into<String>) -> Status {
    Status::Limitation {
        code: code.to_owned(),
        detail: detail.into(),
    }
}

fn refused_or_pass(findings: Vec<String>) -> Status {
    if findings.is_empty() {
        Status::Pass
    } else {
        Status::Refused { findings }
    }
}

/// Every file git would commit, relative to the repository root.
fn candidate_files(root: &Path) -> Result<Vec<String>, String> {
    git_ls_files(root, &["--cached", "--others", "--exclude-standard"])
}

/// Every file tracked in the index, relative to the repository root.
#[cfg(test)]
pub fn tracked_files(root: &Path) -> Result<Vec<String>, String> {
    git_ls_files(root, &["--cached"])
}

fn git_ls_files(root: &Path, selection: &[&str]) -> Result<Vec<String>, String> {
    let out = util::capture(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["ls-files", "-z"])
            .args(selection),
    )
    .map_err(|e| format!("git ls-files could not run: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let set: BTreeSet<String> = out
        .stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    Ok(set.into_iter().collect())
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Refuse interpreter sources and container build files.
pub fn check_languages(files: &[String]) -> Status {
    let mut findings = Vec::new();
    for f in files {
        let name = file_name(f).to_ascii_lowercase();
        let ext = name.rsplit_once('.').map(|(_, e)| e);
        if matches!(ext, Some("py" | "js" | "mjs" | "cjs")) {
            findings.push(format!(
                "{f}: forbidden language (.{})",
                ext.unwrap_or_default()
            ));
        }
        if name.starts_with("dockerfile")
            || name.ends_with(".dockerfile")
            || name == "containerfile"
        {
            findings.push(format!("{f}: container build file"));
        }
    }
    refused_or_pass(findings)
}

/// Absolute home-directory prefixes, built at run time so this source file
/// does not itself contain them.
fn home_prefixes() -> [String; 2] {
    [["/", "Users", "/"].concat(), ["/", "home", "/"].concat()]
}

fn read_candidate(root: &Path, f: &str) -> Option<Vec<u8>> {
    let p = root.join(f);
    fs::symlink_metadata(&p)
        .ok()
        .filter(fs::Metadata::is_file)
        .and_then(|_| fs::read(&p).ok())
}

/// Refuse absolute home paths in file contents or names.
pub fn check_home_paths(root: &Path, files: &[String]) -> Status {
    let prefixes = home_prefixes();
    let mut findings = Vec::new();
    for f in files {
        for p in &prefixes {
            if f.contains(p.as_str()) {
                findings.push(format!("{f}: absolute home path in the file name"));
            }
        }
        let Some(bytes) = read_candidate(root, f) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        for (n, line) in text.lines().enumerate() {
            if prefixes.iter().any(|p| line.contains(p.as_str())) {
                findings.push(format!("{f}:{}: absolute home path", n + 1));
            }
        }
    }
    refused_or_pass(findings)
}

fn word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Whether `haystack` (lowercase) contains `needle` (lowercase) as a whole word.
fn contains_word(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let at = start + pos;
        let end = at + needle.len();
        let before_ok = at == 0 || !word_byte(h[at - 1]);
        let after_ok = end >= h.len() || !word_byte(h[end]);
        if before_ok && after_ok {
            return true;
        }
        start = at + 1;
        while start < haystack.len() && !haystack.is_char_boundary(start) {
            start += 1;
        }
    }
    false
}

/// One allowance from `charter/safety-allow.txt`: `<path or prefix*> — <reason>`.
#[derive(Debug)]
struct Allow {
    pattern: String,
}

impl Allow {
    fn matches(&self, file: &str) -> bool {
        match self.pattern.strip_suffix('*') {
            Some(prefix) => file.starts_with(prefix),
            None => self.pattern == file,
        }
    }
}

/// The shared allow-list (the same file `charter safety` reads). Entries
/// without a reason are refused.
fn allow_list(root: &Path) -> Result<Vec<Allow>, Vec<String>> {
    let path = root.join("charter").join("safety-allow.txt");
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let mut bad = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line.split_once(" — ") {
            Some((pattern, reason)) if !pattern.trim().is_empty() && !reason.trim().is_empty() => {
                out.push(Allow {
                    pattern: pattern.trim().to_owned(),
                });
            }
            _ => bad.push(format!(
                "charter/safety-allow.txt:{}: an allowance needs '<path> — <reason>'",
                n + 1
            )),
        }
    }
    if bad.is_empty() { Ok(out) } else { Err(bad) }
}

/// Refuse denylisted private identifiers (case-insensitive, whole word).
/// Findings name the file, line and denylist entry number, never the
/// identifier itself.
pub fn check_denylist(root: &Path, files: &[String]) -> Status {
    let Some(path) = util::denylist_path() else {
        return limitation(
            "denylist_absent",
            "no private root could be resolved (AGENT_PRIVATE_ROOT, XDG_DATA_HOME, HOME)",
        );
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return limitation(
            "denylist_absent",
            "the private denylist <private root>/denylist.txt is absent or unreadable",
        );
    };
    let entries: Vec<(usize, String)> = text
        .lines()
        .enumerate()
        .map(|(n, l)| (n + 1, l.trim().to_lowercase()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'))
        .collect();
    if entries.is_empty() {
        return limitation("denylist_empty", "the private denylist has no entries");
    }
    let allows = match allow_list(root) {
        Ok(a) => a,
        Err(findings) => return Status::Refused { findings },
    };
    let mut findings = Vec::new();
    for f in files {
        if allows.iter().any(|a| a.matches(f)) {
            continue;
        }
        let lower_name = f.to_lowercase();
        for (n, id) in &entries {
            if contains_word(&lower_name, id) {
                findings.push(format!(
                    "<path redacted>: file name holds denylist entry #{n}"
                ));
            }
        }
        let Some(bytes) = read_candidate(root, f) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes).to_lowercase();
        for (ln, line) in text.lines().enumerate() {
            for (n, id) in &entries {
                if contains_word(line, id) {
                    findings.push(format!("{f}:{}: denylist entry #{n}", ln + 1));
                }
            }
        }
    }
    refused_or_pass(findings)
}

/// The repository license (owner decision D-07, 2026-11-04), as an SPDX id.
pub const LICENSE_SPDX: &str = "PolyForm-Noncommercial-1.0.0";

/// SHA-256 of `PolyForm-Noncommercial-1.0.0.md` from the PolyForm project at
/// upstream commit `76a278c402bc43b8d2b561da140b0f3e17263015`. `LICENSE`
/// must be byte-identical to it.
pub const LICENSE_SHA256: &str = "c0ea4a896d2c8c394b29f9427589996db826cd501c512279ff0ed3ef48fabbe5";

/// SHA-256 of `NOTICE`: exactly the two Required Notice lines of the slice-1b
/// done list (licensor copyright line, then the commercial-use contact line),
/// each ending in a newline. Pinned by hash so the licensor name stays in
/// the files that carry the recorded licensor-name exception.
pub const NOTICE_SHA256: &str = "b48acabe3c9a16a65780c380afc1c05f5578286a1f3119612b4eca99865c717b";

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `LICENSE` is byte-identical to the pinned official license text, and
/// `NOTICE` holds exactly the pinned Required Notice lines.
pub fn check_license_text(root: &Path) -> Status {
    let mut findings = Vec::new();
    match fs::read(root.join("LICENSE")) {
        Ok(bytes) => {
            let actual = sha256_hex(&bytes);
            if actual != LICENSE_SHA256 {
                findings.push(format!(
                    "LICENSE: sha256 {actual} is not the pinned {LICENSE_SPDX} text ({LICENSE_SHA256})"
                ));
            }
        }
        Err(_) => findings.push("LICENSE: missing".to_owned()),
    }
    match fs::read(root.join("NOTICE")) {
        Ok(bytes) => {
            let actual = sha256_hex(&bytes);
            if actual != NOTICE_SHA256 {
                findings.push(format!(
                    "NOTICE: sha256 {actual} is not the pinned Required Notice text ({NOTICE_SHA256})"
                ));
            }
        }
        Err(_) => findings.push("NOTICE: missing".to_owned()),
    }
    refused_or_pass(findings)
}

/// Every workspace package declares `license = "PolyForm-Noncommercial-1.0.0"`.
/// Any other license, or none, is refused and the crate is named.
pub fn check_crate_licenses(root: &Path) -> Status {
    let mut findings = Vec::new();
    let out = util::capture(Command::new("cargo").current_dir(root).args([
        "metadata",
        "--format-version",
        "1",
        "--no-deps",
        "--locked",
    ]));
    let out = match out {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            findings.push(format!(
                "cargo metadata failed: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            ));
            return Status::Refused { findings };
        }
        Err(e) => {
            findings.push(format!("cargo metadata could not run: {e}"));
            return Status::Refused { findings };
        }
    };
    let meta: serde_json::Value = match serde_json::from_slice(&out.stdout) {
        Ok(v) => v,
        Err(e) => {
            findings.push(format!("cargo metadata output unreadable: {e}"));
            return Status::Refused { findings };
        }
    };
    let packages = meta["packages"].as_array().cloned().unwrap_or_default();
    if packages.is_empty() {
        findings.push("cargo metadata lists no workspace package".to_owned());
    }
    for p in &packages {
        let name = p["name"].as_str().unwrap_or("<unnamed>");
        let license = p["license"].as_str();
        if license != Some(LICENSE_SPDX) {
            findings.push(format!(
                "crate {name}: license is {}, not \"{LICENSE_SPDX}\"",
                license.map_or_else(|| "absent".to_owned(), |l| format!("\"{l}\""))
            ));
        }
    }
    refused_or_pass(findings)
}

/// Paths that may name the former license because they are third-party
/// metadata or pinned contracts, never a claim about this repository, with
/// the reason the guard prints:
/// - `Cargo.lock`: third-party package metadata;
/// - `charter/DONE/`: done lists pinned by hash at dispatch.
///
/// Owner-decision records are not exempt as a directory (slice-1b obligation
/// D1): each exempt record is listed by exact path, with its reason, in
/// [`LICENSE_MENTION_ALLOW`]. A QA record is exempt only when its slice is
/// accepted and its current bytes are pinned (see [`qa_pin_exemption`]).
const STALE_LICENSE_EXEMPT: [(&str, &str); 2] = [
    ("Cargo.lock", "third-party package metadata"),
    ("charter/DONE/", "done list pinned by hash at dispatch"),
];

/// Exact-path exemptions from `stale_license_mentions`, one
/// `<path> — <reason>` per line.
pub const LICENSE_MENTION_ALLOW: &str = "charter/license-mention-allow.txt";

/// The exact-path allowances of [`LICENSE_MENTION_ALLOW`] as `(path, reason)`.
/// An absent file allows nothing. An entry without a reason, or with a glob,
/// is refused.
fn license_mention_allow(root: &Path) -> Result<Vec<(String, String)>, Vec<String>> {
    let Ok(text) = fs::read_to_string(root.join(LICENSE_MENTION_ALLOW)) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let mut bad = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line.split_once(" — ") {
            Some((path, reason))
                if !path.trim().is_empty()
                    && !reason.trim().is_empty()
                    && !path.contains('*')
                    && !path.trim().ends_with('/') =>
            {
                out.push((path.trim().to_owned(), reason.trim().to_owned()));
            }
            _ => bad.push(format!(
                "{LICENSE_MENTION_ALLOW}:{}: an allowance needs '<exact file path> — <reason>'",
                n + 1
            )),
        }
    }
    if bad.is_empty() { Ok(out) } else { Err(bad) }
}

/// Whether the slice's dispatch record says it was accepted.
fn slice_accepted(root: &Path, slice: &str) -> bool {
    let path = root
        .join("charter")
        .join(".dispatch")
        .join(format!("{slice}.json"));
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .is_some_and(|v| v["state"].as_str() == Some("accepted"))
}

/// The reason a `qa/<slice>/...` file is exempt, or `None`. It is exempt only
/// when `<slice>` is recorded accepted and `charter/qa-pins/<slice>.sha256`
/// lists the file's path with the sha256 of its current bytes.
pub fn qa_pin_exemption(root: &Path, file: &str) -> Option<String> {
    let rest = file.strip_prefix("qa/")?;
    let (slice, name) = rest.split_once('/')?;
    if slice.is_empty() || slice.starts_with('.') || name.is_empty() {
        return None;
    }
    if !slice_accepted(root, slice) {
        return None;
    }
    let pins_rel = format!("charter/qa-pins/{slice}.sha256");
    let pins = fs::read_to_string(root.join(&pins_rel)).ok()?;
    let actual = sha256_hex(&read_candidate(root, file)?);
    let pinned = pins.lines().any(|l| {
        l.split_once("  ")
            .is_some_and(|(hash, path)| hash.trim() == actual && path.trim() == file)
    });
    pinned.then(|| format!("accepted {slice}, sha256 pinned in {pins_rel}"))
}

/// Whether `line` of `file` is a third-party license allow-list entry
/// (`deny.toml`, a line holding only a quoted license id).
fn third_party_allow_entry(file: &str, line: &str) -> bool {
    file == "deny.toml" && {
        let t = line.trim().trim_end_matches(',');
        t.len() > 2 && t.starts_with('"') && t.ends_with('"')
    }
}

/// No file still names the former license (its SPDX id, matched as a whole word,
/// case-insensitive) outside third-party contexts and immutable records.
/// The word is assembled at run time so this source does not itself hold it.
#[cfg(test)]
pub fn check_stale_license_mentions(root: &Path, files: &[String]) -> Status {
    scan_stale_license_mentions(root, files).status
}

/// The `stale_license_mentions` outcome plus every exemption it applied.
#[derive(Debug)]
pub struct StaleLicenseScan {
    /// Pass or refused (with file and line).
    pub status: Status,
    /// `<file>: <n> line(s) exempt: <reason>` for every exempted file that
    /// names the former license.
    pub exempt: Vec<String>,
}

/// Run `stale_license_mentions`, recording why each exempt file is exempt.
pub fn scan_stale_license_mentions(root: &Path, files: &[String]) -> StaleLicenseScan {
    let word = ["m", "it"].concat();
    let allows = match license_mention_allow(root) {
        Ok(a) => a,
        Err(findings) => {
            return StaleLicenseScan {
                status: Status::Refused { findings },
                exempt: Vec::new(),
            };
        }
    };
    let mut findings = Vec::new();
    let mut exempt = Vec::new();
    for f in files {
        let Some(bytes) = read_candidate(root, f) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<usize> = text
            .lines()
            .enumerate()
            .filter(|(_, line)| {
                contains_word(&line.to_lowercase(), &word) && !third_party_allow_entry(f, line)
            })
            .map(|(n, _)| n + 1)
            .collect();
        if lines.is_empty() {
            continue;
        }
        let reason = STALE_LICENSE_EXEMPT
            .iter()
            .find(|(e, _)| f == e || (e.ends_with('/') && f.starts_with(e)))
            .map(|(_, r)| (*r).to_owned())
            .or_else(|| {
                allows
                    .iter()
                    .find(|(p, _)| p == f)
                    .map(|(_, r)| format!("listed in {LICENSE_MENTION_ALLOW}: {r}"))
            })
            .or_else(|| qa_pin_exemption(root, f));
        match reason {
            Some(r) => exempt.push(format!("{f}: {} line(s) exempt: {r}", lines.len())),
            None => findings.extend(
                lines
                    .iter()
                    .map(|n| format!("{f}:{n}: names the former license")),
            ),
        }
    }
    StaleLicenseScan {
        status: refused_or_pass(findings),
        exempt,
    }
}

/// The licensor-name exception record check (slice-1c P3), run by the test
/// `licensor_exception_lists_every_file` against the files tracked on the
/// branch under test.
#[cfg(test)]
pub mod licensor {
    use super::{BTreeSet, Path, allow_list, contains_word, fs, read_candidate, sha256_hex};

    /// SHA-256 of the licensor's name, lowercase, its words joined by one space.
    /// Files that carry the name are found by hashing every pair of adjacent
    /// words, so this source does not hold the name.
    pub const LICENSOR_NAME_SHA256: &str =
        "c42005ba9026ae7bcc52c9a142a35c935c82fcfee533a110fb2f751483203b23";

    /// The owner decision whose "Licensor-name exception" section lists every file
    /// allowed to carry the licensor name.
    pub const LICENSE_DECISION_RECORD: &str =
        "charter/owner-decisions/2026-11-04-license-polyform-noncommercial.md";

    const LICENSOR_EXCEPTION_HEADING: &str = "## Licensor-name exception";

    /// Whether `bytes` contain the licensor name: two adjacent words (lowercase,
    /// alphanumeric runs) whose space-joined form hashes to
    /// [`LICENSOR_NAME_SHA256`].
    pub fn carries_licensor_name(bytes: &[u8]) -> bool {
        let lower = String::from_utf8_lossy(bytes).to_lowercase();
        let words: Vec<&str> = lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        words
            .windows(2)
            .any(|w| sha256_hex(format!("{} {}", w[0], w[1]).as_bytes()) == LICENSOR_NAME_SHA256)
    }

    /// The files among `files` that carry the licensor name.
    pub fn licensor_name_carriers(root: &Path, files: &[String]) -> Vec<String> {
        files
            .iter()
            .filter(|f| read_candidate(root, f).is_some_and(|b| carries_licensor_name(&b)))
            .cloned()
            .collect()
    }

    /// One file in the licensor-name exception record.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ExceptionEntry {
        /// Repository-relative path.
        pub path: String,
        /// The record marks the file as forthcoming (not yet on this branch).
        pub forthcoming: bool,
    }

    /// The files listed in the record's licensor-name exception section: every
    /// bullet that begins with a backquoted path. A bullet containing the word
    /// "forthcoming" marks its file as forthcoming.
    pub fn licensor_exception_entries(record: &str) -> Vec<ExceptionEntry> {
        let mut out = Vec::new();
        let mut inside = false;
        for line in record.lines() {
            if line.starts_with("## ") {
                inside = line.starts_with(LICENSOR_EXCEPTION_HEADING);
                continue;
            }
            if !inside {
                continue;
            }
            let Some(rest) = line.trim_start().strip_prefix("- `") else {
                continue;
            };
            let Some((path, _)) = rest.split_once('`') else {
                continue;
            };
            out.push(ExceptionEntry {
                path: path.to_owned(),
                forthcoming: contains_word(&line.to_lowercase(), "forthcoming"),
            });
        }
        out
    }

    /// Compare the licensor-name exception record with the files in `tracked`
    /// that carry the name. Findings, empty when complete:
    /// - a carrier the record does not list;
    /// - a listed file that is not tracked, unless the record marks it forthcoming;
    /// - a listed, tracked file that does not carry the name;
    /// - a listed file or carrier without an entry in `charter/safety-allow.txt`.
    pub fn licensor_exception_findings(root: &Path, tracked: &[String]) -> Vec<String> {
        let mut findings = Vec::new();
        let Ok(record) = fs::read_to_string(root.join(LICENSE_DECISION_RECORD)) else {
            return vec![format!("{LICENSE_DECISION_RECORD}: missing")];
        };
        let entries = licensor_exception_entries(&record);
        if entries.is_empty() {
            findings.push(format!(
                "{LICENSE_DECISION_RECORD}: the licensor-name exception section lists no file"
            ));
        }
        let carriers = licensor_name_carriers(root, tracked);
        let allows = allow_list(root).unwrap_or_else(|bad| {
            findings.extend(bad);
            Vec::new()
        });
        for c in &carriers {
            if !entries.iter().any(|e| &e.path == c) {
                findings.push(format!(
                    "{c}: carries the licensor name but the exception record does not list it"
                ));
            }
        }
        for e in &entries {
            let is_tracked = tracked.contains(&e.path);
            if !is_tracked && !e.forthcoming {
                findings.push(format!(
                "{}: listed in the exception record but not tracked, and not marked forthcoming",
                e.path
            ));
            }
            if is_tracked && !carriers.contains(&e.path) {
                findings.push(format!(
                    "{}: listed in the exception record but does not carry the licensor name",
                    e.path
                ));
            }
        }
        let mut needs_allow: BTreeSet<&str> = carriers.iter().map(String::as_str).collect();
        needs_allow.extend(entries.iter().map(|e| e.path.as_str()));
        for p in needs_allow {
            if !allows.iter().any(|a| a.matches(p)) {
                findings.push(format!("{p}: not in charter/safety-allow.txt"));
            }
        }
        findings
    }
}

fn cargo_subcommand_present(root: &Path, sub: &str) -> bool {
    util::capture(
        Command::new("cargo")
            .current_dir(root)
            .args([sub, "--version"]),
    )
    .is_ok_and(|o| o.status.success())
}

fn run_tool(root: &Path, name: &str, cmd: &mut Command) -> Status {
    match util::capture(cmd.current_dir(root)) {
        Ok(o) if o.status.success() => Status::Pass,
        Ok(o) => {
            let mut findings = vec![format!("{name} exited {:?}", o.status.code())];
            let text = format!(
                "{}\n{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            for line in text.lines().filter(|l| {
                let l = l.trim_start();
                l.starts_with("error") || l.starts_with("charter: safety") || l.contains("RUSTSEC-")
            }) {
                findings.push(line.trim().to_owned());
            }
            Status::Refused { findings }
        }
        Err(e) => Status::Refused {
            findings: vec![format!("{name} could not run: {e}")],
        },
    }
}

/// `cargo deny check` (licenses, bans, advisories, sources).
pub fn check_cargo_deny(root: &Path) -> Status {
    if !cargo_subcommand_present(root, "deny") {
        return limitation(
            "cargo_deny_absent",
            "cargo-deny is not installed on this host",
        );
    }
    run_tool(
        root,
        "cargo deny check",
        Command::new("cargo").args(["deny", "--locked", "check"]),
    )
}

/// `cargo audit` against the lockfile.
pub fn check_cargo_audit(root: &Path) -> Status {
    if !cargo_subcommand_present(root, "audit") {
        return limitation(
            "cargo_audit_absent",
            "cargo-audit is not installed on this host",
        );
    }
    run_tool(
        root,
        "cargo audit",
        Command::new("cargo").args(["audit", "--deny", "warnings"]),
    )
}

/// The pinned charter tool: `$CHARTER_BIN`, else
/// `$HOME/open_protocols/charter/target/release/charter`.
pub fn charter_bin() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("CHARTER_BIN").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(p));
    }
    util::home().map(|h| {
        h.join("open_protocols")
            .join("charter")
            .join("target")
            .join("release")
            .join("charter")
    })
}

/// `charter safety --denylist <private denylist>` on the files tracked at HEAD.
pub fn check_charter_safety(root: &Path) -> Status {
    let Some(bin) = charter_bin().filter(|b| b.is_file()) else {
        return limitation(
            "charter_absent",
            "the charter tool is not installed (CHARTER_BIN or the default build path)",
        );
    };
    let Some(deny) = util::denylist_path().filter(|d| d.is_file()) else {
        return limitation(
            "denylist_absent",
            "charter safety needs the private denylist, which is absent",
        );
    };
    run_tool(
        root,
        "charter safety",
        Command::new(bin).arg("safety").arg("--denylist").arg(deny),
    )
}

/// Run every guard check.
pub fn run(root: &Path) -> GuardReport {
    let mut checks = Vec::new();
    match candidate_files(root) {
        Ok(files) => {
            checks.push(Check::new("languages", check_languages(&files)));
            checks.push(Check::new(
                "absolute_home_paths",
                check_home_paths(root, &files),
            ));
            checks.push(Check::new(
                "private_identifiers",
                check_denylist(root, &files),
            ));
            let scan = scan_stale_license_mentions(root, &files);
            checks.push(Check {
                name: "stale_license_mentions",
                status: scan.status,
                exempt: scan.exempt,
            });
        }
        Err(e) => checks.push(Check::new(
            "candidate_files",
            Status::Refused { findings: vec![e] },
        )),
    }
    checks.push(Check::new("license_text", check_license_text(root)));
    checks.push(Check::new("crate_licenses", check_crate_licenses(root)));
    checks.push(Check::new("cargo_deny", check_cargo_deny(root)));
    checks.push(Check::new("cargo_audit", check_cargo_audit(root)));
    checks.push(Check::new("charter_safety", check_charter_safety(root)));
    GuardReport { checks }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_refuses_python_and_js_and_dockerfiles() {
        let files: Vec<String> = [
            "tools/x.py",
            "a.mjs",
            "b.CJS",
            "web/c.js",
            "Dockerfile",
            "ok.rs",
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        let Status::Refused { findings } = check_languages(&files) else {
            panic!("must refuse");
        };
        assert_eq!(findings.len(), 5);
        assert!(findings[0].starts_with("tools/x.py"));
    }

    #[test]
    fn whole_word_matching() {
        assert!(contains_word("see acme here", "acme"));
        assert!(contains_word("x/acme/y", "acme"));
        assert!(!contains_word("acmecorp", "acme"));
        assert!(!contains_word("my_acme", "acme"));
    }

    #[test]
    fn stale_license_mentions_ignore_only_third_party_and_records() {
        let root = std::env::temp_dir().join(format!(
            "holden-mcp-xtask-stale-license-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("charter/DONE")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        let word = ["M", "IT"].concat();
        let files: Vec<String> = [
            ("README.md", format!("{word}. See `LICENSE`.\n")),
            ("deny.toml", format!("allow = [\n    \"{word}\",\n]\n")),
            ("Cargo.lock", format!("license = \"{word}\"\n")),
            (
                "charter/DONE/slice-x.md",
                format!("from {word} to PolyForm\n"),
            ),
            (
                "docs/ok.md",
                "commit, submit and limit are fine\n".to_owned(),
            ),
        ]
        .into_iter()
        .map(|(f, body)| {
            fs::write(root.join(f), body).unwrap();
            f.to_owned()
        })
        .collect();
        let status = check_stale_license_mentions(&root, &files);
        assert_eq!(
            status,
            Status::Refused {
                findings: vec!["README.md:1: names the former license".to_owned()]
            }
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn undeclared_limitation_is_not_a_pass() {
        let r = GuardReport {
            checks: vec![Check::new(
                "private_identifiers",
                limitation("denylist_absent", "x"),
            )],
        };
        assert_eq!(r.verdict(true), Verdict::Limitation);
        let r = GuardReport {
            checks: vec![Check::new(
                "cargo_deny",
                limitation("cargo_deny_absent", "x"),
            )],
        };
        assert_eq!(r.verdict(false), Verdict::Limitation);
        assert_eq!(r.verdict(true), Verdict::PassWithDeclaredLimitations);
    }
}
