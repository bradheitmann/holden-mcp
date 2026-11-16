//! The module's file set, `SHA256SUMS`, the content scan and `forge export`.
//!
//! The file set is every regular file under the module root except the
//! build directory (`target/` at the root), version-control directories,
//! `.DS_Store` files and `SHA256SUMS` itself. `SHA256SUMS` lists each file as
//! `<sha256 hex>  <relative path>`, sorted by path, in the format
//! `shasum -a 256 -c` reads.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::sha256;

/// The checksum file name.
pub const SUMS: &str = "SHA256SUMS";

/// SHA-256 of the official PolyForm Noncommercial 1.0.0 text (`LICENSE`).
pub const LICENSE_SHA256: &str = "c0ea4a896d2c8c394b29f9427589996db826cd501c512279ff0ed3ef48fabbe5";

/// SHA-256 of `NOTICE`: exactly the owner's two Required Notice lines. The
/// notice must name the licensor, so `NOTICE` is the one file the denylist
/// scan exempts, and only while it holds exactly these bytes.
pub const NOTICE_SHA256: &str = "b48acabe3c9a16a65780c380afc1c05f5578286a1f3119612b4eca99865c717b";

fn skip(rel: &str, is_dir: bool) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    (is_dir && (rel == "target" || name == ".git"))
        || (!is_dir && (rel == SUMS || name == ".DS_Store"))
}

/// Every file of the module rooted at `root`, as sorted `/`-separated
/// relative paths. A symbolic link is an error: an export copies bytes,
/// never links.
pub fn module_files(root: &Path) -> io::Result<Vec<String>> {
    fn walk(root: &Path, rel: &str, out: &mut Vec<String>) -> io::Result<()> {
        let dir = if rel.is_empty() {
            root.to_path_buf()
        } else {
            root.join(rel)
        };
        for e in fs::read_dir(&dir)? {
            let e = e?;
            let name = e.file_name().to_string_lossy().into_owned();
            let child = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            let ft = e.file_type()?;
            if ft.is_symlink() {
                return Err(io::Error::other(format!(
                    "{child}: symbolic links are not allowed in the module"
                )));
            }
            if skip(&child, ft.is_dir()) {
                continue;
            }
            if ft.is_dir() {
                walk(root, &child, out)?;
            } else if ft.is_file() {
                out.push(child);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, "", &mut out)?;
    out.sort();
    Ok(out)
}

/// The `SHA256SUMS` text for `files` under `root`.
pub fn sums_text(root: &Path, files: &[String]) -> io::Result<String> {
    let mut out = String::new();
    for f in files {
        let bytes = fs::read(root.join(f))?;
        out.push_str(&format!("{}  {f}\n", sha256::hex(&bytes)));
    }
    Ok(out)
}

/// Regenerate `SHA256SUMS` in `root`. Returns the number of files covered.
pub fn write_sums(root: &Path) -> io::Result<usize> {
    let files = module_files(root)?;
    fs::write(root.join(SUMS), sums_text(root, &files)?)?;
    Ok(files.len())
}

/// Check `SHA256SUMS` in `root`: every listed file exists with that hash, and
/// every module file is listed. Returns the file count, or every finding,
/// each naming a file.
pub fn verify_sums(root: &Path) -> Result<usize, Vec<String>> {
    let text = fs::read_to_string(root.join(SUMS))
        .map_err(|e| vec![format!("{SUMS}: cannot be read: {e}")])?;
    let files =
        module_files(root).map_err(|e| vec![format!("the module cannot be listed: {e}")])?;
    let mut findings = Vec::new();
    let mut listed: Vec<&str> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let Some((hash, path)) = line.split_once("  ") else {
            findings.push(format!("{SUMS}:{}: malformed line", n + 1));
            continue;
        };
        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            findings.push(format!("{SUMS}:{}: malformed hash", n + 1));
            continue;
        }
        if listed.contains(&path) {
            findings.push(format!("{path}: listed twice"));
            continue;
        }
        listed.push(path);
        if !files.iter().any(|f| f == path) {
            findings.push(format!("{path}: listed but not in the module"));
            continue;
        }
        match fs::read(root.join(path)) {
            Ok(bytes) if sha256::hex(&bytes) == hash => {}
            Ok(_) => findings.push(format!("{path}: sha256 does not match {SUMS}")),
            Err(e) => findings.push(format!("{path}: cannot be read: {e}")),
        }
    }
    for f in &files {
        if !listed.contains(&f.as_str()) {
            findings.push(format!("{f}: not covered by {SUMS}"));
        }
    }
    if findings.is_empty() {
        Ok(files.len())
    } else {
        Err(findings)
    }
}

