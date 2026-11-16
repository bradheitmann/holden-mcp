//! Adapters for a client repository's existing planning process.
//!
//! Detection is explicit and ordered: charter, dispatch graph, then Markdown.
//! Unknown layouts use the Markdown adapter and carry a typed limitation.
//! Promises remain byte spans of their original documents. Write-back is a
//! patch proposal by default; applying it requires [`WritebackOptions::apply`].

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::approval::{Approval, ApprovalError, HostableCognition, OwnerTrust};
use crate::promise::{
    PROMISE_MARKER, PlanningDocument, Promise, PromiseError, extract_promises, promise_id,
};

/// The planning process detected in a client repository.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterKind {
    /// `charter/INTENT.md` plus at least one `charter/DONE/*.md` list.
    Charter,
    /// One or more graph-shaped `dispatch-graph.json` files.
    DispatchGraph,
    /// README, PRD, and Markdown files below `docs/`.
    Markdown,
}

impl AdapterKind {
    /// Stable lower-case name used by detection reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Charter => "charter",
            Self::DispatchGraph => "dispatch_graph",
            Self::Markdown => "markdown",
        }
    }
}

/// A limitation recorded by detection. It never becomes a silent pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterLimitation {
    /// No known planning layout was found; Markdown extraction was attempted.
    UnknownProcess,
}

impl AdapterLimitation {
    /// Stable machine-readable limitation code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnknownProcess => "unknown_process_markdown_fallback",
        }
    }
}

/// A detected adapter and its repository-relative planning files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanningAdapter {
    /// Detection result.
    pub kind: AdapterKind,
    /// Input documents, sorted by repository-relative path.
    pub documents: Vec<PathBuf>,
    /// Typed limitations, empty for a recognized layout.
    pub limitations: Vec<AdapterLimitation>,
}

/// Promises extracted from one client document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentPromises {
    /// Exact source document.
    pub document: PlanningDocument,
    /// Verbatim spans in source-byte order.
    pub promises: Vec<Promise>,
}

/// How the client's candidate promises were discovered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryKind {
    /// At least one explicit marker or established charter outcome span was found.
    Explicit,
    /// No explicit span existed, so prose paragraphs are owner-approval candidates.
    Prose,
}

/// All planning evidence read through an adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientPromises {
    /// The adapter that selected these documents.
    pub adapter: PlanningAdapter,
    /// Documents in deterministic path order.
    pub documents: Vec<DocumentPromises>,
    /// Whether these are marked promises or prose candidates awaiting approval.
    pub discovery: DiscoveryKind,
}

impl ClientPromises {
    /// Every promise with the document whose exact bytes carry its span.
    #[must_use]
    pub fn spans(&self) -> Vec<(&PlanningDocument, &Promise)> {
        self.documents
            .iter()
            .flat_map(|d| d.promises.iter().map(move |p| (&d.document, p)))
            .collect()
    }

    /// Every source document, in adapter order.
    #[must_use]
    pub fn planning_documents(&self) -> Vec<&PlanningDocument> {
        self.documents.iter().map(|d| &d.document).collect()
    }

    /// The first selected source, used only for presentation (for example,
    /// a domain heading). Verification always uses each span's own source.
    #[must_use]
    pub fn primary_document(&self) -> Option<&PlanningDocument> {
        self.documents.first().map(|d| &d.document)
    }

    /// Admit approved cognitions against each promise's own source bytes.
    /// A matching promise id in another document is not enough: the span's
    /// source hash, offsets and text must all verify.
    #[must_use]
    pub fn admit(
        &self,
        cognitions: &[crate::SituatedCognition],
        approvals: &[Approval],
        trust: &OwnerTrust,
    ) -> (
        Vec<HostableCognition>,
        Vec<(crate::ContentId, ApprovalError)>,
    ) {
        let mut admitted = Vec::new();
        let mut refused = Vec::new();
        for cognition in cognitions {
            let span = self
                .spans()
                .into_iter()
                .find(|(_, promise)| promise.id == cognition.promise);
            let Some((document, promise)) = span else {
                refused.push((cognition.id, ApprovalError::CognitionForOtherPromise));
                continue;
            };
            let approval = approvals.iter().find(|a| a.cognition == cognition.id);
            match HostableCognition::admit(document, promise, cognition, approval, trust.public()) {
                Ok(hosted) => admitted.push(hosted),
                Err(error) => refused.push((cognition.id, error)),
            }
        }
        (admitted, refused)
    }
}

