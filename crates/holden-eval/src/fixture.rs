//! The fixture repository, created at run time from the committed fixture
//! product (`fixtures/products/notes`).
//!
//! Commit `keeps` holds every file of the product except `variants/`. Each
//! directory under `variants/` becomes one more commit, made on top of
//! `keeps` by copying that overlay's files over it. Commit identity and
//! dates are fixed, so the commits are reproducible.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::EvalError;
use crate::binding::Probe;
use crate::git::Git;

/// The committed fixture product, relative to the workspace root.
pub const NOTES_FIXTURE: &str = "fixtures/products/notes";

/// The name of the base commit.
pub const BASE: &str = "keeps";

/// The fixture variant that does nothing for the promise `probe` observes,
/// and keeps every other promise (slice-5): the do-nothing product of the
/// vacuity control, and the targeted broken variant of a plan whose
/// targeted check observes `probe`.
#[must_use]
pub const fn do_nothing_variant(probe: Probe) -> &'static str {
    match probe {
        Probe::InterruptedSave => "import-noop",
        Probe::ListEvery => "list-noop",
        Probe::ExportFailureExplained => "export-noop",
        Probe::DeleteAsksFirst => "delete-noop",
        Probe::CargoWorkspace => "workspace-noop",
    }
}

/// A fixture repository and the commit of each variant.
#[derive(Debug, Clone)]
pub struct FixtureRepo {
    /// The repository.
    pub path: PathBuf,
    /// Variant name → commit id. Always holds [`BASE`].
    pub commits: BTreeMap<String, String>,
    git: Git,
}

fn io(what: &'static str) -> impl Fn(std::io::Error) -> EvalError {
    move |e| EvalError::Io(what, e.kind())
}

/// Copy every file under `from` into `to`, skipping top-level names in `skip`.
fn copy_tree(from: &Path, to: &Path, skip: &[&str]) -> Result<(), EvalError> {
    let mut entries: Vec<_> = fs::read_dir(from)
        .map_err(io("fixture source"))?
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(fs::DirEntry::file_name);
    for e in entries {
        let name = e.file_name();
        if skip.iter().any(|s| name == *s) {
            continue;
        }
        let src = e.path();
        let dst = to.join(&name);
        let ty = e.file_type().map_err(io("fixture source"))?;
        if ty.is_dir() {
            fs::create_dir_all(&dst).map_err(io("fixture repository"))?;
            copy_tree(&src, &dst, &[])?;
        } else if ty.is_file() {
            fs::copy(&src, &dst).map_err(io("fixture repository"))?;
        } else {
            return Err(EvalError::Fixture(format!(
                "{} is neither a file nor a directory",
                src.display()
            )));
        }
    }
    Ok(())
}

impl FixtureRepo {
    /// Create the fixture repository at `dest` (which must not exist) from
    /// the committed product at `source`.
    ///
    /// # Errors
    /// [`EvalError`] when a file cannot be copied or git refuses.
    pub fn create(source: &Path, dest: &Path) -> Result<Self, EvalError> {
        if dest.exists() {
            return Err(EvalError::Fixture(format!("{} exists", dest.display())));
        }
        fs::create_dir_all(dest).map_err(io("fixture repository"))?;
        let home = dest.with_extension("home");
        fs::create_dir_all(&home).map_err(io("fixture home"))?;
        let git = Git::new(dest, &home);
        git.init()?;
        copy_tree(source, dest, &["variants", "target", ".git"])?;
        let base = git.commit_all(BASE)?;
        let mut commits = BTreeMap::new();
        commits.insert(BASE.to_owned(), base.clone());
        let variants = source.join("variants");
        let mut names: Vec<String> = fs::read_dir(&variants)
            .map_err(io("fixture variants"))?
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for name in names {
            git.checkout_detached(&base)?;
            copy_tree(&variants.join(&name), dest, &[])?;
            let commit = git.commit_all(&name)?;
            commits.insert(name, commit);
        }
        git.checkout_detached(&base)?;
        Ok(Self {
            path: dest.to_path_buf(),
            commits,
            git,
        })
    }

    /// The commit of `variant`.
    ///
    /// # Errors
    /// [`EvalError::Fixture`] for an unknown variant.
    pub fn commit(&self, variant: &str) -> Result<String, EvalError> {
        self.commits
            .get(variant)
            .cloned()
            .ok_or_else(|| EvalError::Fixture(format!("no variant {variant}")))
    }

    /// Make one more commit on top of `parent` that writes `files`
    /// (path → bytes); returns its id and leaves `parent` checked out.
    ///
    /// # Errors
    /// [`EvalError`] when a file cannot be written or git refuses.
    pub fn commit_on(
        &self,
        parent: &str,
        message: &str,
        files: &[(&str, &[u8])],
    ) -> Result<String, EvalError> {
        self.git.checkout_detached(parent)?;
        for (rel, bytes) in files {
            let p = self.path.join(rel);
            if let Some(dir) = p.parent() {
                fs::create_dir_all(dir).map_err(io("fixture repository"))?;
            }
            fs::write(&p, bytes).map_err(io("fixture repository"))?;
        }
        let commit = self.git.commit_all(message)?;
        self.git.checkout_detached(parent)?;
        Ok(commit)
    }
}
