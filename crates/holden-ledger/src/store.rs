//! The append-only, hash-chained, Ed25519-signed SQLite ledger (slice-5 P1).
//!
//! Each record `n` stores its kind, its JSON body, the previous record's
//! hash, the signer's Ed25519 public key, a signature and its own hash:
//!
//! ```text
//! hash_n = SHA-256("holden.ledger/1\n" ‖ n (u64 BE) ‖ hash_{n-1} ‖
//!                  len(kind) (u64 BE) ‖ kind ‖ len(body) (u64 BE) ‖ body ‖ signer)
//! signature_n = Ed25519(signer key, hash_n)
//! ```
//!
//! with `hash_0` all zeros. Editing a record changes its hash; deleting or
//! reordering records breaks the sequence and the chain; splicing in a
//! record signed under the same key from another copy of the ledger breaks
//! the chain at the next record. [`Ledger::verify`] walks the whole chain
//! and names every record that does not hold.
//!
//! [`Ledger::append`] does not re-verify the chain: inside one
//! `BEGIN IMMEDIATE` transaction it checks only the tip (its hash, its
//! signature, and that it was signed by the appending key), then links the
//! new record to it. SQLite triggers refuse `UPDATE` and `DELETE` on the
//! records table, so the ledger is append-only through SQLite; a writer who
//! edits the file directly is caught by `verify`. Truncating the newest
//! records is caught only against a tip recorded elsewhere
//! ([`TipAnchor`]).

use std::fmt;
use std::path::{Path, PathBuf};

use holden_seal::{EvaluatorKey, EvaluatorPublicKey, EvaluatorSignature};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The ledger format, also the hash domain.
pub const LEDGER_FORMAT: &str = "holden.ledger/1";

/// The previous hash of the first record.
pub const GENESIS: [u8; 32] = [0; 32];

/// The ledger file name inside the private root's `db/`.
pub const LEDGER_FILE: &str = "ledger.sqlite";

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS ledger_meta (
    k TEXT PRIMARY KEY NOT NULL,
    v TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS ledger_records (
    seq INTEGER PRIMARY KEY NOT NULL,
    prev_hash TEXT NOT NULL,
    kind TEXT NOT NULL,
    body TEXT NOT NULL,
    signer TEXT NOT NULL,
    signature TEXT NOT NULL,
    hash TEXT NOT NULL
) STRICT;
CREATE TRIGGER IF NOT EXISTS ledger_records_no_update
    BEFORE UPDATE ON ledger_records
    BEGIN SELECT RAISE(ABORT, 'the holden ledger is append-only'); END;
CREATE TRIGGER IF NOT EXISTS ledger_records_no_delete
    BEFORE DELETE ON ledger_records
    BEGIN SELECT RAISE(ABORT, 'the holden ledger is append-only'); END;
";

/// The default ledger path:
/// `${AGENT_PRIVATE_ROOT:-${XDG_DATA_HOME:-$HOME/.local/share}/agent-private}/holden-mcp/db/ledger.sqlite`.
#[must_use]
pub fn default_path() -> Option<PathBuf> {
    let root = if let Some(r) = std::env::var_os("AGENT_PRIVATE_ROOT").filter(|v| !v.is_empty()) {
        PathBuf::from(r).join("holden-mcp")
    } else {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .filter(|p| p.is_absolute())
                    .map(|h| h.join(".local").join("share"))
            })?;
        data.join("agent-private").join("holden-mcp")
    };
    Some(root.join("db").join(LEDGER_FILE))
}

/// Why the ledger refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    /// SQLite failed.
    Sqlite(String),
    /// The database is not a holden ledger of this format.
    Format(String),
    /// The tip does not hold, so nothing may be appended after it.
    TipRefused {
        /// The tip's sequence number.
        seq: u64,
        /// What does not hold.
        why: &'static str,
    },
    /// A body could not be encoded or decoded.
    Body(String),
    /// A directory could not be created.
    Io(String),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "ledger: sqlite: {e}"),
            Self::Format(e) => write!(f, "ledger: not a {LEDGER_FORMAT} ledger: {e}"),
            Self::TipRefused { seq, why } => {
                write!(f, "ledger: append refused: the tip (record {seq}) {why}")
            }
            Self::Body(e) => write!(f, "ledger: body: {e}"),
            Self::Io(e) => write!(f, "ledger: {e}"),
        }
    }
}

impl std::error::Error for LedgerError {}

