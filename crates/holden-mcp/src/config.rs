//! The sealed server config.
//!
//! "Sealed" means that a [`ServerConfig`] exists only as the result of
//! validation, and it cannot be changed after that. Its fields are private,
//! and no setter exists. It is parsed from JSON with unknown fields refused.
//! A caller's role comes only from this config: the kernel-reported peer uid
//! is looked up in `roles`.
//!
//! ```json
//! {
//!   "socket_dir": "<dir, created 0700>",
//!   "key_dir": "<FileKeyStore dir>",
//!   "epoch_key": "epoch",
//!   "seal_key": "seal",
//!   "plans_dir": "<dir of *.sealed.json>",
//!   "run_window_ms": 60000,
//!   "roles": [{ "uid": 501, "role": "evaluator" }],
//!   "stdio": { "enabled": false },
//!   "tcp": { "addr": "127.0.0.1", "port": 7411 },
//!   "verdict_key": "<64 hex: the evaluator's Ed25519 public key>",
//!   "ledger": { "db": "<absolute path of the ledger file>", "key": "ledger" },
//!   "authority_bars": {
//!     "block": { "catch_lower_min": 0.9, "false_fail_upper_max": 0.05 },
//!     "recommend": { "catch_lower_min": 0.5, "false_fail_upper_max": 0.25 },
//!     "min_broken_trials": 10, "min_good_trials": 10
//!   }
//! }
//! ```
//!
//! `verdict_key` (slice-4) is optional; without it the `verdict` tool
//! refuses every record. `ledger` (slice-5) is optional: with it, every
//! accepted verdict is appended to the signed ledger (signed with the key
//! of that name in `key_dir`) and the `telemetry` tool reads it.
//! `authority_bars` (slice-5) are the owner-set bars authority is measured
//! against; absent, the reasoned defaults (`holden_ledger::Bars::DEFAULT`)
//! apply. Bars out of range, or a block bar weaker than the recommend bar,
//! refuse the config. `stdio` and `tcp` are optional. Stdio is off unless `enabled` is `true`,
//! and its caller is always the evaluator. TCP binds only `127.0.0.1` or
//! `::1` (standard §2). TCP callers carry no kernel-verified identity, so
//! they are treated as unmapped and may call `brief` only.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use holden_ledger::Bars;
use holden_seal::{EvaluatorPublicKey, KeyName, keystore};
use serde::Deserialize;

/// The file name of the Unix socket inside `socket_dir`.
pub const SOCKET_FILE: &str = "holden-mcp.sock";

/// A caller's role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The builder side. May read the brief and request a nonce commitment,
    /// and may audit a closed run. Never sees parameters before close.
    Implementer,
    /// The sealed evaluator. May begin runs and read parameters during a run.
    Evaluator,
}

impl Role {
    /// The role's wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Implementer => "implementer",
            Self::Evaluator => "evaluator",
        }
    }
}