/// The private identifier denylist: one identifier per line, `#` comments.
#[derive(Clone, Debug)]
pub struct Denylist {
    entries: Vec<(usize, String)>,
}

impl Denylist {
    /// Parse a denylist file's text. Matching is case-insensitive, whole word.
    pub fn parse(text: &str) -> Denylist {
        Denylist {
            entries: text
                .lines()
                .enumerate()
                .map(|(n, l)| (n + 1, l.trim().to_lowercase()))
                .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'))
                .collect(),
        }
    }

    /// Whether the list has no entry (an empty list cannot vouch for anything).
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Whether `haystack` (lowercase) contains `needle` (lowercase) as a whole word.
fn contains_word(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let at = start + pos;
        let end = at + needle.len();
        if (at == 0 || !word_byte(h[at - 1])) && (end >= h.len() || !word_byte(h[end])) {
            return true;
        }
        start = at + 1;
        while start < haystack.len() && !haystack.is_char_boundary(start) {
            start += 1;
        }
    }
    false
}

/// Absolute-path prefixes refused in exported text. They are assembled at
/// run time so this source file does not itself contain them.
fn absolute_prefixes() -> Vec<String> {
    [
        ["/", "Users", "/"].concat(),
        ["/", "home", "/"].concat(),
        ["/", "root", "/"].concat(),
        ["/", "private", "/"].concat(),
        ["/", "var", "/", "folders", "/"].concat(),
        ["/", "tmp", "/"].concat(),
        [":", "\\", "Users", "\\"].concat(),
    ]
    .to_vec()
}

/// References to the repository this module was developed in. Assembled at
/// run time for the same reason.
fn parent_references() -> Vec<String> {
    [
        ["holden", "-", "mcp"].concat(),
        ["modules", "/", "holden-forge"].concat(),
    ]
    .to_vec()
}

/// Scan `files` under `root` for absolute paths, private identifiers and
/// references to the parent repository. Findings name the file and line,
/// never the identifier found.
pub fn scan(root: &Path, files: &[String], denylist: &Denylist) -> Vec<String> {
    let abs = absolute_prefixes();
    let parents = parent_references();
    let mut findings = Vec::new();
    for f in files {
        let lower_name = f.to_lowercase();
        for (n, id) in &denylist.entries {
            if contains_word(&lower_name, id) {
                findings.push(format!("{f}: the file name holds denylist entry #{n}"));
            }
        }
        let bytes = match fs::read(root.join(f)) {
            Ok(b) => b,
            Err(e) => {
                findings.push(format!("{f}: cannot be read: {e}"));
                continue;
            }
        };
        let notice_exempt = f == "NOTICE" && sha256::hex(&bytes) == NOTICE_SHA256;
        let text = String::from_utf8_lossy(&bytes);
        for (ln, line) in text.lines().enumerate() {
            let at = ln + 1;
            if abs.iter().any(|p| line.contains(p.as_str())) {
                findings.push(format!("{f}:{at}: absolute path"));
            }
            let lower = line.to_lowercase();
            if parents.iter().any(|p| lower.contains(p.as_str())) {
                findings.push(format!("{f}:{at}: reference to the parent repository"));
            }
            if !notice_exempt {
                for (n, id) in &denylist.entries {
                    if contains_word(&lower, id) {
                        findings.push(format!(
                            "{f}:{at}: private identifier (denylist entry #{n})"
                        ));
                    }
                }
            }
        }
    }
    findings
}