/// The public card for a checkpoint written into the client's plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldoutCheckpoint {
    /// Dispatch-graph node id; must begin with `HOLDOUT-`.
    pub id: String,
    /// Public checkpoint title.
    pub title: String,
    /// Owner-approved objective, without sealed parameters.
    pub objective: String,
    /// Public acceptance criterion.
    pub acceptance_criterion: String,
}

/// Write-back controls. The default is proposal-only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WritebackOptions {
    /// Apply only when explicitly set to true.
    pub apply: bool,
    /// Proposal destination; ignored when `apply` is true.
    pub patch_path: PathBuf,
}

impl Default for WritebackOptions {
    fn default() -> Self {
        Self {
            apply: false,
            patch_path: PathBuf::from(".holden/holdout-writeback.patch"),
        }
    }
}

/// What write-back did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WritebackOutcome {
    /// Repository-relative target.
    pub target: PathBuf,
    /// Patch written in proposal mode.
    pub patch: Option<PathBuf>,
    /// Whether the target was changed.
    pub applied: bool,
}

/// Adapter failures.
#[derive(Debug)]
pub enum AdapterError {
    /// File-system failure.
    Io(io::Error),
    /// Verbatim promise construction failed.
    Promise(PromiseError),
    /// Invalid dispatch-graph JSON.
    Json(serde_json::Error),
    /// A graph is missing a field required for safe write-back.
    InvalidGraph(&'static str),
    /// A checkpoint id is malformed or already present.
    InvalidCheckpoint(String),
    /// This adapter has no native write-back representation.
    UnsupportedWriteback(AdapterKind),
    /// The editable charter DONE template is absent.
    MissingDoneTemplate,
    /// An operation would overwrite a pinned DONE list.
    PinnedDoneList(PathBuf),
    /// A path could not be represented relative to the client root.
    PathOutsideRoot(PathBuf),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "planning adapter I/O failed: {e}"),
            Self::Promise(e) => e.fmt(f),
            Self::Json(e) => write!(f, "invalid dispatch graph: {e}"),
            Self::InvalidGraph(reason) => write!(f, "invalid dispatch graph: {reason}"),
            Self::InvalidCheckpoint(reason) => write!(f, "invalid holdout checkpoint: {reason}"),
            Self::UnsupportedWriteback(kind) => {
                write!(f, "write-back is not defined for {kind:?} planning")
            }
            Self::MissingDoneTemplate => {
                f.write_str("charter/DONE/TEMPLATE.md is required for charter write-back")
            }
            Self::PinnedDoneList(path) => {
                write!(f, "refusing to modify pinned DONE list {}", path.display())
            }
            Self::PathOutsideRoot(path) => {
                write!(f, "path is outside the client root: {}", path.display())
            }
        }
    }
}

impl std::error::Error for AdapterError {}

impl From<io::Error> for AdapterError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<PromiseError> for AdapterError {
    fn from(value: PromiseError) -> Self {
        Self::Promise(value)
    }
}

impl From<serde_json::Error> for AdapterError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

fn planning_files(
    root: &Path,
    directory: &Path,
    selected: impl Fn(&fs::DirEntry) -> bool + Copy,
) -> io::Result<Vec<PathBuf>> {
    let mut directories = vec![directory.to_path_buf()];
    let mut files = Vec::new();
    while let Some(current) = directories.pop() {
        if !current.is_dir() {
            continue;
        }
        let mut entries = fs::read_dir(&current)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                if entry.file_name() != ".git" && entry.file_name() != "target" {
                    directories.push(path);
                }
            } else if file_type.is_file() && selected(&entry) {
                files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
            }
        }
    }
    files.sort();
    Ok(files)
}

fn markdown_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for name in ["PLANNING.md", "PRD.md", "README.md"] {
        if root.join(name).is_file() {
            files.push(PathBuf::from(name));
        }
    }
    files.extend(planning_files(root, &root.join("docs"), |entry| {
        entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "md" || extension == "markdown")
    })?);
    files.sort();
    files.dedup();
    Ok(files)
}

