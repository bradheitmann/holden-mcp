//! The note store: one file per note, written atomically.
//!
//! Variant: refuses (errors). An import cut off halfway marks the whole
//! store damaged; from then on every read is refused with an error, so the
//! notes saved before the interruption can no longer be reached.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The marker file of a damaged store.
const DAMAGED: &str = "DAMAGED";

pub struct Store {
    dir: PathBuf,
}

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

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.note"))
    }

    /// Save one note: write a temporary file, then rename it into place.
    pub fn save(&self, name: &str, body: &[u8]) -> io::Result<()> {
        if !valid_name(name) {
            return Err(bad_name(name));
        }
        fs::create_dir_all(&self.dir)?;
        let tmp = self.dir.join(format!(".{name}.tmp"));
        fs::write(&tmp, body)?;
        fs::rename(&tmp, self.path(name))
    }

    /// Save every complete record of `input`; returns how many were saved.
    pub fn import(&self, input: &[u8]) -> io::Result<usize> {
        let mut saved = 0;
        let mut rest = input;
        while let Some(end) = rest.iter().position(|b| *b == b'\n') {
            let line = &rest[..end];
            rest = &rest[end + 1..];
            let Some(tab) = line.iter().position(|b| *b == b'\t') else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "a record has no tab",
                ));
            };
            let name = String::from_utf8_lossy(&line[..tab]).into_owned();
            self.save(&name, &line[tab + 1..])?;
            saved += 1;
        }
        if !rest.is_empty() {
            fs::create_dir_all(&self.dir)?;
            fs::write(self.dir.join(DAMAGED), b"interrupted import\n")?;
            return Err(io::Error::other(
                "the import was interrupted; the store is now marked damaged",
            ));
        }
        Ok(saved)
    }

    fn refuse_if_damaged(&self) -> io::Result<()> {
        if self.dir.join(DAMAGED).exists() {
            return Err(io::Error::other("the store is damaged; run repair first"));
        }
        Ok(())
    }

    pub fn list(&self) -> io::Result<Vec<String>> {
        self.refuse_if_damaged()?;
        let mut names = Vec::new();
        let entries = match fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(names),
            Err(e) => return Err(e),
        };
        for entry in entries {
            let entry = entry?;
            let file = entry.file_name().to_string_lossy().into_owned();
            if let Some(name) = file.strip_suffix(".note") {
                names.push(name.to_owned());
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn get(&self, name: &str) -> io::Result<String> {
        self.refuse_if_damaged()?;
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
        let mut out = Vec::new();
        let names = self.list()?;
        for n in &names {
            out.extend_from_slice(n.as_bytes());
            out.push(b'\t');
            out.extend_from_slice(self.get(n)?.as_bytes());
            out.push(b'\n');
        }
        fs::write(to, out)?;
        Ok(names.len())
    }
}