fn sql(e: rusqlite::Error) -> LedgerError {
    LedgerError::Sqlite(e.to_string())
}

/// One stored record, as read (not yet verified).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Sequence number, from 1.
    pub seq: u64,
    /// The previous record's hash, hex.
    pub prev_hash: String,
    /// The record kind.
    pub kind: String,
    /// The JSON body.
    pub body: String,
    /// The signer's Ed25519 public key, hex.
    pub signer: String,
    /// The Ed25519 signature over the hash, hex.
    pub signature: String,
    /// The record hash, hex.
    pub hash: String,
}

impl Entry {
    /// Decode the body as `T`.
    ///
    /// # Errors
    /// [`LedgerError::Body`] when it does not decode.
    pub fn decode<T: serde::de::DeserializeOwned>(&self) -> Result<T, LedgerError> {
        serde_json::from_str(&self.body).map_err(|e| LedgerError::Body(e.to_string()))
    }
}

/// The hash of a record from its parts.
#[must_use]
pub fn record_hash(
    seq: u64,
    prev: &[u8; 32],
    kind: &str,
    body: &str,
    signer: &[u8; 32],
) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(LEDGER_FORMAT.as_bytes());
    h.update(b"\n");
    h.update(seq.to_be_bytes());
    h.update(prev);
    h.update((kind.len() as u64).to_be_bytes());
    h.update(kind.as_bytes());
    h.update((body.len() as u64).to_be_bytes());
    h.update(body.as_bytes());
    h.update(signer);
    h.finalize().into()
}

fn unhex<const N: usize>(s: &str) -> Option<[u8; N]> {
    hex::decode(s).ok().and_then(|v| v.try_into().ok())
}

/// What is wrong with one record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "problem", rename_all = "snake_case")]
pub enum Problem {
    /// Records are missing before this one (deleted).
    Missing {
        /// The first missing sequence number.
        from: u64,
        /// The last missing sequence number.
        to: u64,
    },
    /// Its contents do not hash to its stored hash (edited or moved).
    Edited,
    /// Its previous hash is not the hash of the record before it
    /// (reordered, deleted before, or spliced in).
    Unlinked,
    /// Its signature does not verify.
    BadSignature,
    /// It was signed by another key than the ledger's.
    OtherSigner,
    /// A field is not well-formed hex.
    Malformed,
    /// The recorded tip is gone or changed (truncated or rewritten).
    TipMissing,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { from, to } if from == to => write!(f, "record {from} is missing"),
            Self::Missing { from, to } => write!(f, "records {from} to {to} are missing"),
            Self::Edited => f.write_str("edited: its contents do not match its hash"),
            Self::Unlinked => f.write_str("unlinked: it does not follow the record before it"),
            Self::BadSignature => f.write_str("its signature does not verify"),
            Self::OtherSigner => f.write_str("signed by another key than the ledger's"),
            Self::Malformed => f.write_str("a field is malformed"),
            Self::TipMissing => f.write_str("the recorded tip is not in the ledger"),
        }
    }
}

/// One finding of [`Ledger::verify`], naming the record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// The record's sequence number.
    pub seq: u64,
    /// The record's kind (empty when the record is gone).
    pub kind: String,
    /// What is wrong.
    pub problem: Problem,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.kind.is_empty() {
            write!(f, "record {}: {}", self.seq, self.problem)
        } else {
            write!(f, "record {} ({}): {}", self.seq, self.kind, self.problem)
        }
    }
}

/// A tip recorded outside the ledger, to catch truncation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TipAnchor {
    /// The tip's sequence number.
    pub seq: u64,
    /// The tip's hash, hex.
    pub hash: String,
}

impl TipAnchor {
    /// Parse `<seq>:<hash>`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let (seq, hash) = s.split_once(':')?;
        let seq = seq.parse().ok()?;
        unhex::<32>(hash)?;
        Some(Self {
            seq,
            hash: hash.to_owned(),
        })
    }
}

/// The result of a full verification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyReport {
    /// How many records were read.
    pub records: u64,
    /// The tip's sequence number and hash (`<seq>:<hash>`), if any record.
    pub tip: Option<String>,
    /// The key the ledger is verified under, hex.
    pub signer: Option<String>,
    /// Whether that key was pinned by the caller (otherwise it is the first
    /// record's signer).
    pub signer_pinned: bool,
    /// Every finding, in record order.
    pub findings: Vec<Finding>,
}