/// Detect a client's planning process with fixed type precedence and sorted
/// paths. No directory or map iteration determines the selected adapter.
///
/// # Errors
/// Directory enumeration failures.
pub fn detect(root: &Path) -> Result<PlanningAdapter, AdapterError> {
    let intent = root.join("charter/INTENT.md");
    let done = root.join("charter/DONE");
    if !root.join("PLANNING.md").is_file() && intent.is_file() && done.is_dir() {
        let mut documents = vec![PathBuf::from("charter/INTENT.md")];
        let guarantees = PathBuf::from("charter/GUARANTEES.yaml");
        if root.join(&guarantees).is_file() {
            documents.push(guarantees);
        }
        let mut lists = fs::read_dir(done)?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|entry| {
                entry.path().extension().is_some_and(|e| e == "md")
                    && !entry.file_name().to_string_lossy().starts_with('_')
            })
            .map(|entry| PathBuf::from("charter/DONE").join(entry.file_name()))
            .collect::<Vec<_>>();
        lists.sort();
        if !lists.is_empty() {
            documents.extend(lists);
            documents.sort();
            return Ok(PlanningAdapter {
                kind: AdapterKind::Charter,
                documents,
                limitations: Vec::new(),
            });
        }
    }

    let graphs = planning_files(root, root, |entry| {
        entry.file_name() == "dispatch-graph.json"
    })?;
    if !graphs.is_empty() {
        return Ok(PlanningAdapter {
            kind: AdapterKind::DispatchGraph,
            documents: graphs,
            limitations: Vec::new(),
        });
    }

    let documents = markdown_files(root)?;
    let known_markdown = !documents.is_empty();
    Ok(PlanningAdapter {
        kind: AdapterKind::Markdown,
        documents,
        limitations: if known_markdown {
            Vec::new()
        } else {
            vec![AdapterLimitation::UnknownProcess]
        },
    })
}

/// Byte spans of JSON string values that carry planning promises.
struct GraphSpans<'a> {
    document: &'a PlanningDocument,
    position: usize,
    promises: Vec<Promise>,
}

impl<'a> GraphSpans<'a> {
    fn whitespace(&mut self) {
        while self
            .document
            .bytes
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }

    fn string(&mut self) -> Result<(usize, usize, String), AdapterError> {
        let start = self.position;
        self.position += 1;
        while let Some(byte) = self.document.bytes.get(self.position) {
            match byte {
                b'\\' => self.position += 2,
                b'"' => {
                    self.position += 1;
                    let end = self.position;
                    let decoded = serde_json::from_slice(&self.document.bytes[start..end])?;
                    return Ok((start, end, decoded));
                }
                _ => self.position += 1,
            }
        }
        Err(AdapterError::InvalidGraph("unterminated JSON string"))
    }

    fn value(&mut self, path: &mut Vec<String>) -> Result<(), AdapterError> {
        self.whitespace();
        match self.document.bytes.get(self.position) {
            Some(b'{') => {
                self.position += 1;
                self.whitespace();
                if self.document.bytes.get(self.position) == Some(&b'}') {
                    self.position += 1;
                    return Ok(());
                }
                loop {
                    self.whitespace();
                    let (_, _, key) = self.string()?;
                    self.whitespace();
                    self.position += 1;
                    path.push(key);
                    self.value(path)?;
                    path.pop();
                    self.whitespace();
                    let separator = self.document.bytes[self.position];
                    self.position += 1;
                    if separator == b'}' {
                        return Ok(());
                    }
                }
            }
            Some(b'[') => {
                self.position += 1;
                self.whitespace();
                if self.document.bytes.get(self.position) == Some(&b']') {
                    self.position += 1;
                    return Ok(());
                }
                let mut index = 0usize;
                loop {
                    path.push(index.to_string());
                    self.value(path)?;
                    path.pop();
                    index += 1;
                    self.whitespace();
                    let separator = self.document.bytes[self.position];
                    self.position += 1;
                    if separator == b']' {
                        return Ok(());
                    }
                }
            }
            Some(b'"') => {
                let (start, end, _) = self.string()?;
                let planning_value = path.first().is_some_and(|p| p == "nodes")
                    && path.get(2).is_some_and(|p| p == "contract")
                    && (path
                        .last()
                        .is_some_and(|p| p == "objective" || p == "acceptance_criterion")
                        || (path.len() == 5
                            && path.get(3).is_some_and(|p| p == "acceptance_criterion")
                            && path.last().is_some_and(|p| p.parse::<usize>().is_ok())));
                if planning_value {
                    self.promises.push(Promise::slice(
                        self.document,
                        (start + 1) as u64,
                        (end - 1) as u64,
                    )?);
                }
                Ok(())
            }
            Some(_) => {
                while self
                    .document
                    .bytes
                    .get(self.position)
                    .is_some_and(|b| !b.is_ascii_whitespace() && !matches!(b, b',' | b']' | b'}'))
                {
                    self.position += 1;
                }
                Ok(())
            }
            None => Err(AdapterError::InvalidGraph("missing JSON value")),
        }
    }
}

