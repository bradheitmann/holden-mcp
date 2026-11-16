//! The embedded agent's private state directory: everything sealed lives
//! here, never in the host repository.
//!
//! ```text
//! <state>/
//!   agent.json            the agent's config (AgentConfig)
//!   keys/                 file key store: epoch, seal, draw, evaluator, ledger
//!   lens/                 the lens corpus the plans are drawn from
//!   host-pack.json        the forge pack overlaid with the host's domain
//!   plans/                sealed plans (*.sealed.json)
//!   server/               sealed server configs, one per role (0600)
//!   se/  si/              the evaluator's and the implementer's socket directories
//!   db/ledger.sqlite      the signed ledger
//!   armed.json            nonce commitments made before the commits they open
//!   runs/<run id>/        signed record, audit reveal and trace of each run
//!   worlds/               confined evaluation worlds
//! ```
//!
//! By default it is `<private root>/hosts/<host directory name>`.

use std::path::{Path, PathBuf};

use holden_mcp::Role;
use holden_seal::{FileKeyStore, KeyMaterial, KeyName, KeyStore, KeyStoreError};
use serde::{Deserialize, Serialize};

use crate::util::{io_at, json_bytes, private_dir, read_json, write_if_changed};
use crate::{Error, refused};

/// The key names the agent keeps.
pub const KEYS: [&str; 5] = ["epoch", "seal", "draw", "evaluator", "ledger"];

/// The agent's config (`agent.json`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    /// `holden.agent/1`.
    pub format: String,
    /// The host repository's root (canonical).
    pub host: PathBuf,
    /// The planning document, relative to the host root.
    pub planning: String,
    /// The `cargo` of the toolchain host builds use.
    pub cargo: PathBuf,
    /// The `holden-confine` launcher.
    pub confine_helper: PathBuf,
    /// The owner's key store directory (its published public key is the
    /// only trust for approvals).
    pub owner_store: PathBuf,
    /// SHA-256 of the agent's constitution in the host.
    pub constitution_sha256: String,
    /// How long a run stays open, in milliseconds.
    pub run_window_ms: u64,
}

/// The agent config format id.
pub const AGENT_FORMAT: &str = "holden.agent/1";

/// One nonce commitment, made before the commit it opens a run for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Armed {
    /// The plan commit, hex.
    pub plan_commit: String,
    /// The published server-nonce commitment, hex.
    pub commitment: String,
    /// When the server recorded it.
    pub committed_at_ms: u64,
    /// The host's HEAD when it was made (`none` before the first commit).
    pub head: String,
    /// The run that consumed it, once one did.
    pub consumed_by: Option<String>,
}

/// A state directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    dir: PathBuf,
}

