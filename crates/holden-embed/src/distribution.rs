//! One-time machine initialization and per-harness MCP connection files.
//!
//! `init` creates only an implementer-role socket server configuration and
//! the ordinary agent keys. It never starts a service and never emits key
//! material. Harness entries run `holden mcp`, whose stdio relay starts that
//! server as a bounded child process on demand.

use std::fs::{self, DirBuilder, Permissions};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use holden_mcp::{Role, ServerConfig};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::host::MCP_SERVER_NAME;
use crate::state::State;
use crate::util::{json_bytes, write_if_changed};
use crate::{Error, refused};

/// The four slash-command workflow names, in W1–W4 order.
pub const SLASH_WORKFLOWS: [&str; 4] = ["onboard", "check", "explain", "status"];

/// The one MCP workflow tool each slash subcommand may call.
#[must_use]
pub fn workflow_tool(subcommand: &str) -> Option<&'static str> {
    match subcommand {
        "onboard" => Some("holden_onboard"),
        "check" => Some("holden_check"),
        "explain" => Some("holden_explain"),
        "status" => Some("holden_status"),
        _ => None,
    }
}

/// The public command instructions installed for Claude Code and Droid.
#[must_use]
pub fn holden_command() -> &'static str {
    "---\ndescription: Run one Holden workflow; owner actions remain owner-only\n---\n\
Use exactly one MCP workflow tool for /holden $ARGUMENTS.\n\
- onboard: call holden_onboard with host set to the current repository root.\n\
- check: call holden_check with host set to the current repository root.\n\
- explain: call holden_explain with host set to the current repository root and the supplied run_id.\n\
- status: call holden_status with host set to the current repository root.\n\
No other subcommand is valid. Never call a low-level approval, nonce, reveal, observation, verdict, telemetry or audit tool for this command.\n\
When the workflow returns OWNER_ACTION_REQUIRED or pending_owner_actions, print each owner_command verbatim and stop. Do not execute it.\n"
}

/// The short per-root socket directory. Unix socket names have a small
/// platform limit; private roots under macOS temporary directories can be
/// longer than that limit, so this stable content-derived name lives in /tmp.
#[must_use]
pub fn socket_dir(root: &Path) -> PathBuf {
    let digest = hex::encode(Sha256::digest(root.as_os_str().as_encoded_bytes()));
    PathBuf::from("/tmp").join(format!(
        "holden-{}-{}",
        holden_seal::keystore::current_uid(),
        &digest[..16]
    ))
}

/// The result of safe, idempotent machine initialization.
#[derive(Clone, Debug)]
pub struct InitReport {
    /// The sealed implementer server configuration.
    pub config: PathBuf,
    /// Names of keys created this time, never their values.
    pub keys_created: Vec<&'static str>,
}

fn secure_existing_root(root: &Path) -> Result<(), Error> {
    match fs::symlink_metadata(root) {
        Ok(meta) => {
            if !meta.file_type().is_dir() {
                return Err(refused("the private root is not a directory"));
            }
            if meta.uid() != holden_seal::keystore::current_uid() || meta.mode() & 0o077 != 0 {
                return Err(refused(format!(
                    "{} has mode {:04o}; it must be owned by this user and not group- or world-accessible",
                    root.display(),
                    meta.mode() & 0o7777
                )));
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::Io {
            path: root.to_path_buf(),
            kind: e.kind(),
        }),
    }
}

fn secure_server_dirs(state: &State) -> Result<(), Error> {
    for dir in [
        state.dir().to_path_buf(),
        state.plans_dir(),
        state.dir().join("server"),
        socket_dir(state.dir()),
    ] {
        secure_existing_root(&dir)?;
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&dir)
            .map_err(|e| Error::Io {
                path: dir.clone(),
                kind: e.kind(),
            })?;
        fs::set_permissions(&dir, Permissions::from_mode(0o700)).map_err(|e| Error::Io {
            path: dir.clone(),
            kind: e.kind(),
        })?;
    }
    Ok(())
}