fn graph_promises(document: &PlanningDocument) -> Result<Vec<Promise>, AdapterError> {
    let _: Value = serde_json::from_slice(&document.bytes)?;
    let mut parser = GraphSpans {
        document,
        position: 0,
        promises: Vec::new(),
    };
    parser.value(&mut Vec::new())?;
    Ok(parser.promises)
}

#[derive(Clone, Copy)]
enum ProofBlock {
    Label { seen_item: bool },
    Heading { level: usize },
    Title,
}

fn done_promises(document: &PlanningDocument) -> Result<Vec<Promise>, AdapterError> {
    let has_proof_label = document
        .bytes
        .split_inclusive(|byte| *byte == b'\n')
        .filter_map(|line| std::str::from_utf8(line).ok())
        .any(|line| {
            let trimmed = line.trim_start();
            let level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
            let heading = ((1..=6).contains(&level)
                && trimmed
                    .as_bytes()
                    .get(level)
                    .is_some_and(u8::is_ascii_whitespace))
            .then(|| trimmed[level + 1..].trim_start());
            trimmed.starts_with("Proofs")
                || heading.is_some_and(|text| text.to_ascii_lowercase().contains("proof"))
        });
    let mut block = None;
    let mut pending: Option<(usize, usize)> = None;
    let mut promises = Vec::new();
    let mut offset = 0usize;
    let mut fenced = false;

    fn flush(
        document: &PlanningDocument,
        pending: &mut Option<(usize, usize)>,
        promises: &mut Vec<Promise>,
    ) -> Result<(), AdapterError> {
        if let Some((start, end)) = pending.take() {
            let promise = Promise::slice(document, start as u64, end as u64)?;
            if promise.text.chars().count() > 40 {
                promises.push(promise);
            }
        }
        Ok(())
    }

    for line in document.bytes.split_inclusive(|byte| *byte == b'\n') {
        let bare = line.strip_suffix(b"\n").unwrap_or(line);
        let bare = bare.strip_suffix(b"\r").unwrap_or(bare);
        let Ok(text) = std::str::from_utf8(bare) else {
            flush(document, &mut pending, &mut promises)?;
            offset += line.len();
            continue;
        };
        let trimmed = text.trim_start();
        let indent = text.len() - trimmed.len();
        let numbered = trimmed.split_once(". ").and_then(|(number, _)| {
            let digits = number.bytes().take_while(u8::is_ascii_digit).count();
            let valid = digits > 0
                && (number.len() == digits
                    || (number.len() == digits + 1
                        && number.as_bytes()[digits].is_ascii_lowercase()));
            valid.then_some(indent + number.len() + 2)
        });
        let bulleted = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .map(|_| indent + 2);
        let heading_level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        let heading = ((1..=6).contains(&heading_level)
            && trimmed
                .as_bytes()
                .get(heading_level)
                .is_some_and(u8::is_ascii_whitespace))
        .then(|| trimmed[heading_level + 1..].trim_start());
        let proof_heading = heading.is_some_and(|text| text.to_ascii_lowercase().contains("proof"));

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush(document, &mut pending, &mut promises)?;
            if matches!(block, Some(ProofBlock::Label { seen_item: true })) {
                block = None;
            }
            fenced = !fenced;
            offset += line.len();
            continue;
        }
        if fenced {
            offset += line.len();
            continue;
        }
        if heading.is_some() {
            flush(document, &mut pending, &mut promises)?;
            if proof_heading {
                block = Some(ProofBlock::Heading {
                    level: heading_level,
                });
            } else {
                match block {
                    Some(ProofBlock::Heading { level }) if heading_level <= level => {
                        block = None;
                    }
                    Some(ProofBlock::Label { .. }) => block = None,
                    Some(ProofBlock::Title) if heading_level == 2 => block = None,
                    _ => {}
                }
                if !has_proof_label && heading_level == 1 && block.is_none() {
                    block = Some(ProofBlock::Title);
                }
            }
            offset += line.len();
            continue;
        }
        if trimmed.starts_with("Proofs") {
            flush(document, &mut pending, &mut promises)?;
            block = Some(ProofBlock::Label { seen_item: false });
            offset += line.len();
            continue;
        }
        if trimmed.is_empty() {
            flush(document, &mut pending, &mut promises)?;
            offset += line.len();
            continue;
        }
        if indent == 0
            && matches!(
                trimmed.trim_end_matches(':'),
                "Gates" | "Steps" | "Out of scope" | "Schedule" | "Landing"
            )
        {
            flush(document, &mut pending, &mut promises)?;
            block = None;
            offset += line.len();
            continue;
        }

        let top_level_item = indent == 0
            && (numbered.is_some()
                || (!matches!(block, Some(ProofBlock::Title)) && bulleted.is_some()));
        if block.is_some() && top_level_item {
            flush(document, &mut pending, &mut promises)?;
            let body_start = numbered.or(bulleted).unwrap_or(0);
            pending = Some((offset + body_start, offset + bare.len()));
            if let Some(ProofBlock::Label { seen_item }) = block.as_mut() {
                *seen_item = true;
            }
            offset += line.len();
            continue;
        }

        let structural = trimmed.starts_with('|') || trimmed.starts_with('<');
        let continuation = pending.is_some()
            && indent > 0
            && numbered.is_none()
            && bulleted.is_none()
            && !structural;
        if continuation {
            if let Some((_, end)) = pending.as_mut() {
                *end = offset + bare.len();
            }
            offset += line.len();
            continue;
        }
        flush(document, &mut pending, &mut promises)?;
        if indent == 0 && matches!(block, Some(ProofBlock::Label { seen_item: true })) {
            block = None;
        }
        offset += line.len();
    }
    flush(document, &mut pending, &mut promises)?;
    Ok(promises)
}

