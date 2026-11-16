//! Git, run with a scrubbed environment and with repository hooks and
//! filesystem monitors switched off, so reading or checking out the
//! evaluated commit never runs the builder's code outside confinement.

use std::path::{Path, PathBuf};
use std::process::Output;

use crate::EvalError;
use crate::spawn::scrubbed;

/// A git repository.
#[derive(Debug, Clone)]
pub struct Git {
    repo: PathBuf,
    env: Vec<(String, String)>,
}

/// One blob of a tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    /// The git mode (`100644`, `100755`, `120000`, `160000`).
    pub mode: String,
    /// The object id.
    pub oid: String,
    /// The path inside the tree.
    pub path: String,
}

/// The fixed identity and dates of fixture commits, so they are reproducible.
const FIXTURE_IDENTITY: [(&str, &str); 6] = [
    ("GIT_AUTHOR_NAME", "holden-fixture"),
    ("GIT_AUTHOR_EMAIL", "fixture@example.invalid"),
    ("GIT_AUTHOR_DATE", "2026-01-01T00:00:00+0000"),
    ("GIT_COMMITTER_NAME", "holden-fixture"),
    ("GIT_COMMITTER_EMAIL", "fixture@example.invalid"),
    ("GIT_COMMITTER_DATE", "2026-01-01T00:00:00+0000"),
];

impl Git {
    /// Git for `repo`; `home` is the only HOME git sees.
    #[must_use]
    pub fn new(repo: &Path, home: &Path) -> Self {
        let env = vec![
            ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
            ("HOME".to_owned(), home.display().to_string()),
            ("GIT_CONFIG_NOSYSTEM".to_owned(), "1".to_owned()),
            ("GIT_CONFIG_GLOBAL".to_owned(), "/dev/null".to_owned()),
            ("GIT_TERMINAL_PROMPT".to_owned(), "0".to_owned()),
            ("LC_ALL".to_owned(), "C".to_owned()),
        ];
        Self {
            repo: repo.to_path_buf(),
            env,
        }
    }

    /// The repository path.
    #[must_use]
    pub fn repo(&self) -> &Path {
        &self.repo
    }

