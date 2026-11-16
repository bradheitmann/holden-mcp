//! The host fixture project (slice-7): a small, real, strongly typed git
//! project, created at run time from committed files, so no private
//! project is ever touched.
//!
//! Its product is the committed notes fixture product without its
//! `variants/` ([`NOTES_FIXTURE`]); its own planning document and README
//! come from [`HOST_FIXTURE`]. Everything is committed once with a fixed
//! identity and date.

use std::fs;
use std::path::{Path, PathBuf};

use holden_eval::git::Git;

use crate::util::{copy_tree, io_at};
use crate::{Error, refused};

/// The committed notes product, relative to the workspace root.
pub const NOTES_FIXTURE: &str = "fixtures/products/notes";

/// The committed host files, relative to the workspace root.
pub const HOST_FIXTURE: &str = "fixtures/hosts/notes-host";

/// The product files taken from the notes fixture.
const PRODUCT_FILES: [&str; 4] = ["Cargo.toml", "Cargo.lock", "notes.toml", "src"];

/// The host's `.gitignore`.
pub const GITIGNORE: &str = "target/\n";

/// Create the host repository at `dest` (which must not exist) from the
/// workspace at `workspace`. Returns its first commit.
///
/// # Errors
/// [`Error`] when `dest` exists, a file cannot be copied or git refuses.
pub fn create_host(workspace: &Path, dest: &Path) -> Result<String, Error> {
    if dest.exists() {
        return Err(refused(format!("{} exists", dest.display())));
    }
    let notes = workspace.join(NOTES_FIXTURE);
    let host_files = workspace.join(HOST_FIXTURE);
    fs::create_dir_all(dest).map_err(io_at(dest))?;
    for f in PRODUCT_FILES {
        let (from, to) = (notes.join(f), dest.join(f));
        if from.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(io_at(&from))?;
        }
    }
    copy_tree(&host_files, dest)?;
    fs::write(dest.join(".gitignore"), GITIGNORE).map_err(io_at(dest))?;
    let tag =
        holden_seal::random_bytes::<8>().map_err(|_| refused("the OS random source failed"))?;
    let home: PathBuf = std::env::temp_dir().join(format!("holden-host-git-{}", hex::encode(tag)));
    fs::create_dir_all(&home).map_err(io_at(&home))?;
    let git = Git::new(dest, &home);
    let made = git
        .init()
        .and_then(|()| git.commit_all("Pocket Notes: a small note keeper"))
        .map_err(|e| refused(e.to_string()));
    let _ = fs::remove_dir_all(&home);
    made
}