impl VerifyReport {
    /// Whether every record holds.
    #[must_use]
    pub fn holds(&self) -> bool {
        self.findings.is_empty()
    }
}

/// An open ledger.
#[derive(Debug)]
pub struct Ledger {
    conn: Connection,
    path: PathBuf,
}

const COLUMNS: &str = "seq, prev_hash, kind, body, signer, signature, hash";

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Entry> {
    let seq: i64 = r.get(0)?;
    Ok(Entry {
        seq: u64::try_from(seq).unwrap_or(0),
        prev_hash: r.get(1)?,
        kind: r.get(2)?,
        body: r.get(3)?,
        signer: r.get(4)?,
        signature: r.get(5)?,
        hash: r.get(6)?,
    })
}

impl Ledger {
    /// Open (creating if absent) the ledger at `path`; its directory is
    /// created 0700 if absent.
    ///
    /// # Errors
    /// [`LedgerError`] when SQLite fails or the file is another format.
    pub fn open(path: &Path) -> Result<Self, LedgerError> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            use std::os::unix::fs::DirBuilderExt;
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(dir)
                .map_err(|e| LedgerError::Io(format!("{}: {e}", dir.display())))?;
        }
        let conn = Connection::open(path).map_err(sql)?;
        conn.busy_timeout(std::time::Duration::from_secs(30))
            .map_err(sql)?;
        conn.execute_batch(SCHEMA).map_err(sql)?;
        let format: Option<String> = conn
            .query_row("SELECT v FROM ledger_meta WHERE k = 'format'", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(sql)?;
        match format {
            Some(f) if f == LEDGER_FORMAT => {}
            Some(f) => return Err(LedgerError::Format(f)),
            None => {
                conn.execute(
                    "INSERT OR IGNORE INTO ledger_meta (k, v) VALUES ('format', ?1)",
                    params![LEDGER_FORMAT],
                )
                .map_err(sql)?;
            }
        }
        Ok(Self {
            conn,
            path: path.to_path_buf(),
        })
    }

    /// The file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The tip, read without verification.
    ///
    /// # Errors
    /// [`LedgerError::Sqlite`].
    pub fn tip(&self) -> Result<Option<Entry>, LedgerError> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM ledger_records ORDER BY seq DESC LIMIT 1"),
                [],
                row,
            )
            .optional()
            .map_err(sql)
    }

    /// Check the tip only: its hash, its signature, and that `signer` signed
    /// it. Returns its sequence number and hash.
    fn check_tip(tip: &Entry, signer: &EvaluatorPublicKey) -> Result<(u64, [u8; 32]), LedgerError> {
        let refuse = |why| LedgerError::TipRefused { seq: tip.seq, why };
        let (Some(prev), Some(stored), Some(key), Some(sig)) = (
            unhex::<32>(&tip.prev_hash),
            unhex::<32>(&tip.hash),
            unhex::<32>(&tip.signer),
            unhex::<64>(&tip.signature),
        ) else {
            return Err(refuse("is malformed"));
        };
        if record_hash(tip.seq, &prev, &tip.kind, &tip.body, &key) != stored {
            return Err(refuse("does not match its hash"));
        }
        if key != signer.0 {
            return Err(refuse("was signed by another key"));
        }
        EvaluatorPublicKey(key)
            .verify(&stored, &EvaluatorSignature(sig))
            .map_err(|_| refuse("has a signature that does not verify"))?;
        Ok((tip.seq, stored))
    }

    /// Append a record of `kind` with `body`, signed by `key`. Only the tip
    /// is checked (see the module docs); the whole chain is not re-read.
    ///
    /// # Errors
    /// [`LedgerError::TipRefused`] when the tip does not hold;
    /// [`LedgerError::Sqlite`].
    pub fn append(
        &mut self,
        key: &EvaluatorKey,
        kind: &str,
        body: &str,
    ) -> Result<Entry, LedgerError> {
        let signer = key.public();
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(sql)?;
        let tip = tx
            .query_row(
                &format!("SELECT {COLUMNS} FROM ledger_records ORDER BY seq DESC LIMIT 1"),
                [],
                row,
            )
            .optional()
            .map_err(sql)?;
        let (seq, prev) = match &tip {
            Some(t) => {
                let (seq, hash) = Self::check_tip(t, &signer)?;
                (seq + 1, hash)
            }
            None => (1, GENESIS),
        };
        let hash = record_hash(seq, &prev, kind, body, &signer.0);
        let signature = key.sign(&hash);
        let entry = Entry {
            seq,
            prev_hash: hex::encode(prev),
            kind: kind.to_owned(),
            body: body.to_owned(),
            signer: hex::encode(signer.0),
            signature: hex::encode(signature.0),
            hash: hex::encode(hash),
        };
        tx.execute(
            &format!("INSERT INTO ledger_records ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"),
            params![
                i64::try_from(entry.seq).map_err(|e| LedgerError::Body(e.to_string()))?,
                entry.prev_hash,
                entry.kind,
                entry.body,
                entry.signer,
                entry.signature,
                entry.hash
            ],
        )
        .map_err(sql)?;
        tx.commit().map_err(sql)?;
        Ok(entry)
    }

    /// Append `body` encoded as JSON.
    ///
    /// # Errors
    /// As [`Ledger::append`], or [`LedgerError::Body`].
    pub fn append_json<T: Serialize>(
        &mut self,
        key: &EvaluatorKey,
        kind: &str,
        body: &T,
    ) -> Result<Entry, LedgerError> {
        let text = serde_json::to_string(body).map_err(|e| LedgerError::Body(e.to_string()))?;
        self.append(key, kind, &text)
    }

    /// Every record, in sequence order, read without verification.
    ///
    /// # Errors
    /// [`LedgerError::Sqlite`].
    pub fn entries(&self) -> Result<Vec<Entry>, LedgerError> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM ledger_records ORDER BY seq"
            ))
            .map_err(sql)?;
        let rows = stmt.query_map([], row).map_err(sql)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql)
    }

    /// Verify the whole chain. The signer is `pinned` when given, otherwise
    /// the first record's signer. `anchor`, when given, must still be in the
    /// ledger (catches truncation).
    ///
    /// # Errors
    /// [`LedgerError::Sqlite`] when the records cannot be read.
    pub fn verify(
        &self,
        pinned: Option<&EvaluatorPublicKey>,
        anchor: Option<&TipAnchor>,
    ) -> Result<VerifyReport, LedgerError> {
        Ok(verify_entries(&self.entries()?, pinned, anchor))
    }
}