/// A charter publishes only its top-level numbered or bulleted DONE proofs
/// and guarantee statements as approval candidates. Proof lists may follow a
/// bare `Proofs` line or a Markdown heading containing `proof`; headings,
/// narrative, indented sub-items and template files are not promises. Only
/// the bytes after the item marker (and any YAML quotes around a guarantee
/// statement) become the promise.
fn charter_promises(document: &PlanningDocument) -> Result<Vec<Promise>, AdapterError> {
    let guarantee = document.name == "charter/GUARANTEES.yaml";
    let done_proof = document.name.starts_with("charter/DONE/");
    if !guarantee && !done_proof {
        return Ok(Vec::new());
    }
    if done_proof {
        return done_promises(document);
    }
    let source = document.sha256();
    let mut offset = 0usize;
    let mut in_proofs = guarantee;
    let mut promises = Vec::new();
    let mut pending: Option<(usize, usize, String)> = None;
    let flush = |pending: &mut Option<(usize, usize, String)>, promises: &mut Vec<Promise>| {
        if let Some((start, end, text)) = pending.take()
            && text.chars().count() > 40
        {
            let start = start as u64;
            let end = end as u64;
            let id = promise_id(&source, start, end, &text);
            promises.push(Promise {
                id,
                source,
                start,
                end,
                text,
            });
        }
    };
    for line in document.bytes.split_inclusive(|byte| *byte == b'\n') {
        let bare = line.strip_suffix(b"\n").unwrap_or(line);
        let bare = bare.strip_suffix(b"\r").unwrap_or(bare);
        let Ok(text) = std::str::from_utf8(bare) else {
            flush(&mut pending, &mut promises);
            offset += line.len();
            continue;
        };
        let trimmed = text.trim_start();
        let indent = text.len() - trimmed.len();
        let numbered = trimmed.split_once(". ").and_then(|(number, _)| {
            let digits = number.bytes().take_while(u8::is_ascii_digit).count();
            let valid = digits > 0
                && (number.len() == digits
                    || (number.len() == digits + 1
                        && number.as_bytes()[digits].is_ascii_lowercase()));
            valid.then_some(indent + number.len() + 2)
        });
        let bulleted = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .map(|_| indent + 2);
        let heading_level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        let heading = ((1..=6).contains(&heading_level)
            && trimmed
                .as_bytes()
                .get(heading_level)
                .is_some_and(u8::is_ascii_whitespace))
        .then(|| trimmed[heading_level + 1..].trim_start());
        let proof_heading = heading.is_some_and(|text| text.to_ascii_lowercase().contains("proof"));
        if done_proof {
            if trimmed.starts_with("Proofs") || proof_heading {
                flush(&mut pending, &mut promises);
                in_proofs = true;
                offset += line.len();
                continue;
            }
            if heading.is_some() {
                flush(&mut pending, &mut promises);
                in_proofs = false;
            }
            let top_level_item = indent == 0 && (numbered.is_some() || bulleted.is_some());
            if pending.is_some() {
                let continuation = in_proofs
                    && indent > 0
                    && !trimmed.is_empty()
                    && heading.is_none()
                    && numbered.is_none()
                    && bulleted.is_none();
                if continuation {
                    if let Some((_, end, text)) = pending.as_mut() {
                        *end = offset + bare.len();
                        text.push(' ');
                        text.push_str(trimmed);
                    }
                    offset += line.len();
                    continue;
                }
                flush(&mut pending, &mut promises);
            }
            if in_proofs && indent == 0 && !trimmed.is_empty() && !top_level_item {
                in_proofs = false;
            }
        }
        let body_start = if guarantee {
            trimmed.strip_prefix("statement: ").map(|_| indent + 11)
        } else if in_proofs && indent == 0 {
            numbered.or(bulleted)
        } else {
            None
        };
        if let Some(body_start) = body_start {
            let mut start = body_start;
            let mut end = bare.len();
            if guarantee
                && end > start + 1
                && ((bare[start] == b'"' && bare[end - 1] == b'"')
                    || (bare[start] == b'\'' && bare[end - 1] == b'\''))
            {
                start += 1;
                end -= 1;
            }
            if guarantee {
                if text[start..end].chars().count() > 40 {
                    let text = text[start..end].to_owned();
                    let start = (offset + start) as u64;
                    let end = (offset + end) as u64;
                    let id = promise_id(&source, start, end, &text);
                    promises.push(Promise {
                        id,
                        source,
                        start,
                        end,
                        text,
                    });
                }
            } else {
                pending = Some((offset + start, offset + end, text[start..end].to_owned()));
            }
        }
        offset += line.len();
    }
    flush(&mut pending, &mut promises);
    Ok(promises)
}

