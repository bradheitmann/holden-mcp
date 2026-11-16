//! Verbatim promises (docs/ARCHITECTURE.md §3.1 step 1).
//!
//! A promise is a span of a planning document: byte offsets plus the
//! SHA-256 of the whole source file. Its text is always sliced from the
//! source bytes, never typed in, so a promise cannot be paraphrased. A
//! quote is accepted only when its bytes equal the source bytes at its
//! offsets exactly: no trimming, no whitespace or case normalisation.
//!
//! Extraction convention: a planning-document line that starts with
//! [`PROMISE_MARKER`] (optionally after a Markdown list bullet `- `) states
//! one promise; the promise is the rest of that line, without the line end.

use std::fmt;
use std::fs;
use std::path::Path;

use holden_canon::Sha256Digest;
use serde::{Deserialize, Serialize};

/// A content-hash id.
pub type ContentId = Sha256Digest;

/// The marker that opens a promise line.
pub const PROMISE_MARKER: &str = "PROMISE: ";

/// A planning document: a label (never an absolute path) and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanningDocument {
    /// A relative label for the document, such as `PLANNING.md`.
    pub name: String,
    /// The exact bytes.
    pub bytes: Vec<u8>,
}

impl PlanningDocument {
    /// Read `path`, labelled `name`.
    ///
    /// # Errors
    /// The I/O error.
    pub fn read(path: &Path, name: &str) -> std::io::Result<Self> {
        Ok(Self {
            name: name.to_owned(),
            bytes: fs::read(path)?,
        })
    }

    /// SHA-256 of the whole source file.
    #[must_use]
    pub fn sha256(&self) -> Sha256Digest {
        Sha256Digest::of(&self.bytes)
    }
}

/// A verbatim promise.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Promise {
    /// `H(canonical bytes)` of the fields below.
    pub id: ContentId,
    /// SHA-256 of the source file.
    pub source: Sha256Digest,
    /// Start byte offset (inclusive).
    pub start: u64,
    /// End byte offset (exclusive).
    pub end: u64,
    /// The source bytes at `start..end`, as UTF-8.
    pub text: String,
}

/// Why a promise was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromiseError {
    /// The offsets fall outside the source or are reversed.
    OutOfRange {
        /// Start.
        start: u64,
        /// End.
        end: u64,
        /// Source length.
        len: u64,
    },
    /// The span is empty.
    Empty,
    /// The span is not UTF-8 (or splits a character).
    NotUtf8,
    /// The quoted bytes are not the source bytes at those offsets.
    QuoteDiffers {
        /// Start.
        start: u64,
        /// End.
        end: u64,
    },
    /// The promise was taken from a different source file.
    SourceDiffers {
        /// The hash the promise records.
        recorded: Sha256Digest,
        /// The hash of the document it was checked against.
        actual: Sha256Digest,
    },
    /// The id is not the content hash of the promise.
    IdNotContentHash,
}

impl fmt::Display for PromiseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange { start, end, len } => write!(
                f,
                "refused: promise span {start}..{end} is outside the source ({len} bytes)"
            ),
            Self::Empty => f.write_str("refused: an empty span is not a promise"),
            Self::NotUtf8 => f.write_str("refused: the promise span is not UTF-8"),
            Self::QuoteDiffers { start, end } => write!(
                f,
                "refused: the quoted promise is not the source bytes at {start}..{end}"
            ),
            Self::SourceDiffers { recorded, actual } => write!(
                f,
                "refused: the promise quotes source {recorded}, not this document ({actual})"
            ),
            Self::IdNotContentHash => f.write_str("refused: promise id is not its content hash"),
        }
    }
}

impl std::error::Error for PromiseError {}

/// Compute a promise id.
#[must_use]
pub fn promise_id(source: &Sha256Digest, start: u64, end: u64, text: &str) -> ContentId {
    let mut b = Vec::with_capacity(64 + text.len());
    let tag = b"holden.promise/1";
    b.extend_from_slice(&(tag.len() as u64).to_be_bytes());
    b.extend_from_slice(tag);
    b.extend_from_slice(source.as_bytes());
    b.extend_from_slice(&start.to_be_bytes());
    b.extend_from_slice(&end.to_be_bytes());
    b.extend_from_slice(&(text.len() as u64).to_be_bytes());
    b.extend_from_slice(text.as_bytes());
    Sha256Digest::of(&b)
}

