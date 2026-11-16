//! Verify a canon or corpus root against its pin, byte for byte.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::digest::Sha256Digest;
use crate::pin::{Pin, PinnedPath};

/// One reason a root does not match its pin. Every finding names its file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Finding {
    /// The file exists but its bytes differ from the pin.
    Differs {
        /// The pinned file.
        file: PinnedPath,
        /// The digest the pin records.
        expected: Sha256Digest,
        /// The digest of the bytes on disk.
        actual: Sha256Digest,
    },
    /// The pin lists the file but the root does not hold it.
    Missing {
        /// The pinned file.
        file: PinnedPath,
    },
    /// The root holds a file the pin does not list.
    Extra {
        /// The unpinned file, relative to the root.
        file: String,
    },
    /// The root holds an entry that is neither a regular file nor a
    /// directory (for example a symlink). It is refused, never followed.
    Unsupported {
        /// The entry, relative to the root.
        file: String,
    },
}

impl Finding {
    /// The file this finding names.
    #[must_use]
    pub fn file(&self) -> &str {
        match self {
            Self::Differs { file, .. } | Self::Missing { file } => file.as_str(),
            Self::Extra { file } | Self::Unsupported { file } => file,
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Differs {
                file,
                expected,
                actual,
            } => write!(f, "differs: {file} (pinned {expected}, found {actual})"),
            Self::Missing { file } => write!(f, "missing: {file}"),
            Self::Extra { file } => write!(f, "extra: {file} (not in the pin)"),
            Self::Unsupported { file } => {
                write!(f, "unsupported: {file} (not a regular file or directory)")
            }
        }
    }
}

/// A verification that did not pass.
#[derive(Debug)]
pub enum CanonError {
    /// Typed limitation `canon_absent`: the root does not exist. This is
    /// never a pass; it says the check could not run.
    CanonAbsent {
        /// The root that was asked for.
        root: PathBuf,
    },
    /// The root exists but is not a directory.
    NotADirectory {
        /// The root that was asked for.
        root: PathBuf,
    },
    /// The root differs from the pin. Every finding names a file.
    Refused(Vec<Finding>),
    /// An I/O error while reading the root.
    Io {
        /// The path being read.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
}

impl CanonError {
    /// The stable machine name of this outcome.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::CanonAbsent { .. } => "canon_absent",
            Self::NotADirectory { .. } => "not_a_directory",
            Self::Refused(_) => "refused",
            Self::Io { .. } => "io_error",
        }
    }

    /// True when the check could not run (a limitation, not a refusal).
    /// A limitation is never a pass.
    #[must_use]
    pub fn is_limitation(&self) -> bool {
        matches!(self, Self::CanonAbsent { .. })
    }
}

impl fmt::Display for CanonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CanonAbsent { root } => write!(
                f,
                "limitation canon_absent: root {} does not exist; nothing was verified",
                root.display()
            ),
            Self::NotADirectory { root } => {
                write!(f, "refused: root {} is not a directory", root.display())
            }
            Self::Refused(findings) => {
                write!(f, "refused: {} finding(s)", findings.len())?;
                for finding in findings {
                    write!(f, "\n  {finding}")?;
                }
                Ok(())
            }
            Self::Io { path, source } => write!(f, "io error at {}: {source}", path.display()),
        }
    }
}

impl std::error::Error for CanonError {}

/// The result of a passing verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyReport {
    /// The verified canon version label.
    pub canon_version: String,
    /// The validator hash carried by the pin, if any.
    pub validator_hash: Option<Sha256Digest>,
    /// Number of files verified.
    pub file_count: usize,
    /// Digest of the canonical listing (see [`Pin::listing_digest`]).
    pub digest: Sha256Digest,
}

/// Recompute the SHA-256 of every file under `root` and compare with `pin`.
///
/// # Errors
/// - [`CanonError::CanonAbsent`] when `root` does not exist (a limitation,
///   never a pass);
/// - [`CanonError::Refused`] naming every differing, missing, extra or
///   unsupported file;
/// - [`CanonError::NotADirectory`] or [`CanonError::Io`] otherwise.
pub fn verify(root: &Path, pin: &Pin) -> Result<VerifyReport, CanonError> {
    let meta = match fs::symlink_metadata(root) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(CanonError::CanonAbsent {
                root: root.to_path_buf(),
            });
        }
        Err(source) => {
            return Err(CanonError::Io {
                path: root.to_path_buf(),
                source,
            });
        }
    };
    if !meta.is_dir() {
        return Err(CanonError::NotADirectory {
            root: root.to_path_buf(),
        });
    }

    let mut on_disk: BTreeMap<String, Sha256Digest> = BTreeMap::new();
    let mut findings: Vec<Finding> = Vec::new();
    walk(root, "", &mut on_disk, &mut findings)?;

    for (path, expected) in &pin.files {
        match on_disk.remove(path.as_str()) {
            None => findings.push(Finding::Missing { file: path.clone() }),
            Some(actual) if actual != *expected => findings.push(Finding::Differs {
                file: path.clone(),
                expected: *expected,
                actual,
            }),
            Some(_) => {}
        }
    }
    for file in on_disk.into_keys() {
        findings.push(Finding::Extra { file });
    }

    if !findings.is_empty() {
        findings.sort_by(|a, b| a.file().cmp(b.file()));
        return Err(CanonError::Refused(findings));
    }
    Ok(VerifyReport {
        canon_version: pin.canon_version.clone(),
        validator_hash: pin.validator_hash,
        file_count: pin.files.len(),
        digest: pin.listing_digest(),
    })
}

fn walk(
    dir: &Path,
    prefix: &str,
    out: &mut BTreeMap<String, Sha256Digest>,
    findings: &mut Vec<Finding>,
) -> Result<(), CanonError> {
    let io_err = |path: &Path| {
        let path = path.to_path_buf();
        move |source| CanonError::Io { path, source }
    };
    let entries = fs::read_dir(dir).map_err(io_err(dir))?;
    for entry in entries {
        let entry = entry.map_err(io_err(dir))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let kind = entry.file_type().map_err(io_err(&path))?;
        if kind.is_dir() {
            walk(&path, &rel, out, findings)?;
        } else if kind.is_file() {
            let bytes = fs::read(&path).map_err(io_err(&path))?;
            out.insert(rel, Sha256Digest::of(&bytes));
        } else {
            findings.push(Finding::Unsupported { file: rel });
        }
    }
    Ok(())
}
