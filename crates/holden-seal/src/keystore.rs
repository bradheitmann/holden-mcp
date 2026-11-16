//! Key stores behind one trait (docs/ARCHITECTURE.md §3.2).
//!
//! Slice 1 ships the file backend: one 32-byte key per file, mode 0600,
//! owned by the current user, inside a 0700 directory owned by the same
//! user. Loading refuses a key file or directory that is group- or
//! world-accessible or owned by another user, naming the path and the mode
//! (or owner). Key bytes are never printed: errors carry paths and modes only.
//!
//! The macOS Keychain backend is in `keychain` behind the `keychain` feature;
//! the Linux Secret Service / kernel-keyring backend remains a later half.

use std::fmt;
use std::fs::{self, DirBuilder, File, Metadata, OpenOptions, Permissions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use crate::key::KeyMaterial;

/// Mode of a key file.
pub const KEY_FILE_MODE: u32 = 0o600;

/// Mode of a key directory.
pub const KEY_DIR_MODE: u32 = 0o700;

/// Mode bits that make a path group- or world-accessible.
const GROUP_OR_WORLD: u32 = 0o077;

/// A key name: 1 to 64 characters of `[a-z0-9-]`, so it is always one plain
/// file name inside the store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyName(String);

impl KeyName {
    /// Validate `name`.
    ///
    /// # Errors
    /// [`KeyStoreError::BadName`] for an empty, long or non-`[a-z0-9-]` name.
    pub fn new(name: &str) -> Result<Self, KeyStoreError> {
        let ok = !name.is_empty()
            && name.len() <= 64
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if ok {
            Ok(Self(name.to_owned()))
        } else {
            Err(KeyStoreError::BadName)
        }
    }

    /// The validated key name, for OS key-store account identifiers.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn file_name(&self) -> String {
        format!("{}.key", self.0)
    }
}

/// Why a key store refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyStoreError {
    /// The key name is not `[a-z0-9-]{1,64}`.
    BadName,
    /// An I/O operation failed.
    Io {
        /// The path.
        path: PathBuf,
        /// What failed.
        kind: io::ErrorKind,
    },
    /// The path is owned by another user.
    WrongOwner {
        /// The path.
        path: PathBuf,
        /// Its owner.
        uid: u32,
        /// The current effective user.
        expected: u32,
    },
    /// The path is group- or world-accessible.
    InsecureMode {
        /// The path.
        path: PathBuf,
        /// Its permission bits.
        mode: u32,
    },
    /// The store path is not a directory (a symlink is refused too).
    NotADirectory {
        /// The path.
        path: PathBuf,
    },
    /// The key path is not a regular file (a symlink is refused too).
    NotARegularFile {
        /// The path.
        path: PathBuf,
    },
    /// The file was replaced between the check and the read.
    ChangedDuringLoad {
        /// The path.
        path: PathBuf,
    },
    /// A key of this name already exists; keys are never overwritten.
    AlreadyExists {
        /// The path.
        path: PathBuf,
    },
    /// The OS key store has no item under this name.
    NotFound {
        /// The keychain path, or a default-keychain marker.
        path: PathBuf,
    },
    /// Security.framework refused an operation. The code is public OS status;
    /// no key bytes or item contents enter the error.
    Keychain {
        /// The keychain path, or a default-keychain marker.
        path: PathBuf,
        /// The attempted operation.
        operation: &'static str,
        /// The Security.framework OSStatus.
        code: i32,
    },
    /// The file does not hold exactly 32 bytes.
    BadLength {
        /// The path.
        path: PathBuf,
        /// Its length.
        len: u64,
    },
    /// A passphrase-sealed key requires input from the controlling terminal.
    InteractiveTerminalRequired {
        /// The sealed key path.
        path: PathBuf,
    },
    /// A sealed key is malformed, authenticated decryption failed, or its
    /// passphrase was empty. These cases deliberately share one public error.
    SealedKey {
        /// The sealed key path.
        path: PathBuf,
    },
}

impl fmt::Display for KeyStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadName => f.write_str("refused: key names are [a-z0-9-]{1,64}"),
            Self::Io { path, kind } => write!(f, "refused: {}: {kind}", path.display()),
            Self::WrongOwner {
                path,
                uid,
                expected,
            } => write!(
                f,
                "refused: {} is owned by uid {uid}, not the current user (uid {expected})",
                path.display()
            ),
            Self::InsecureMode { path, mode } => write!(
                f,
                "refused: {} has mode {mode:04o}; it must not be group- or world-accessible \
                 (key files 0600, key directories 0700)",
                path.display()
            ),
            Self::NotADirectory { path } => {
                write!(f, "refused: {} is not a directory", path.display())
            }
            Self::NotARegularFile { path } => {
                write!(f, "refused: {} is not a regular file", path.display())
            }
            Self::ChangedDuringLoad { path } => {
                write!(f, "refused: {} changed while it was loaded", path.display())
            }
            Self::AlreadyExists { path } => write!(
                f,
                "refused: {} exists; keys are never overwritten",
                path.display()
            ),
            Self::NotFound { path } => {
                write!(f, "refused: {} holds no key with this name", path.display())
            }
            Self::Keychain {
                path,
                operation,
                code,
            } => write!(
                f,
                "refused: Keychain operation '{operation}' on {} failed with OSStatus {code}",
                path.display()
            ),
            Self::BadLength { path, len } => {
                write!(f, "refused: {} holds {len} bytes, not 32", path.display())
            }
            Self::InteractiveTerminalRequired { path } => write!(
                f,
                "refused: owner presence required: {} can be unsealed only with a passphrase from an interactive TTY",
                path.display()
            ),
            Self::SealedKey { path } => write!(
                f,
                "refused: {} is malformed or its passphrase could not unseal it",
                path.display()
            ),
        }
    }
}