fn span(doc: &PlanningDocument, start: u64, end: u64) -> Result<&[u8], PromiseError> {
    let len = doc.bytes.len() as u64;
    if start > end || end > len {
        return Err(PromiseError::OutOfRange { start, end, len });
    }
    if start == end {
        return Err(PromiseError::Empty);
    }
    let (Ok(s), Ok(e)) = (usize::try_from(start), usize::try_from(end)) else {
        return Err(PromiseError::OutOfRange { start, end, len });
    };
    Ok(&doc.bytes[s..e])
}

/// Exact byte equality: the only way a quote matches its source.
fn quote_matches(source: &[u8], quoted: &[u8]) -> bool {
    source == quoted
}

impl Promise {
    /// The promise at `start..end` of `doc`, its text sliced from the bytes.
    ///
    /// # Errors
    /// [`PromiseError`] for a bad span.
    pub fn slice(doc: &PlanningDocument, start: u64, end: u64) -> Result<Self, PromiseError> {
        let bytes = span(doc, start, end)?;
        let text = std::str::from_utf8(bytes)
            .map_err(|_| PromiseError::NotUtf8)?
            .to_owned();
        let source = doc.sha256();
        Ok(Self {
            id: promise_id(&source, start, end, &text),
            source,
            start,
            end,
            text,
        })
    }

    /// Accept `quoted` as the promise at `start..end` only if it is exactly
    /// the source bytes there.
    ///
    /// # Errors
    /// [`PromiseError::QuoteDiffers`] for any other text, or a span error.
    pub fn from_quote(
        doc: &PlanningDocument,
        start: u64,
        end: u64,
        quoted: &str,
    ) -> Result<Self, PromiseError> {
        let bytes = span(doc, start, end)?;
        if !quote_matches(bytes, quoted.as_bytes()) {
            return Err(PromiseError::QuoteDiffers { start, end });
        }
        Self::slice(doc, start, end)
    }

    /// Check a stored promise against its source document: same file hash,
    /// offsets in range, text equal to the source bytes, id equal to the
    /// content hash.
    ///
    /// # Errors
    /// The first [`PromiseError`] found.
    pub fn verify_against(&self, doc: &PlanningDocument) -> Result<(), PromiseError> {
        let actual = doc.sha256();
        if actual != self.source {
            return Err(PromiseError::SourceDiffers {
                recorded: self.source,
                actual,
            });
        }
        let bytes = span(doc, self.start, self.end)?;
        if !quote_matches(bytes, self.text.as_bytes()) {
            return Err(PromiseError::QuoteDiffers {
                start: self.start,
                end: self.end,
            });
        }
        if promise_id(&self.source, self.start, self.end, &self.text) != self.id {
            return Err(PromiseError::IdNotContentHash);
        }
        Ok(())
    }
}

/// Extract every promise line of `doc` (see the module docs).
///
/// # Errors
/// [`PromiseError`] when a promise line has an empty or non-UTF-8 promise.
pub fn extract_promises(doc: &PlanningDocument) -> Result<Vec<Promise>, PromiseError> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    for line in doc.bytes.split_inclusive(|b| *b == b'\n') {
        let bare = line.strip_suffix(b"\n").unwrap_or(line);
        let bare = bare.strip_suffix(b"\r").unwrap_or(bare);
        let lead = if bare.starts_with(b"- ") { 2 } else { 0 };
        if bare[lead..].starts_with(PROMISE_MARKER.as_bytes()) {
            let start = offset + lead + PROMISE_MARKER.len();
            let end = offset + bare.len();
            out.push(Promise::slice(doc, start as u64, end as u64)?);
        }
        offset += line.len();
    }
    Ok(out)
}