/// Verify `entries` (in stored sequence order) as a chain.
#[must_use]
pub fn verify_entries(
    entries: &[Entry],
    pinned: Option<&EvaluatorPublicKey>,
    anchor: Option<&TipAnchor>,
) -> VerifyReport {
    let mut findings = Vec::new();
    let signer: Option<[u8; 32]> = pinned
        .map(|p| p.0)
        .or_else(|| entries.first().and_then(|e| unhex::<32>(&e.signer)));
    let mut expected = 1u64;
    let mut prev = GENESIS;
    for e in entries {
        if e.seq > expected {
            findings.push(Finding {
                seq: expected,
                kind: String::new(),
                problem: Problem::Missing {
                    from: expected,
                    to: e.seq - 1,
                },
            });
        }
        let mut push = |problem| {
            findings.push(Finding {
                seq: e.seq,
                kind: e.kind.clone(),
                problem,
            });
        };
        let (Some(eprev), Some(stored), Some(key), Some(sig)) = (
            unhex::<32>(&e.prev_hash),
            unhex::<32>(&e.hash),
            unhex::<32>(&e.signer),
            unhex::<64>(&e.signature),
        ) else {
            push(Problem::Malformed);
            expected = e.seq + 1;
            continue;
        };
        if record_hash(e.seq, &eprev, &e.kind, &e.body, &key) == stored {
            if EvaluatorPublicKey(key)
                .verify(&stored, &EvaluatorSignature(sig))
                .is_err()
            {
                push(Problem::BadSignature);
            }
        } else {
            push(Problem::Edited);
        }
        if eprev != prev {
            push(Problem::Unlinked);
        }
        if signer.is_some_and(|s| s != key) {
            push(Problem::OtherSigner);
        }
        prev = stored;
        expected = e.seq + 1;
    }
    if let Some(a) = anchor
        && !entries.iter().any(|e| e.seq == a.seq && e.hash == a.hash)
    {
        findings.push(Finding {
            seq: a.seq,
            kind: String::new(),
            problem: Problem::TipMissing,
        });
    }
    VerifyReport {
        records: entries.len() as u64,
        tip: entries.last().map(|e| format!("{}:{}", e.seq, e.hash)),
        signer: signer.map(hex::encode),
        signer_pinned: pinned.is_some(),
        findings,
    }
}