impl State {
    /// The state at `dir`.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
        }
    }

    /// The default state of the host at `host`: `<private root>/hosts/<name>`.
    #[must_use]
    pub fn default_for(host: &Path) -> Option<Self> {
        let name = host.file_name()?;
        Some(Self::new(&crate::private_root()?.join("hosts").join(name)))
    }

    /// The directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// `agent.json`.
    #[must_use]
    pub fn agent_config_path(&self) -> PathBuf {
        self.dir.join("agent.json")
    }

    /// The key store directory.
    #[must_use]
    pub fn keys_dir(&self) -> PathBuf {
        self.dir.join("keys")
    }

    /// The lens corpus.
    #[must_use]
    pub fn lens_dir(&self) -> PathBuf {
        self.dir.join("lens")
    }

    /// The host-domain pack.
    #[must_use]
    pub fn host_pack(&self) -> PathBuf {
        self.dir.join("host-pack.json")
    }

    /// The sealed plans.
    #[must_use]
    pub fn plans_dir(&self) -> PathBuf {
        self.dir.join("plans")
    }

    /// The sealed server config of `role`.
    #[must_use]
    pub fn server_config(&self, role: Role) -> PathBuf {
        self.dir
            .join("server")
            .join(format!("{}.json", role.as_str()))
    }

    /// The socket directory of `role`: each role has its own socket.
    #[must_use]
    pub fn socket_dir(&self, role: Role) -> PathBuf {
        self.dir.join(match role {
            Role::Evaluator => "se",
            Role::Implementer => "si",
        })
    }

    /// The socket of `role`.
    #[must_use]
    pub fn socket(&self, role: Role) -> PathBuf {
        self.socket_dir(role).join(holden_mcp::config::SOCKET_FILE)
    }

    /// The signed ledger.
    #[must_use]
    pub fn ledger_db(&self) -> PathBuf {
        self.dir.join("db").join("ledger.sqlite")
    }

    /// `armed.json`.
    #[must_use]
    pub fn armed_path(&self) -> PathBuf {
        self.dir.join("armed.json")
    }

    /// The runs directory.
    #[must_use]
    pub fn runs_dir(&self) -> PathBuf {
        self.dir.join("runs")
    }

    /// One run's directory.
    #[must_use]
    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.runs_dir().join(run_id)
    }

    /// The worlds directory.
    #[must_use]
    pub fn worlds_dir(&self) -> PathBuf {
        self.dir.join("worlds")
    }

    /// The HOME git runs with.
    #[must_use]
    pub fn git_home(&self) -> PathBuf {
        self.dir.join("git-home")
    }

    /// Scratch space for the forge.
    #[must_use]
    pub fn forge_work(&self) -> PathBuf {
        self.dir.join("forge")
    }

    /// The key store, created 0700 if absent.
    ///
    /// # Errors
    /// [`Error::Refused`] when the store refuses.
    pub fn key_store(&self) -> Result<FileKeyStore, Error> {
        FileKeyStore::open(self.keys_dir()).map_err(|e| refused(e.to_string()))
    }

    /// Make every agent key that does not exist yet from the OS random
    /// source. Existing keys are never replaced. Returns the names created.
    ///
    /// # Errors
    /// [`Error::Refused`] when the store refuses or randomness fails.
    pub fn ensure_keys(&self) -> Result<Vec<&'static str>, Error> {
        let store = self.key_store()?;
        let mut made = Vec::new();
        for name in KEYS {
            let key = KeyName::new(name).map_err(|e| refused(e.to_string()))?;
            match store.get(&key) {
                Ok(_) => {}
                Err(KeyStoreError::Io {
                    kind: std::io::ErrorKind::NotFound,
                    ..
                }) => {
                    let bytes = holden_seal::random_bytes::<32>()
                        .map_err(|_| refused("the OS random source failed"))?;
                    store
                        .put(&key, &KeyMaterial::from_bytes(bytes))
                        .map_err(|e| refused(e.to_string()))?;
                    made.push(name);
                }
                Err(e) => return Err(refused(e.to_string())),
            }
        }
        Ok(made)
    }

    /// Load the key named `name`.
    ///
    /// # Errors
    /// [`Error::Refused`] when the store refuses.
    pub fn key(&self, name: &str) -> Result<KeyMaterial, Error> {
        let key = KeyName::new(name).map_err(|e| refused(e.to_string()))?;
        self.key_store()?
            .get(&key)
            .map_err(|e| refused(e.to_string()))
    }

    /// Create the state's directories (0700).
    ///
    /// # Errors
    /// [`Error::Io`].
    pub fn create_dirs(&self) -> Result<(), Error> {
        for d in [
            self.dir.clone(),
            self.plans_dir(),
            self.dir.join("server"),
            self.runs_dir(),
            self.worlds_dir(),
            self.git_home(),
            self.dir.join("db"),
        ] {
            private_dir(&d)?;
        }
        Ok(())
    }

    /// Read `agent.json`.
    ///
    /// # Errors
    /// [`Error`] when it is absent or malformed.
    pub fn agent_config(&self) -> Result<AgentConfig, Error> {
        let c: AgentConfig = read_json(&self.agent_config_path())?;
        if c.format != AGENT_FORMAT {
            return Err(refused("agent.json has an unknown format"));
        }
        Ok(c)
    }

    /// Write `agent.json` (0600).
    ///
    /// # Errors
    /// [`Error::Io`].
    pub fn write_agent_config(&self, c: &AgentConfig) -> Result<(), Error> {
        write_if_changed(&self.agent_config_path(), &json_bytes(c)?, 0o600).map(|_| ())
    }

    /// The armed commitments.
    ///
    /// # Errors
    /// [`Error`] when the file is malformed.
    pub fn armed(&self) -> Result<Vec<Armed>, Error> {
        let path = self.armed_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        read_json(&path)
    }

    /// Replace the armed commitments.
    ///
    /// # Errors
    /// [`Error::Io`].
    pub fn write_armed(&self, armed: &[Armed]) -> Result<(), Error> {
        write_if_changed(&self.armed_path(), &json_bytes(&armed)?, 0o600).map(|_| ())
    }

    /// Every sealed plan file, sorted.
    ///
    /// # Errors
    /// [`Error::Io`].
    pub fn sealed_plan_files(&self) -> Result<Vec<PathBuf>, Error> {
        let dir = self.plans_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut out: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map_err(io_at(&dir))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.ends_with(".sealed.json"))
            })
            .collect();
        out.sort();
        Ok(out)
    }
}