    fn output(
        &self,
        args: &[&str],
        stdin: Option<&[u8]>,
        extra: &[(&str, &str)],
    ) -> Result<Output, EvalError> {
        let mut env = self.env.clone();
        env.extend(
            extra
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned())),
        );
        let mut cmd = scrubbed("git", &env);
        cmd.arg("-C")
            .arg(&self.repo)
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "core.fsmonitor=false",
            ])
            .args(args);
        let out = match stdin {
            None => cmd.output(),
            Some(input) => {
                use std::io::Write;
                cmd.stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());
                let mut child = cmd.spawn().map_err(|e| EvalError::Io("git", e.kind()))?;
                let mut pipe = child.stdin.take();
                let input = input.to_vec();
                let writer = std::thread::spawn(move || {
                    if let Some(p) = pipe.as_mut() {
                        let _ = p.write_all(&input);
                    }
                });
                let out = child.wait_with_output();
                let _ = writer.join();
                out
            }
        }
        .map_err(|e| EvalError::Io("git", e.kind()))?;
        Ok(out)
    }

    fn run(&self, args: &[&str]) -> Result<Vec<u8>, EvalError> {
        self.run_with(args, &[])
    }

    fn run_with(&self, args: &[&str], extra: &[(&str, &str)]) -> Result<Vec<u8>, EvalError> {
        let out = self.output(args, None, extra)?;
        if out.status.success() {
            Ok(out.stdout)
        } else {
            Err(EvalError::Git(format!(
                "git {} failed: {}",
                args.first().copied().unwrap_or_default(),
                String::from_utf8_lossy(&out.stderr).trim()
            )))
        }
    }

    /// The full commit id `rev` names.
    ///
    /// # Errors
    /// [`EvalError::Git`] when `rev` names no commit.
    pub fn resolve_commit(&self, rev: &str) -> Result<String, EvalError> {
        if rev.starts_with('-') {
            return Err(EvalError::Git(
                "a revision may not start with '-'".to_owned(),
            ));
        }
        let spec = format!("{rev}^{{commit}}");
        let out = self.run(&["rev-parse", "--verify", "--quiet", &spec])?;
        Ok(String::from_utf8_lossy(&out).trim().to_owned())
    }

    /// Write the tree of `commit` into `dest` through a private index file.
    /// Nothing is recorded in the repository: no worktree entry, no `.git`
    /// link and no shared index, so concurrent evaluations of the same
    /// repository cannot disturb each other (a shared `worktree prune` did).
    ///
    /// # Errors
    /// [`EvalError::Io`] when `dest` cannot be created; [`EvalError::Git`].
    pub fn export_tree(&self, dest: &Path, commit: &str, index: &Path) -> Result<(), EvalError> {
        std::fs::create_dir_all(dest).map_err(|e| EvalError::Io("create source dir", e.kind()))?;
        let index = index.display().to_string();
        let env = [("GIT_INDEX_FILE", index.as_str())];
        self.run_with(&["read-tree", commit], &env)?;
        let prefix = format!("--prefix={}/", dest.display());
        self.run_with(&["checkout-index", "--all", "--force", &prefix], &env)
            .map(|_| ())
    }

    /// Every entry of `commit`'s tree, recursively.
    ///
    /// # Errors
    /// [`EvalError::Git`].
    pub fn ls_tree(&self, commit: &str) -> Result<Vec<TreeEntry>, EvalError> {
        let out = self.run(&["ls-tree", "-r", "-z", "--full-tree", commit])?;
        let mut entries = Vec::new();
        for rec in out.split(|b| *b == 0).filter(|r| !r.is_empty()) {
            let rec = String::from_utf8_lossy(rec);
            let Some((meta, path)) = rec.split_once('\t') else {
                return Err(EvalError::Git("unreadable ls-tree output".to_owned()));
            };
            let mut it = meta.split(' ');
            let (Some(mode), Some(_kind), Some(oid)) = (it.next(), it.next(), it.next()) else {
                return Err(EvalError::Git("unreadable ls-tree output".to_owned()));
            };
            entries.push(TreeEntry {
                mode: mode.to_owned(),
                oid: oid.to_owned(),
                path: path.to_owned(),
            });
        }
        Ok(entries)
    }

    /// The contents of `oids`, in order (`git cat-file --batch`).
    ///
    /// # Errors
    /// [`EvalError::Git`] when an object is missing or the output is unreadable.
    pub fn cat_blobs(&self, oids: &[String]) -> Result<Vec<Vec<u8>>, EvalError> {
        let mut input = Vec::new();
        for o in oids {
            input.extend_from_slice(o.as_bytes());
            input.push(b'\n');
        }
        let out = self.output(&["cat-file", "--batch"], Some(&input), &[])?;
        if !out.status.success() {
            return Err(EvalError::Git("git cat-file failed".to_owned()));
        }
        let mut rest = out.stdout.as_slice();
        let mut blobs = Vec::with_capacity(oids.len());
        for _ in oids {
            let nl = rest
                .iter()
                .position(|b| *b == b'\n')
                .ok_or_else(|| EvalError::Git("unreadable cat-file output".to_owned()))?;
            let header = String::from_utf8_lossy(&rest[..nl]).into_owned();
            rest = &rest[nl + 1..];
            let size: usize = header
                .rsplit(' ')
                .next()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| EvalError::Git(format!("missing object: {header}")))?;
            if rest.len() < size + 1 {
                return Err(EvalError::Git("short cat-file output".to_owned()));
            }
            blobs.push(rest[..size].to_vec());
            rest = &rest[size + 1..];
        }
        Ok(blobs)
    }

    /// The bytes of `path` at `commit`, or `None` when the commit has no
    /// such file.
    ///
    /// # Errors
    /// [`EvalError::Git`] when git cannot run.
    pub fn file_at(&self, commit: &str, path: &str) -> Result<Option<Vec<u8>>, EvalError> {
        let spec = format!("{commit}:{path}");
        let exists = self.output(&["cat-file", "-e", &spec], None, &[])?;
        if !exists.status.success() {
            return Ok(None);
        }
        self.run(&["cat-file", "blob", &spec]).map(Some)
    }

    /// `git init` (branch `main`).
    ///
    /// # Errors
    /// [`EvalError::Git`].
    pub fn init(&self) -> Result<(), EvalError> {
        self.run(&["init", "--quiet", "--initial-branch=main"])
            .map(|_| ())
    }

    /// Stage everything and commit with the fixed fixture identity; returns
    /// the new commit id.
    ///
    /// # Errors
    /// [`EvalError::Git`].
    pub fn commit_all(&self, message: &str) -> Result<String, EvalError> {
        self.run(&["add", "--all"])?;
        self.run_with(
            &["commit", "--quiet", "--no-gpg-sign", "-m", message],
            &FIXTURE_IDENTITY,
        )?;
        self.resolve_commit("HEAD")
    }

    /// Check out `commit` detached, discarding nothing (the tree is clean).
    ///
    /// # Errors
    /// [`EvalError::Git`].
    pub fn checkout_detached(&self, commit: &str) -> Result<(), EvalError> {
        self.run(&["checkout", "--quiet", "--detach", commit])
            .map(|_| ())
    }
}
