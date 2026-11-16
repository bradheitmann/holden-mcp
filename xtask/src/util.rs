//! Small shared helpers: process execution, private-root resolution, UTC time.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// The repository root (the parent of the `xtask` crate).
pub fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map_or(manifest.clone(), Path::to_path_buf)
}

/// `$HOME`, if set and absolute.
pub fn home() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
}

/// The holden-mcp private root:
/// `${AGENT_PRIVATE_ROOT:-${XDG_DATA_HOME:-$HOME/.local/share}/agent-private}/holden-mcp`.
pub fn private_root() -> Option<PathBuf> {
    if let Some(root) = env::var_os("AGENT_PRIVATE_ROOT").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(root).join("holden-mcp"));
    }
    let data = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| home().map(|h| h.join(".local").join("share")))?;
    Some(data.join("agent-private").join("holden-mcp"))
}

/// The private identifier denylist path.
pub fn denylist_path() -> Option<PathBuf> {
    private_root().map(|r| r.join("denylist.txt"))
}

/// The base of every per-run build directory: `${HOLDEN_TARGET_BASE:-/tmp}`.
pub fn target_base() -> PathBuf {
    env::var_os("HOLDEN_TARGET_BASE")
        .filter(|v| !v.is_empty())
        .map_or_else(|| PathBuf::from("/tmp"), PathBuf::from)
}

/// The per-purpose build directory: `<target base>/holden-mcp-target-<purpose>`.
pub fn target_dir(purpose: &str) -> PathBuf {
    target_base().join(format!("holden-mcp-target-{purpose}"))
}

/// Run a command to completion, capturing stdout and stderr.
pub fn capture(cmd: &mut Command) -> std::io::Result<Output> {
    cmd.stdin(Stdio::null()).output()
}

/// Run a command with inherited stdio; returns its exit code (None if killed).
pub fn run_inherit(cmd: &mut Command) -> std::io::Result<Option<i32>> {
    cmd.stdin(Stdio::null()).status().map(|s| s.code())
}

/// Trimmed stdout of a successful command, or None.
pub fn stdout_of(cmd: &mut Command) -> Option<String> {
    let out = capture(cmd).ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Seconds since the Unix epoch.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// UTC timestamp parts from Unix seconds (proleptic Gregorian calendar).
fn civil(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    // Howard Hinnant's days-from-civil inverse.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let as_u32 = |v: i64| u32::try_from(v).unwrap_or(0);
    let rem = u32::try_from(rem).unwrap_or(0);
    (
        y,
        as_u32(m),
        as_u32(d),
        rem / 3_600,
        (rem % 3_600) / 60,
        rem % 60,
    )
}

/// `YYYYMMDDTHHMMSSZ`.
pub fn utc_compact(secs: u64) -> String {
    let (y, mo, d, h, mi, s) = civil(secs);
    format!("{y:04}{mo:02}{d:02}T{h:02}{mi:02}{s:02}Z")
}

/// `YYYY-MM-DDTHH:MM:SSZ`.
pub fn utc_iso(secs: u64) -> String {
    let (y, mo, d, h, mi, s) = civil(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_formats_known_instants() {
        assert_eq!(utc_iso(0), "1970-01-01T00:00:00Z");
        assert_eq!(utc_iso(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(utc_compact(1_793_793_600), "20261104T120000Z");
    }
}
