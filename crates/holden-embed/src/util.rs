//! Small shared helpers: files, digests, git and the private root.

use std::fs::{self, DirBuilder, Permissions};
use std::io;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::{Error, refused};

/// An [`Error::Io`] for `path`.
pub(crate) fn io_at(path: &Path) -> impl FnOnce(io::Error) -> Error + '_ {
    move |e| Error::Io {
        path: path.to_path_buf(),
        kind: e.kind(),
    }
}

/// SHA-256 of `bytes`, lowercase hex.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Create `dir` (and its parents) with mode 0700 when absent.
pub(crate) fn private_dir(dir: &Path) -> Result<(), Error> {
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(io_at(dir))
}

/// Write `bytes` to `path` unless it already holds exactly them, creating
/// its parent directory; then set `mode`. Returns whether it wrote.
pub(crate) fn write_if_changed(path: &Path, bytes: &[u8], mode: u32) -> Result<bool, Error> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(io_at(dir))?;
    }
    let same = fs::read(path).is_ok_and(|old| old == bytes);
    if !same {
        fs::write(path, bytes).map_err(io_at(path))?;
    }
    fs::set_permissions(path, Permissions::from_mode(mode)).map_err(io_at(path))?;
    Ok(!same)
}

/// Pretty JSON with a final line feed.
pub(crate) fn json_bytes<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    let mut b = serde_json::to_vec_pretty(value).map_err(|e| refused(format!("json: {e}")))?;
    b.push(b'\n');
    Ok(b)
}

/// Read and parse a JSON file.
pub(crate) fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let bytes = fs::read(path).map_err(io_at(path))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| refused(format!("{}: not the expected JSON: {e}", path.display())))
}

/// The private root:
/// `${AGENT_PRIVATE_ROOT:-${XDG_DATA_HOME:-$HOME/.local/share}/agent-private}/holden-mcp`.
#[must_use]
pub fn private_root() -> Option<PathBuf> {
    let base = if let Some(r) = std::env::var_os("AGENT_PRIVATE_ROOT").filter(|v| !v.is_empty()) {
        PathBuf::from(r)
    } else if let Some(x) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        PathBuf::from(x).join("agent-private")
    } else {
        std::env::home_dir()?.join(".local/share/agent-private")
    };
    Some(base.join("holden-mcp"))
}

/// Git for a host repository: a scrubbed environment (the repository's own
/// config still applies, the user's global config does not), and hooks off
/// for every command holden runs.
#[derive(Debug, Clone)]
pub(crate) struct HostGit {
    repo: PathBuf,
    home: PathBuf,
}

impl HostGit {
    pub(crate) fn new(repo: &Path, home: &Path) -> Self {
        Self {
            repo: repo.to_path_buf(),
            home: home.to_path_buf(),
        }
    }

    /// Run git with `args`; its trimmed stdout.
    pub(crate) fn run(&self, args: &[&str]) -> Result<String, Error> {
        self.raw(args)
            .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
    }

    /// Run git with `args`; its stdout, byte for byte.
    pub(crate) fn raw(&self, args: &[&str]) -> Result<Vec<u8>, Error> {
        let out = Command::new("git")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &self.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .arg("-C")
            .arg(&self.repo)
            .args(["-c", "core.fsmonitor=false"])
            .args(args)
            .output()
            .map_err(|e| Error::Process {
                program: "git".to_owned(),
                why: e.to_string(),
            })?;
        if out.status.success() {
            Ok(out.stdout)
        } else {
            Err(Error::Process {
                program: format!("git {}", args.first().copied().unwrap_or_default()),
                why: String::from_utf8_lossy(&out.stderr).trim().to_owned(),
            })
        }
    }

    /// Whether git answers with success.
    pub(crate) fn succeeds(&self, args: &[&str]) -> bool {
        self.run(args).is_ok()
    }
}

/// Copy every regular file under `from` to `to`, recursively.
pub(crate) fn copy_tree(from: &Path, to: &Path) -> Result<(), Error> {
    fs::create_dir_all(to).map_err(io_at(to))?;
    let mut entries: Vec<_> = fs::read_dir(from)
        .map_err(io_at(from))?
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(fs::DirEntry::file_name);
    for e in entries {
        let src = e.path();
        let dst = to.join(e.file_name());
        let ty = e.file_type().map_err(io_at(&src))?;
        if ty.is_dir() {
            copy_tree(&src, &dst)?;
        } else if ty.is_file() {
            fs::copy(&src, &dst).map_err(io_at(&src))?;
        }
    }
    Ok(())
}