impl std::error::Error for KeyStoreError {}

/// A place that keeps 32-byte keys by name.
pub trait KeyStore {
    /// Store `key` under `name`. Refuses to overwrite an existing key.
    ///
    /// # Errors
    /// [`KeyStoreError`] when the store refuses or I/O fails.
    fn put(&self, name: &KeyName, key: &KeyMaterial) -> Result<(), KeyStoreError>;

    /// Load the key stored under `name`.
    ///
    /// # Errors
    /// [`KeyStoreError`] when the store refuses or I/O fails.
    fn get(&self, name: &KeyName) -> Result<KeyMaterial, KeyStoreError>;
}

/// The file backend.
#[derive(Debug, Clone)]
pub struct FileKeyStore {
    dir: PathBuf,
}

pub(crate) fn io_err(path: &Path) -> impl FnOnce(io::Error) -> KeyStoreError + '_ {
    move |e| KeyStoreError::Io {
        path: path.to_path_buf(),
        kind: e.kind(),
    }
}

/// The current effective user id (the owner of files this process creates).
#[must_use]
pub fn current_uid() -> u32 {
    rustix::process::geteuid().as_raw()
}

/// Refuse `meta` unless it is owned by the current user and not group- or
/// world-accessible. Owner is checked first.
pub(crate) fn check_owner_and_mode(path: &Path, meta: &Metadata) -> Result<(), KeyStoreError> {
    let expected = current_uid();
    if meta.uid() != expected {
        return Err(KeyStoreError::WrongOwner {
            path: path.to_path_buf(),
            uid: meta.uid(),
            expected,
        });
    }
    let mode = meta.mode() & 0o7777;
    if mode & GROUP_OR_WORLD != 0 {
        return Err(KeyStoreError::InsecureMode {
            path: path.to_path_buf(),
            mode,
        });
    }
    Ok(())
}

pub(crate) fn check_dir(dir: &Path) -> Result<(), KeyStoreError> {
    let meta = fs::symlink_metadata(dir).map_err(io_err(dir))?;
    if !meta.file_type().is_dir() {
        return Err(KeyStoreError::NotADirectory {
            path: dir.to_path_buf(),
        });
    }
    check_owner_and_mode(dir, &meta)
}

impl FileKeyStore {
    /// Open the store at `dir`, creating it with mode 0700 if it is absent.
    /// An existing directory must already be owned by the current user and
    /// not group- or world-accessible.
    ///
    /// # Errors
    /// [`KeyStoreError`] when the directory is refused or cannot be created.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, KeyStoreError> {
        let dir = dir.into();
        match fs::symlink_metadata(&dir) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                DirBuilder::new()
                    .recursive(true)
                    .mode(KEY_DIR_MODE)
                    .create(&dir)
                    .map_err(io_err(&dir))?;
                // The umask may have cleared bits; set the mode exactly.
                fs::set_permissions(&dir, Permissions::from_mode(KEY_DIR_MODE))
                    .map_err(io_err(&dir))?;
            }
            Err(e) => return Err(io_err(&dir)(e)),
        }
        check_dir(&dir)?;
        Ok(Self { dir })
    }

    /// The store directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The file that holds `name`.
    #[must_use]
    pub fn key_path(&self, name: &KeyName) -> PathBuf {
        self.dir.join(name.file_name())
    }
}

impl KeyStore for FileKeyStore {
    fn put(&self, name: &KeyName, key: &KeyMaterial) -> Result<(), KeyStoreError> {
        check_dir(&self.dir)?;
        let path = self.key_path(name);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(KEY_FILE_MODE)
            .open(&path)
            .map_err(|e| {
                if e.kind() == io::ErrorKind::AlreadyExists {
                    KeyStoreError::AlreadyExists { path: path.clone() }
                } else {
                    io_err(&path)(e)
                }
            })?;
        file.set_permissions(Permissions::from_mode(KEY_FILE_MODE))
            .map_err(io_err(&path))?;
        file.write_all(key.expose()).map_err(io_err(&path))?;
        file.sync_all().map_err(io_err(&path))?;
        Ok(())
    }

    fn get(&self, name: &KeyName) -> Result<KeyMaterial, KeyStoreError> {
        check_dir(&self.dir)?;
        let path = self.key_path(name);
        let before = fs::symlink_metadata(&path).map_err(io_err(&path))?;
        if !before.file_type().is_file() {
            return Err(KeyStoreError::NotARegularFile { path });
        }
        check_owner_and_mode(&path, &before)?;
        let mut file = File::open(&path).map_err(io_err(&path))?;
        let opened = file.metadata().map_err(io_err(&path))?;
        if opened.dev() != before.dev() || opened.ino() != before.ino() {
            return Err(KeyStoreError::ChangedDuringLoad { path });
        }
        check_owner_and_mode(&path, &opened)?;
        if opened.len() != 32 {
            return Err(KeyStoreError::BadLength {
                path,
                len: opened.len(),
            });
        }
        let mut buf = Zeroizing::new([0u8; 32]);
        file.read_exact(buf.as_mut()).map_err(io_err(&path))?;
        let mut extra = [0u8; 1];
        if file.read(&mut extra).map_err(io_err(&path))? != 0 {
            return Err(KeyStoreError::ChangedDuringLoad { path });
        }
        Ok(KeyMaterial::from_zeroizing(buf))
    }
}
