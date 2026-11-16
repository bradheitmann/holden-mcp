//! The note store: one file per note under `notes/`, written atomically
//! and synced.
//!
//! Variant: keeps-alt (correct, a different implementation). It keeps every
//! promise the base product keeps, through different code: notes live in a
//! `notes/` subfolder as `<name>.txt`, each save is synced before it is
//! renamed into place, the listing is collected into an ordered set, and an
//! export is written to a temporary file and renamed. It is a known-good
//! control (slice-5 P2): failing it is a false fail.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub struct Store {
    dir: PathBuf,
}

const EXT: &str = ".txt";

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn bad_name(name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("'{name}' is not a note name"),
    )
}

/// Write `bytes` to a temporary file next to `to`, sync it, then rename it.
fn write_synced(to: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = to.with_extension("partial");
    let mut f = File::create(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    fs::rename(&tmp, to)
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn notes(&self) -> PathBuf {
        self.dir.join("notes")
    }

    fn path(&self, name: &str) -> PathBuf {
        self.notes().join(format!("{name}{EXT}"))
    }

    /// Save one note: write a synced temporary file, then rename it.
    pub fn save(&self, name: &str, body: &[u8]) -> io::Result<()> {
        if !valid_name(name) {
            return Err(bad_name(name));
        }
        fs::create_dir_all(self.notes())?;
        write_synced(&self.path(name), body)
    }

    /// Save every complete record of `input`; returns how many were saved.
    pub fn import(&self, input: &[u8]) -> io::Result<usize> {
        let mut saved = 0;
        for record in input.split_inclusive(|b| *b == b'\n') {
            // A record without its line feed was cut off: never saved.
            let Some(line) = record.strip_suffix(b"\n") else {
                break;
            };
            let mut parts = line.splitn(2, |b| *b == b'\t');
            let (Some(name), Some(body)) = (parts.next(), parts.next()) else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "a record has no tab",
                ));
            };
            self.save(&String::from_utf8_lossy(name), body)?;
            saved += 1;
        }
        Ok(saved)
    }

    pub fn list(&self) -> io::Result<Vec<String>> {
        let mut names = BTreeSet::new();
        let entries = match fs::read_dir(self.notes()) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        for entry in entries {
            let file = entry?.file_name().to_string_lossy().into_owned();
            if let Some(name) = file.strip_suffix(EXT) {
                names.insert(name.to_owned());
            }
        }
        Ok(names.into_iter().collect())
    }

    pub fn get(&self, name: &str) -> io::Result<String> {
        if !valid_name(name) {
            return Err(bad_name(name));
        }
        let bytes = fs::read(self.path(name))?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn delete(&self, name: &str) -> io::Result<()> {
        if !valid_name(name) {
            return Err(bad_name(name));
        }
        fs::remove_file(self.path(name))
    }

    pub fn export(&self, to: &Path) -> io::Result<usize> {
        let names = self.list()?;
        let mut out = Vec::new();
        for n in &names {
            let body = self.get(n)?;
            out.extend_from_slice(format!("{n}\t{body}\n").as_bytes());
        }
        write_synced(to, &out)?;
        Ok(names.len())
    }
}