/// Quote prose without interpreting or rewriting it. Markdown structure is
/// skipped; a paragraph or list body is offered whole, directly from its
/// source bytes. These are candidates only: owner approval remains the sole
/// authority that turns one into a promise which can host a check.
fn prose_promises(document: &PlanningDocument) -> Result<Vec<Promise>, PromiseError> {
    let mut promises = Vec::new();
    let mut paragraph: Option<(usize, usize)> = None;
    let mut offset = 0usize;
    let mut fenced = false;

    fn flush(
        document: &PlanningDocument,
        paragraph: &mut Option<(usize, usize)>,
        promises: &mut Vec<Promise>,
    ) -> Result<(), PromiseError> {
        if let Some((start, end)) = paragraph.take() {
            promises.push(Promise::slice(document, start as u64, end as u64)?);
        }
        Ok(())
    }

    for line in document.bytes.split_inclusive(|byte| *byte == b'\n') {
        let bare = line.strip_suffix(b"\n").unwrap_or(line);
        let bare = bare.strip_suffix(b"\r").unwrap_or(bare);
        let Ok(text) = std::str::from_utf8(bare) else {
            flush(document, &mut paragraph, &mut promises)?;
            offset += line.len();
            continue;
        };
        let trimmed = text.trim_start();
        let indent = text.len() - trimmed.len();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush(document, &mut paragraph, &mut promises)?;
            fenced = !fenced;
            offset += line.len();
            continue;
        }
        let structural = fenced
            || trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with('>')
            || trimmed.starts_with('<')
            || trimmed == "---"
            || trimmed.starts_with("Domain: ")
            || trimmed.starts_with(PROMISE_MARKER);
        if structural {
            flush(document, &mut paragraph, &mut promises)?;
            offset += line.len();
            continue;
        }

        let bullet = if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
        {
            Some(2usize)
        } else {
            trimmed.split_once(". ").and_then(|(number, _)| {
                (!number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
                    .then_some(number.len() + 2)
            })
        };
        if let Some(prefix) = bullet {
            flush(document, &mut paragraph, &mut promises)?;
            let start = offset + indent + prefix;
            promises.push(Promise::slice(
                document,
                start as u64,
                (offset + bare.len()) as u64,
            )?);
        } else {
            let start = paragraph.map_or(offset + indent, |(start, _)| start);
            paragraph = Some((start, offset + bare.len()));
        }
        offset += line.len();
    }
    flush(document, &mut paragraph, &mut promises)?;
    Ok(promises)
}

