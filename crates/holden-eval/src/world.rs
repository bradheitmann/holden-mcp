//! Per-evaluation worlds (never-again control (d)).
//!
//! Every evaluation gets a fresh world: a directory named by a world id of
//! 128 random bits, created exclusively (an existing directory is refused,
//! never reused). The product may write only inside its world. Two
//! evaluations, even of the same plan and commit at the same moment, never
//! share a world.

use std::fs::{self, DirBuilder};
use std::io;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

use crate::EvalError;

/// A fresh world id: 32 lowercase hex characters from the OS random source.
///
/// # Errors
/// [`EvalError::NoRandomness`] when the random source fails.
pub fn world_id() -> Result<String, EvalError> {
    let bytes = holden_seal::random_bytes::<16>().map_err(|_| EvalError::NoRandomness)?;
    Ok(hex::encode(bytes))
}

/// One evaluation's world.
#[derive(Debug, Clone)]
pub struct World {
    id: String,
    root: PathBuf,
}

/// The subdirectories every world has.
const SUBDIRS: [&str; 6] = ["run", "home", "tmp", "cargo-home", "target", "confine"];

impl World {
    /// Create a new world under `scratch` (created 0700 if absent).
    ///
    /// # Errors
    /// [`EvalError::WorldCollision`] if the directory already exists, or
    /// [`EvalError::Io`].
    pub fn create(scratch: &Path) -> Result<Self, EvalError> {
        let io = |what: &'static str| move |e: io::Error| EvalError::Io(what, e.kind());
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(scratch)
            .map_err(io("scratch directory"))?;
        let id = world_id()?;
        let root = scratch.join(format!("w-{id}"));
        match DirBuilder::new().mode(0o700).create(&root) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                return Err(EvalError::WorldCollision);
            }
            Err(e) => return Err(EvalError::Io("world directory", e.kind())),
        }
        let root = fs::canonicalize(&root).map_err(io("world directory"))?;
        for sub in SUBDIRS {
            DirBuilder::new()
                .mode(0o700)
                .create(root.join(sub))
                .map_err(io("world subdirectory"))?;
        }
        Ok(Self { id, root })
    }

    /// The world id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The world directory (canonical).
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<world>/<sub>`.
    #[must_use]
    pub fn path(&self, sub: &str) -> PathBuf {
        self.root.join(sub)
    }

    /// The environment of every process started in this world: nothing from
    /// the caller, only these values.
    #[must_use]
    pub fn product_env(&self) -> Vec<(String, String)> {
        vec![
            ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
            ("HOME".to_owned(), self.path("home").display().to_string()),
            ("TMPDIR".to_owned(), self.path("tmp").display().to_string()),
            ("LC_ALL".to_owned(), "C".to_owned()),
        ]
    }

    /// Remove the world.
    pub fn remove(&self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// The variable names a product may see; everything else is scrubbed.
pub const PRODUCT_ENV_NAMES: [&str; 4] = ["PATH", "HOME", "TMPDIR", "LC_ALL"];