/// Why a config was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// The JSON did not parse into the config shape.
    Malformed(String),
    /// The config file could not be read.
    Unreadable {
        path: PathBuf,
        kind: std::io::ErrorKind,
    },
    /// The config file is writable by group or world, or owned by another user.
    InsecureFile { path: PathBuf, mode: u32, uid: u32 },
    /// The TCP address is not `127.0.0.1` or `::1`.
    NonLoopbackBind { addr: String },
    /// The TCP port is 0 (a port must be configured).
    PortNotConfigured,
    /// A uid is mapped twice.
    DuplicateUid { uid: u32 },
    /// A key name is not a valid [`KeyName`].
    BadKeyName { name: String },
    /// A path is empty or not absolute.
    RelativePath { field: &'static str },
    /// Stdio was requested but the config does not enable it.
    StdioNotEnabled,
    /// The verdict key is not 64 lowercase hex characters.
    BadVerdictKey,
    /// The authority bars are refused.
    BadBars(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(why) => write!(f, "config refused: malformed: {why}"),
            Self::Unreadable { path, kind } => {
                write!(
                    f,
                    "config refused: cannot read {}: {kind:?}",
                    path.display()
                )
            }
            Self::InsecureFile { path, mode, uid } => write!(
                f,
                "config refused: {} has mode {mode:04o} owned by uid {uid}; it must be owned by this user and not group- or world-writable",
                path.display()
            ),
            Self::NonLoopbackBind { addr } => write!(
                f,
                "config refused: non_loopback_bind: {addr:?} is not 127.0.0.1 or ::1"
            ),
            Self::PortNotConfigured => f.write_str("config refused: tcp port must be non-zero"),
            Self::DuplicateUid { uid } => write!(f, "config refused: uid {uid} mapped twice"),
            Self::BadKeyName { name } => write!(f, "config refused: bad key name {name:?}"),
            Self::RelativePath { field } => {
                write!(f, "config refused: {field} must be an absolute path")
            }
            Self::StdioNotEnabled => {
                f.write_str("config refused: stdio is not enabled in the server config")
            }
            Self::BadVerdictKey => {
                f.write_str("config refused: verdict_key must be 64 lowercase hex characters")
            }
            Self::BadBars(why) => write!(f, "config refused: authority_bars: {why}"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRole {
    uid: u32,
    role: Role,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStdio {
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTcp {
    addr: String,
    port: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    socket_dir: PathBuf,
    key_dir: PathBuf,
    epoch_key: String,
    seal_key: String,
    plans_dir: PathBuf,
    run_window_ms: u64,
    roles: Vec<RawRole>,
    #[serde(default)]
    stdio: Option<RawStdio>,
    #[serde(default)]
    tcp: Option<RawTcp>,
    #[serde(default)]
    verdict_key: Option<String>,
    #[serde(default)]
    ledger: Option<RawLedger>,
    #[serde(default)]
    authority_bars: Option<Bars>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLedger {
    db: PathBuf,
    key: String,
}

/// The ledger a server appends verdicts to and reads telemetry from.
#[derive(Debug, Clone)]
pub struct LedgerConfig {
    /// The ledger file.
    pub db: PathBuf,
    /// The signing key's name in the key store.
    pub key: KeyName,
}

/// A validated, immutable server config.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    socket_dir: PathBuf,
    key_dir: PathBuf,
    epoch_key: KeyName,
    seal_key: KeyName,
    plans_dir: PathBuf,
    run_window_ms: u64,
    roles: BTreeMap<u32, Role>,
    stdio_enabled: bool,
    tcp: Option<SocketAddr>,
    verdict_key: Option<EvaluatorPublicKey>,
    ledger: Option<LedgerConfig>,
    bars: Bars,
}

/// Accept only `127.0.0.1` or `::1` with a non-zero port.
///
/// # Errors
/// [`ConfigError::NonLoopbackBind`] for any other address, including
/// `0.0.0.0`, `::`, other loopback-range addresses and host names;
/// [`ConfigError::PortNotConfigured`] for port 0.
pub fn loopback_bind(addr: &str, port: u16) -> Result<SocketAddr, ConfigError> {
    let refused = || ConfigError::NonLoopbackBind {
        addr: addr.to_owned(),
    };
    let ip: IpAddr = addr.parse().map_err(|_| refused())?;
    let ok = match ip {
        IpAddr::V4(v4) => v4 == Ipv4Addr::LOCALHOST,
        IpAddr::V6(v6) => v6 == Ipv6Addr::LOCALHOST,
    };
    if !ok {
        return Err(refused());
    }
    if port == 0 {
        return Err(ConfigError::PortNotConfigured);
    }
    Ok(SocketAddr::new(ip, port))
}

fn absolute(p: &Path, field: &'static str) -> Result<(), ConfigError> {
    if p.is_absolute() {
        Ok(())
    } else {
        Err(ConfigError::RelativePath { field })
    }
}

impl ServerConfig {
    /// Parse and validate a config from JSON text.
    ///
    /// # Errors
    /// [`ConfigError`] naming the first refused field.
    pub fn from_json(text: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig =
            serde_json::from_str(text).map_err(|e| ConfigError::Malformed(e.to_string()))?;
        absolute(&raw.socket_dir, "socket_dir")?;
        absolute(&raw.key_dir, "key_dir")?;
        absolute(&raw.plans_dir, "plans_dir")?;
        let key = |name: &str| {
            KeyName::new(name).map_err(|_| ConfigError::BadKeyName {
                name: name.to_owned(),
            })
        };
        let epoch_key = key(&raw.epoch_key)?;
        let seal_key = key(&raw.seal_key)?;
        let mut roles = BTreeMap::new();
        for r in raw.roles {
            if roles.insert(r.uid, r.role).is_some() {
                return Err(ConfigError::DuplicateUid { uid: r.uid });
            }
        }
        let tcp = match raw.tcp {
            Some(t) => Some(loopback_bind(&t.addr, t.port)?),
            None => None,
        };
        let verdict_key = match raw.verdict_key {
            None => None,
            Some(k) => {
                if k.len() != 64
                    || !k
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err(ConfigError::BadVerdictKey);
                }
                let bytes: [u8; 32] = hex::decode(&k)
                    .ok()
                    .and_then(|v| v.try_into().ok())
                    .ok_or(ConfigError::BadVerdictKey)?;
                Some(EvaluatorPublicKey(bytes))
            }
        };
        let ledger = match raw.ledger {
            None => None,
            Some(l) => {
                absolute(&l.db, "ledger.db")?;
                Some(LedgerConfig {
                    db: l.db,
                    key: key(&l.key)?,
                })
            }
        };
        let bars = raw.authority_bars.unwrap_or(Bars::DEFAULT);
        bars.validate().map_err(ConfigError::BadBars)?;
        Ok(Self {
            socket_dir: raw.socket_dir,
            key_dir: raw.key_dir,
            epoch_key,
            seal_key,
            plans_dir: raw.plans_dir,
            run_window_ms: raw.run_window_ms,
            roles,
            stdio_enabled: raw.stdio.is_some_and(|s| s.enabled),
            tcp,
            verdict_key,
            ledger,
            bars,
        })
    }

    /// The ledger, if configured.
    #[must_use]
    pub fn ledger(&self) -> Option<&LedgerConfig> {
        self.ledger.as_ref()
    }

    /// The owner-set authority bars (or the reasoned defaults).
    #[must_use]
    pub fn bars(&self) -> &Bars {
        &self.bars
    }

    /// The evaluator's public key, which every verdict record must be
    /// signed by.
    #[must_use]
    pub fn verdict_key(&self) -> Option<EvaluatorPublicKey> {
        self.verdict_key
    }

    /// Load a config file. The file must be owned by the current user and
    /// not writable by group or world.
    ///
    /// # Errors
    /// [`ConfigError`] when the file is refused or its contents are.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let unreadable = |e: std::io::Error| ConfigError::Unreadable {
            path: path.to_path_buf(),
            kind: e.kind(),
        };
        let meta = fs::metadata(path).map_err(unreadable)?;
        let mode = meta.mode() & 0o7777;
        if meta.uid() != keystore::current_uid() || mode & 0o022 != 0 {
            return Err(ConfigError::InsecureFile {
                path: path.to_path_buf(),
                mode,
                uid: meta.uid(),
            });
        }
        let text = fs::read_to_string(path).map_err(unreadable)?;
        Self::from_json(&text)
    }

    /// The role mapped to `uid`, if any.
    #[must_use]
    pub fn role_of(&self, uid: u32) -> Option<Role> {
        self.roles.get(&uid).copied()
    }

    /// The directory that holds the Unix socket.
    #[must_use]
    pub fn socket_dir(&self) -> &Path {
        &self.socket_dir
    }

    /// The Unix socket path.
    #[must_use]
    pub fn socket_path(&self) -> PathBuf {
        self.socket_dir.join(SOCKET_FILE)
    }

    /// The key store directory.
    #[must_use]
    pub fn key_dir(&self) -> &Path {
        &self.key_dir
    }

    /// The epoch (VRF) key name.
    #[must_use]
    pub fn epoch_key(&self) -> &KeyName {
        &self.epoch_key
    }

    /// The plan seal (AEAD) key name.
    #[must_use]
    pub fn seal_key(&self) -> &KeyName {
        &self.seal_key
    }

    /// The directory of sealed plans.
    #[must_use]
    pub fn plans_dir(&self) -> &Path {
        &self.plans_dir
    }

    /// How long a run stays open after `begin_run`, in milliseconds.
    #[must_use]
    pub fn run_window_ms(&self) -> u64 {
        self.run_window_ms
    }

    /// Whether stdio mode is enabled.
    #[must_use]
    pub fn stdio_enabled(&self) -> bool {
        self.stdio_enabled
    }

    /// The validated loopback TCP address, if TCP is configured.
    #[must_use]
    pub fn tcp(&self) -> Option<SocketAddr> {
        self.tcp
    }
}
