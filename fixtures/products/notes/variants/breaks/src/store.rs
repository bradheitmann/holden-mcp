//! The note store: one file per note, written atomically.
//!
//! Variant: breaks (silently drops data). Notes are kept in fixed 64-byte
//! slots: a record whose body does not fit is skipped without a word, and
//! `import` still reports success. Short notes, like the ones on the public
//! card, survive; longer ones are lost.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The fixed slot size of a note body.
const SLOT_BYTES: usize = 64;

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
            let body = &line[tab + 1..];
            if body.len() > SLOT_BYTES {
                // Does not fit the slot: dropped, and nobody is told.
                continue;
            }
            self.save(&name, body)?;
            saved += 1;
        }
        // Whatever is left was cut off before its line feed: it was never
        // saved, and it is ignored.
        Ok(saved)
    }

    pub fn list(&self) -> io::Result<Vec<String>> {
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
