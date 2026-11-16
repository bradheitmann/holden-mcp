//! The recorded response of S-CANON-MCP `check_canon_lock`.
//!
//! The repository keeps that response verbatim under `docs/evidence/`. This
//! module parses it into typed values so the committed pin can be compared
//! with it field by field.

use std::collections::BTreeMap;

use crate::digest::Sha256Digest;
use crate::pin::{Pin, PinParseError, PinnedPath};

/// The typed content of a recorded `check_canon_lock` response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedLockRecord {
    /// `canon_source` as served.
    pub canon_source: String,
    /// `canon_version` as served.
    pub canon_version: String,
    /// The served file hashes, by file name.
    pub files: BTreeMap<PinnedPath, Sha256Digest>,
    /// The served validator hash.
    pub validator_hash: Sha256Digest,
}

fn err(line: usize, reason: impl Into<String>) -> PinParseError {
    PinParseError {
        line,
        reason: reason.into(),
    }
}

impl HostedLockRecord {
    /// Parse the recorded response.
    ///
    /// # Errors
    /// Returns the first line that does not fit the recorded layout.
    pub fn parse(text: &str) -> Result<Self, PinParseError> {
        let mut canon_source = None;
        let mut canon_version = None;
        let mut validator_hash = None;
        let mut files = BTreeMap::new();
        let mut in_hashes = false;
        for (idx, raw) in text.lines().enumerate() {
            let n = idx + 1;
            let line = raw.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(item) = line.strip_prefix("  - ") {
                if !in_hashes {
                    return Err(err(n, "hash item outside 'hashes:'"));
                }
                let (hex, name) = item
                    .split_once("  ")
                    .ok_or_else(|| err(n, "expected '<sha256>  <file>'"))?;
                let d: Sha256Digest = hex.parse().map_err(|e| err(n, format!("{e}")))?;
                let p = PinnedPath::new(name).map_err(|e| err(n, e))?;
                if files.insert(p, d).is_some() {
                    return Err(err(n, "duplicate file"));
                }
                continue;
            }
            in_hashes = false;
            if line == "hashes:" {
                in_hashes = true;
            } else if let Some(v) = line.strip_prefix("canon_source: ") {
                canon_source = Some(v.to_owned());
            } else if let Some(v) = line.strip_prefix("canon_version: ") {
                canon_version = Some(v.to_owned());
            } else if let Some(v) = line.strip_prefix("validator_hash: ") {
                validator_hash = Some(v.parse().map_err(|e| err(n, format!("{e}")))?);
            } else {
                return Err(err(n, "unknown line"));
            }
        }
        Ok(Self {
            canon_source: canon_source.ok_or_else(|| err(0, "missing canon_source"))?,
            canon_version: canon_version.ok_or_else(|| err(0, "missing canon_version"))?,
            files,
            validator_hash: validator_hash.ok_or_else(|| err(0, "missing validator_hash"))?,
        })
    }

    /// Field-by-field differences between this record and a pin. Empty means
    /// the pin holds exactly the served version, file hashes and validator hash.
    #[must_use]
    pub fn differences(&self, pin: &Pin) -> Vec<String> {
        let mut out = Vec::new();
        if pin.canon_version != self.canon_version {
            out.push(format!(
                "canon_version: pin '{}' vs hosted '{}'",
                pin.canon_version, self.canon_version
            ));
        }
        if pin.validator_hash != Some(self.validator_hash) {
            out.push("validator_hash differs".to_owned());
        }
        for (file, hosted) in &self.files {
            match pin.files.get(file) {
                None => out.push(format!("{file}: absent from pin")),
                Some(p) if p != hosted => out.push(format!("{file}: hash differs")),
                Some(_) => {}
            }
        }
        for file in pin.files.keys() {
            if !self.files.contains_key(file) {
                out.push(format!("{file}: not served by the hosted lock"));
            }
        }
        out
    }
}