/// Initialize the machine's private root. Existing keys and identical config
/// bytes are retained; an insecure existing private root or key store is
/// refused rather than repaired silently.
pub fn init(root: &Path) -> Result<InitReport, Error> {
    if !root.is_absolute() {
        return Err(refused("the private root must be absolute"));
    }
    secure_existing_root(root)?;
    let state = State::new(root);
    secure_server_dirs(&state)?;
    let keys_created = state.ensure_keys()?;
    let config_path = state.server_config(Role::Implementer);
    let config = json!({
        "socket_dir": socket_dir(root),
        "key_dir": state.keys_dir(),
        "epoch_key": "epoch",
        "seal_key": "seal",
        "plans_dir": state.plans_dir(),
        "run_window_ms": 600_000_u64,
        "roles": [{
            "uid": holden_seal::keystore::current_uid(),
            "role": Role::Implementer.as_str(),
        }],
    });
    ServerConfig::from_json(&config.to_string()).map_err(|e| refused(e.to_string()))?;
    write_if_changed(&config_path, &json_bytes(&config)?, 0o600)?;
    // Loading reopens the exact config bytes, including its file-mode check.
    ServerConfig::load(&config_path).map_err(|e| refused(e.to_string()))?;
    Ok(InitReport {
        config: config_path,
        keys_created,
    })
}

fn merge_json_server(path: &Path, entry: Value) -> Result<Vec<u8>, Error> {
    let mut root: Value = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| refused(format!("{} is not JSON: {e}", path.display())))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(e) => {
            return Err(Error::Io {
                path: path.to_path_buf(),
                kind: e.kind(),
            });
        }
    };
    let object = root
        .as_object_mut()
        .ok_or_else(|| refused(format!("{} must be a JSON object", path.display())))?;
    let servers = object
        .entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| refused(format!("{} mcpServers must be an object", path.display())))?;
    servers.insert(MCP_SERVER_NAME.to_owned(), entry);
    json_bytes(&root)
}

fn relay_args() -> Vec<String> {
    vec!["mcp".to_owned()]
}

fn relay_entry() -> Value {
    json!({"command": "holden", "args": relay_args()})
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}

/// Write and validate one harness's connection files. Re-running writes no
/// bytes when the files already match.
fn write_harness_config_at(
    host: &Path,
    harness: &str,
    _root: Option<&Path>,
) -> Result<Vec<PathBuf>, Error> {
    let (config, command) = match harness {
        "claude" => (
            host.join(".mcp.json"),
            host.join(".claude/commands/holden.md"),
        ),
        "codex" => (
            host.join(".codex/config.toml"),
            host.join(".codex/commands/holden.md"),
        ),
        "droid" => (
            host.join(".factory/mcp.json"),
            host.join(".factory/commands/holden.md"),
        ),
        _ => return Err(refused("harness is claude, codex or droid")),
    };
    let config_bytes = if harness == "codex" {
        // This is Codex's native mcp_servers table shape. JSON string
        // escaping is also valid for the simple command and arg values here.
        let args = relay_args()
            .iter()
            .map(|arg| toml_string(arg))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "[mcp_servers.{MCP_SERVER_NAME}]\ncommand = {}\nargs = [{args}]\n",
            toml_string("holden")
        )
        .into_bytes()
    } else {
        merge_json_server(&config, relay_entry())?
    };
    write_if_changed(&config, &config_bytes, 0o644)?;
    write_if_changed(&command, holden_command().as_bytes(), 0o644)?;
    Ok(vec![config, command])
}

/// Write one supported harness's default machine connection.
pub fn write_harness_config(host: &Path, harness: &str) -> Result<Vec<PathBuf>, Error> {
    write_harness_config_at(host, harness, None)
}

/// Write every supported harness's connection to `root`, or to the default
/// machine private root when `root` is absent.
pub fn write_harness_configs_at(host: &Path, root: Option<&Path>) -> Result<Vec<PathBuf>, Error> {
    let mut written = Vec::new();
    for harness in ["claude", "codex", "droid"] {
        written.extend(write_harness_config_at(host, harness, root)?);
    }
    Ok(written)
}

/// Write connection files for all supported harnesses.
pub fn write_harness_configs(host: &Path) -> Result<Vec<PathBuf>, Error> {
    write_harness_configs_at(host, None)
}