/// Read and extract promises through the detected adapter.
///
/// # Errors
/// I/O, malformed graph JSON, or an invalid verbatim span.
pub fn read_promises(root: &Path) -> Result<ClientPromises, AdapterError> {
    let adapter = detect(root)?;
    let mut documents = Vec::new();
    for relative in &adapter.documents {
        let name = relative.to_string_lossy().into_owned();
        let document = PlanningDocument::read(&root.join(relative), &name)?;
        let promises = match adapter.kind {
            AdapterKind::DispatchGraph => graph_promises(&document)?,
            AdapterKind::Charter => charter_promises(&document)?,
            AdapterKind::Markdown => extract_promises(&document)?,
        };
        documents.push(DocumentPromises { document, promises });
    }
    let mut discovery = DiscoveryKind::Explicit;
    if adapter.kind == AdapterKind::Markdown
        && documents
            .iter()
            .all(|document| document.promises.is_empty())
    {
        discovery = DiscoveryKind::Prose;
        for document in &mut documents {
            document.promises = prose_promises(&document.document)?;
        }
    }
    Ok(ClientPromises {
        adapter,
        documents,
        discovery,
    })
}

fn graph_writeback(bytes: &[u8], checkpoint: &HoldoutCheckpoint) -> Result<Vec<u8>, AdapterError> {
    if !checkpoint.id.starts_with("HOLDOUT-") || checkpoint.id.len() == "HOLDOUT-".len() {
        return Err(AdapterError::InvalidCheckpoint(
            "id must be HOLDOUT- followed by a non-empty suffix".to_owned(),
        ));
    }
    let mut graph: Value = serde_json::from_slice(bytes)?;
    let nodes = graph
        .get_mut("nodes")
        .and_then(Value::as_array_mut)
        .ok_or(AdapterError::InvalidGraph("nodes must be an array"))?;
    if nodes
        .iter()
        .any(|node| node.get("id").and_then(Value::as_str) == Some(&checkpoint.id))
    {
        return Err(AdapterError::InvalidCheckpoint(format!(
            "{} already exists",
            checkpoint.id
        )));
    }
    let mut qa = nodes
        .iter()
        .filter(|node| node.get("type").and_then(Value::as_str) == Some("QA"))
        .filter_map(|node| node.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect::<Vec<_>>();
    qa.sort();
    let mut object = Map::new();
    object.insert("id".to_owned(), json!(checkpoint.id));
    object.insert("type".to_owned(), json!("HOLDOUT"));
    object.insert("title".to_owned(), json!(checkpoint.title));
    object.insert("depends_on".to_owned(), json!(qa));
    object.insert(
        "contract".to_owned(),
        json!({
            "objective": checkpoint.objective,
            "acceptance_criterion": checkpoint.acceptance_criterion,
        }),
    );
    nodes.push(Value::Object(object));
    graph
        .get_mut("dependency_frontiers")
        .and_then(Value::as_array_mut)
        .ok_or(AdapterError::InvalidGraph(
            "dependency_frontiers must be an array",
        ))?
        .push(json!([checkpoint.id]));
    let mut output = serde_json::to_vec_pretty(&graph)?;
    output.push(b'\n');
    Ok(output)
}

fn charter_writeback(
    bytes: &[u8],
    checkpoint: &HoldoutCheckpoint,
) -> Result<Vec<u8>, AdapterError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| AdapterError::InvalidGraph("DONE template must be UTF-8"))?;
    let mut output = text.to_owned();
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output.push_str(&format!(
        "- Holdout proof {}: {}; acceptance: {}.\n",
        checkpoint.id, checkpoint.objective, checkpoint.acceptance_criterion
    ));
    Ok(output.into_bytes())
}

