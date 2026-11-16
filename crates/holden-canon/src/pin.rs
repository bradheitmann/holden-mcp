//! The pin file format `holden.pin/1`.
//!
//! A pin lists the exact SHA-256 of every file under a canon or corpus root.
//! It is plain text so a human can diff it, and it is parsed into typed values:
//!
//! ```text
//! # comment lines start with '#'; blank lines are ignored
//! format: holden.pin/1
//! canon_version: <free text, required>
//! validator_hash: <64 lowercase hex, optional>
//! <64 lowercase hex>  <relative/path>
//! ```
//!
//! - `format` must be the first non-comment line.
//! - Each key appears at most once and before any file line.
//! - File lines use the `sha256sum` layout: digest, two spaces, path.
//! - Paths are relative, use `/` separators, and contain no `.`, `..` or empty
//!   components. Each path appears once.

use std::collections::BTreeMap;
use std::fmt;

use crate::digest::Sha256Digest;

/// The only pin format this crate reads and writes.
pub const PIN_FORMAT: &str = "holden.pin/1";

/// A validated relative path inside a pinned root.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PinnedPath(String);

impl PinnedPath {
    /// Validate a relative `/`-separated path.
    ///
    /// # Errors
    /// Returns the reason when the path is absolute, empty, has a `.` or `..`
    /// component, contains a backslash or a control character.
    pub fn new(path: &str) -> Result<Self, &'static str> {
        if path.is_empty() {
            return Err("empty path");
        }
        if path.starts_with('/') {
            return Err("absolute path");
        }
        if path.contains('\\') {
            return Err("backslash in path");
        }
        if path.chars().any(char::is_control) {
            return Err("control character in path");
        }
        for part in path.split('/') {
            if part.is_empty() || part == "." || part == ".." {
                return Err("empty, '.' or '..' path component");
            }
        }
        Ok(Self(path.to_owned()))
    }

    /// The path text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PinnedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A parsed pin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    /// The canon or corpus version label.
    pub canon_version: String,
    /// The validator hash recorded with the canon, when the source has one.
    pub validator_hash: Option<Sha256Digest>,
    /// Every pinned file and its digest, ordered by path.
    pub files: BTreeMap<PinnedPath, Sha256Digest>,
}

/// Why a pin could not be parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinParseError {
    /// 1-based line number (0 when the error is about the whole file).
    pub line: usize,
    /// What was wrong.
    pub reason: String,
}

impl fmt::Display for PinParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pin line {}: {}", self.line, self.reason)
    }
}

impl std::error::Error for PinParseError {}

fn err(line: usize, reason: impl Into<String>) -> PinParseError {
    PinParseError {
        line,
        reason: reason.into(),
    }
}

impl Pin {
    /// Parse a pin from its text.
    ///
    /// # Errors
    /// Returns the first line that breaks the format.
    pub fn parse(text: &str) -> Result<Self, PinParseError> {
        let mut seen_format = false;
        let mut canon_version: Option<String> = None;
        let mut validator_hash: Option<Sha256Digest> = None;
        let mut files: BTreeMap<PinnedPath, Sha256Digest> = BTreeMap::new();

        for (idx, raw) in text.lines().enumerate() {
            let n = idx + 1;
            let line = raw.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if !seen_format {
                match line.strip_prefix("format: ") {
                    Some(PIN_FORMAT) => {
                        seen_format = true;
                        continue;
                    }
                    _ => return Err(err(n, format!("first line must be 'format: {PIN_FORMAT}'"))),
                }
            }
            if let Some(v) = line.strip_prefix("canon_version: ") {
                if !files.is_empty() {
                    return Err(err(n, "keys must precede file lines"));
                }
                if canon_version.replace(v.trim().to_owned()).is_some() {
                    return Err(err(n, "duplicate canon_version"));
                }
                continue;
            }
            if let Some(v) = line.strip_prefix("validator_hash: ") {
                if !files.is_empty() {
                    return Err(err(n, "keys must precede file lines"));
                }
                let d: Sha256Digest = v.trim().parse().map_err(|e| err(n, format!("{e}")))?;
                if validator_hash.replace(d).is_some() {
                    return Err(err(n, "duplicate validator_hash"));
                }
                continue;
            }
            let Some((hex, path)) = line.split_once("  ") else {
                return Err(err(n, "expected '<sha256>  <path>' or a known key"));
            };
            let digest: Sha256Digest = hex.parse().map_err(|e| err(n, format!("{e}")))?;
            let path = PinnedPath::new(path).map_err(|e| err(n, e))?;
            if files.insert(path.clone(), digest).is_some() {
                return Err(err(n, format!("duplicate path {path}")));
            }
        }

        if !seen_format {
            return Err(err(0, format!("missing 'format: {PIN_FORMAT}'")));
        }
        let canon_version = canon_version.ok_or_else(|| err(0, "missing canon_version"))?;
        if canon_version.is_empty() {
            return Err(err(0, "empty canon_version"));
        }
        if files.is_empty() {
            return Err(err(0, "a pin must list at least one file"));
        }
        Ok(Self {
            canon_version,
            validator_hash,
            files,
        })
    }

    /// Render the pin in canonical form (keys first, files ordered by path).
    #[must_use]
    pub fn render(&self, header_comment: &str) -> String {
        let mut out = String::new();
        for c in header_comment.lines() {
            out.push_str("# ");
            out.push_str(c);
            out.push('\n');
        }
        out.push_str("format: ");
        out.push_str(PIN_FORMAT);
        out.push('\n');
        out.push_str("canon_version: ");
        out.push_str(&self.canon_version);
        out.push('\n');
        if let Some(v) = &self.validator_hash {
            out.push_str("validator_hash: ");
            out.push_str(&v.to_hex());
            out.push('\n');
        }
        out.push_str(&self.listing());
        out
    }

    /// The canonical file listing (`<hex>  <path>\n` per file, by path).
    #[must_use]
    pub fn listing(&self) -> String {
        let mut out = String::new();
        for (path, digest) in &self.files {
            out.push_str(&digest.to_hex());
            out.push_str("  ");
            out.push_str(path.as_str());
            out.push('\n');
        }
        out
    }

    /// The digest of the canonical file listing. Two roots with the same
    /// listing digest hold byte-identical files at identical paths.
    #[must_use]
    pub fn listing_digest(&self) -> Sha256Digest {
        Sha256Digest::of(self.listing().as_bytes())
    }
}
