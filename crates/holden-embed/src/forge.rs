//! Forging the embedded agent's constitution for the host's domain, through
//! the forge's `export` and files only (docs/ARCHITECTURE.md §3.7): no crate
//! of this workspace depends on the forge.
//!
//! 1. `forge export <work>/export-<pid> --denylist <file>`: a clean,
//!    standalone copy of the forge, scanned for private identifiers;
//! 2. `forge verify <export>`: its `SHA256SUMS`, `LICENSE` and `NOTICE` hold;
//! 3. the export's holdout-engineering pack is read as a file, and its
//!    `domain` set to the host's domain; the result is written to the agent's
//!    state (`host-pack.json`);
//! 4. `forge pack --pack <host pack>` validates it;
//! 5. `forge constitution --role evaluator --pack <host pack>` emits the
//!    constitution, which must name the host's domain.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::util::{io_at, write_if_changed};
use crate::{Error, refused};

/// The pack inside a forge export.
pub const EXPORT_PACK: &str = "packs/holdout-engineering/PACK.json";

/// The role the embedded agent's constitution is emitted for.
pub const AGENT_ROLE: &str = "evaluator";

fn forge_run(forge: &Path, args: &[&std::ffi::OsStr]) -> Result<String, Error> {
    let out = Command::new(forge)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .args(args)
        .output()
        .map_err(|e| Error::Process {
            program: forge.display().to_string(),
            why: e.to_string(),
        })?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(Error::Process {
            program: format!(
                "forge {}",
                args.first()
                    .map(|a| a.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ),
            why: format!(
                "exit {:?}: {}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        })
    }
}

/// Forge the evaluator constitution for `domain` with the `forge` binary at
/// `forge`, using scratch space under `work`, and write the host-domain pack
/// to `pack_out`. Returns the constitution's text.
///
/// # Errors
/// [`Error`] when the forge refuses or the constitution does not name the
/// domain.
pub fn forge_constitution(
    forge: &Path,
    denylist: &Path,
    work: &Path,
    domain: &str,
    pack_out: &Path,
) -> Result<String, Error> {
    fs::create_dir_all(work).map_err(io_at(work))?;
    let export = work.join(format!("export-{}", std::process::id()));
    if export.exists() {
        fs::remove_dir_all(&export).map_err(io_at(&export))?;
    }
    let result = (|| {
        forge_run(
            forge,
            &[
                "export".as_ref(),
                export.as_os_str(),
                "--denylist".as_ref(),
                denylist.as_os_str(),
            ],
        )?;
        forge_run(forge, &["verify".as_ref(), export.as_os_str()])?;
        let pack_file = export.join(EXPORT_PACK);
        let text = fs::read_to_string(&pack_file).map_err(io_at(&pack_file))?;
        let pack: Value = serde_json::from_str(&text)
            .map_err(|e| refused(format!("the exported pack is not JSON: {e}")))?;
        // The forge requires its members in schema order, so the domain is
        // replaced in place: the one top-level `"domain": <value>` member.
        let old = serde_json::to_string(&pack["domain"])
            .map_err(|e| refused(format!("the exported pack: {e}")))?;
        let new = serde_json::to_string(&Value::String(domain.to_owned()))
            .map_err(|e| refused(format!("the host pack: {e}")))?;
        let member = format!("\"domain\": {old}");
        if !pack["domain"].is_string() || text.matches(&member).count() != 1 {
            return Err(refused("the exported pack has no single domain member"));
        }
        let host_pack = text.replacen(&member, &format!("\"domain\": {new}"), 1);
        write_if_changed(pack_out, host_pack.as_bytes(), 0o600)?;
        forge_run(
            forge,
            &["pack".as_ref(), "--pack".as_ref(), pack_out.as_os_str()],
        )?;
        let constitution = forge_run(
            forge,
            &[
                "constitution".as_ref(),
                "--role".as_ref(),
                AGENT_ROLE.as_ref(),
                "--pack".as_ref(),
                pack_out.as_os_str(),
            ],
        )?;
        if !constitution.contains(domain) {
            return Err(refused(
                "the forged constitution does not name the host's domain",
            ));
        }
        Ok(constitution)
    })();
    let _ = fs::remove_dir_all(&export);
    result
}