fn unified_patch(path: &Path, old: &[u8], new: &[u8]) -> Vec<u8> {
    let old_text = String::from_utf8_lossy(old);
    let new_text = String::from_utf8_lossy(new);
    let old_lines = old_text.lines().count();
    let new_lines = new_text.lines().count();
    let mut patch = format!(
        "--- a/{}\n+++ b/{}\n@@ -1,{old_lines} +1,{new_lines} @@\n",
        path.display(),
        path.display()
    );
    for line in old_text.lines() {
        patch.push('-');
        patch.push_str(line);
        patch.push('\n');
    }
    for line in new_text.lines() {
        patch.push('+');
        patch.push_str(line);
        patch.push('\n');
    }
    patch.into_bytes()
}

fn pinned_done_patch(root: &Path, patch: &Path) -> bool {
    patch.strip_prefix(root).ok().is_some_and(|relative| {
        relative.starts_with("charter/DONE") && relative != Path::new("charter/DONE/TEMPLATE.md")
    })
}

/// Write a native planning checkpoint, as a patch by default or in place only
/// when `options.apply` is explicitly true. Existing DONE lists are pinned;
/// only `charter/DONE/TEMPLATE.md` is writable.
///
/// # Errors
/// Invalid input, unsupported process, unsafe target, or I/O failure.
pub fn write_back(
    root: &Path,
    checkpoint: &HoldoutCheckpoint,
    options: &WritebackOptions,
) -> Result<WritebackOutcome, AdapterError> {
    let adapter = detect(root)?;
    let target = match adapter.kind {
        AdapterKind::DispatchGraph => adapter
            .documents
            .first()
            .cloned()
            .ok_or(AdapterError::InvalidGraph("no dispatch graph was detected"))?,
        AdapterKind::Charter => {
            let template = PathBuf::from("charter/DONE/TEMPLATE.md");
            if !root.join(&template).is_file() {
                return Err(AdapterError::MissingDoneTemplate);
            }
            template
        }
        AdapterKind::Markdown => return Err(AdapterError::UnsupportedWriteback(adapter.kind)),
    };
    if !options.apply && pinned_done_patch(root, &options.patch_path) {
        return Err(AdapterError::PinnedDoneList(options.patch_path.clone()));
    }
    let target_path = root.join(&target);
    let old = fs::read(&target_path)?;
    let new = match adapter.kind {
        AdapterKind::DispatchGraph => graph_writeback(&old, checkpoint)?,
        AdapterKind::Charter => charter_writeback(&old, checkpoint)?,
        AdapterKind::Markdown => unreachable!("Markdown returned above"),
    };
    if options.apply {
        fs::write(target_path, new)?;
        Ok(WritebackOutcome {
            target,
            patch: None,
            applied: true,
        })
    } else {
        if let Some(parent) = options.patch_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let patch = unified_patch(&target, &old, &new);
        fs::write(&options.patch_path, patch)?;
        let patch = options
            .patch_path
            .strip_prefix(root)
            .map_err(|_| AdapterError::PathOutsideRoot(options.patch_path.clone()))?;
        Ok(WritebackOutcome {
            target,
            patch: Some(patch.to_path_buf()),
            applied: false,
        })
    }
}