/// License and notice checks: `LICENSE` is the pinned PolyForm text and
/// `NOTICE` holds exactly the owner's Required Notice lines.
pub fn check_license(root: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    for (file, pin) in [("LICENSE", LICENSE_SHA256), ("NOTICE", NOTICE_SHA256)] {
        match fs::read(root.join(file)) {
            Ok(b) if sha256::hex(&b) == pin => {}
            Ok(_) => findings.push(format!("{file}: not the pinned text (sha256 {pin})")),
            Err(_) => findings.push(format!("{file}: missing")),
        }
    }
    findings
}

/// Why an export did not happen.
#[derive(Debug)]
pub enum ExportError {
    /// The destination is unusable (not empty, or inside the source tree).
    Destination(String),
    /// The scan or license check refused; every finding names a file.
    Refused(Vec<String>),
    /// Reading or writing failed.
    Io(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Destination(m) | ExportError::Io(m) => f.write_str(m),
            ExportError::Refused(findings) => {
                write!(f, "refused ({} finding(s))", findings.len())?;
                for x in findings {
                    write!(f, "\n  {x}")?;
                }
                Ok(())
            }
        }
    }
}

/// A finished export.
#[derive(Debug)]
pub struct ExportReport {
    /// The destination directory.
    pub dest: PathBuf,
    /// Files copied (not counting the regenerated `SHA256SUMS`).
    pub files: usize,
}

fn absolute(p: &Path) -> io::Result<PathBuf> {
    let p = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()?.join(p)
    };
    // Canonicalise the longest existing ancestor, then re-append the rest.
    let mut existing = p.clone();
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    while !existing.exists() {
        let Some(name) = existing.file_name() else {
            break;
        };
        rest.push(name.to_os_string());
        if !existing.pop() {
            break;
        }
    }
    let mut out = fs::canonicalize(&existing)?;
    for r in rest.into_iter().rev() {
        out.push(r);
    }
    if out.components().any(|c| c == Component::ParentDir) {
        return Err(io::Error::other(
            "the destination path must not contain '..'",
        ));
    }
    Ok(out)
}

/// The nearest ancestor of `dir` (inclusive) that holds a `.git` entry.
fn enclosing_repository(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|a| a.join(".git").exists())
        .map(Path::to_path_buf)
}

/// Export the module at `root` into the empty or absent directory `dest`,
/// outside any repository that contains the module. Scans first and writes
/// nothing when the scan refuses. Then copies the file set, regenerates
/// `SHA256SUMS` in `dest`, and verifies it there.
pub fn export(root: &Path, dest: &Path, denylist: &Denylist) -> Result<ExportReport, ExportError> {
    let io = |e: io::Error| ExportError::Io(e.to_string());
    let root = fs::canonicalize(root).map_err(io)?;
    let dest = absolute(dest).map_err(|e| ExportError::Destination(e.to_string()))?;
    if dest.starts_with(&root) {
        return Err(ExportError::Destination(
            "the destination is inside the module".into(),
        ));
    }
    if let Some(repo) = enclosing_repository(&root)
        && dest.starts_with(&repo)
    {
        return Err(ExportError::Destination(
            "the destination is inside the repository that holds the module".into(),
        ));
    }
    if dest.exists() {
        let mut it = fs::read_dir(&dest).map_err(io)?;
        if it.next().is_some() {
            return Err(ExportError::Destination(
                "the destination directory is not empty".into(),
            ));
        }
    }
    if denylist.is_empty() {
        return Err(ExportError::Refused(vec![
            "the denylist has no entries; the private-identifier scan cannot run".into(),
        ]));
    }
    let files = module_files(&root).map_err(io)?;
    let mut findings = check_license(&root);
    findings.extend(scan(&root, &files, denylist));
    if !findings.is_empty() {
        return Err(ExportError::Refused(findings));
    }
    for f in &files {
        let to = dest.join(f);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(io)?;
        }
        fs::copy(root.join(f), &to).map_err(io)?;
    }
    fs::create_dir_all(&dest).map_err(io)?;
    write_sums(&dest).map_err(io)?;
    match verify_sums(&dest) {
        Ok(n) if n == files.len() => {}
        Ok(n) => {
            return Err(ExportError::Refused(vec![format!(
                "{SUMS}: the export holds {n} files, the module {}",
                files.len()
            )]));
        }
        Err(f) => return Err(ExportError::Refused(f)),
    }
    Ok(ExportReport {
        dest,
        files: files.len(),
    })
}
