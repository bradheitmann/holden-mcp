//! The forge module (`modules/holden-forge`) as seen from the main workspace:
//! the two-way isolation check (slice-6 P1) and the forge gate steps.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util;

/// The forge module, relative to the repository root.
pub const FORGE_DIR: &str = "modules/holden-forge";

/// `cargo metadata --no-deps` for the workspace whose manifest is `manifest`.
fn metadata(root: &Path, manifest: &Path) -> Result<serde_json::Value, String> {
    let out = util::capture(
        Command::new("cargo")
            .current_dir(root)
            .args([
                "metadata",
                "--format-version",
                "1",
                "--no-deps",
                "--offline",
            ])
            .arg("--manifest-path")
            .arg(manifest),
    )
    .map_err(|e| format!("cargo metadata could not run: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo metadata failed for {}: {}",
            manifest.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("cargo metadata output unreadable: {e}"))
}

fn canonical(p: &str) -> PathBuf {
    let p = PathBuf::from(p);
    std::fs::canonicalize(&p).unwrap_or(p)
}

/// Check dependency isolation in both directions with `cargo metadata`:
/// - no main-workspace package lives under the forge directory, depends on a
///   path under it, or depends on a package named `holden-forge`;
/// - the forge is its own workspace, and none of its packages lives outside
///   its directory or has a path dependency outside it.
///
/// Returns every finding; empty means isolated.
pub fn isolation_findings(root: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    let forge = canonical(&root.join(FORGE_DIR).display().to_string());
    let deps = |p: &serde_json::Value| -> Vec<serde_json::Value> {
        p["dependencies"].as_array().cloned().unwrap_or_default()
    };

    match metadata(root, &root.join("Cargo.toml")) {
        Ok(meta) => {
            let packages = meta["packages"].as_array().cloned().unwrap_or_default();
            if packages.is_empty() {
                findings.push("main workspace: cargo metadata lists no package".to_owned());
            }
            for p in &packages {
                let name = p["name"].as_str().unwrap_or("<unnamed>");
                if canonical(p["manifest_path"].as_str().unwrap_or("")).starts_with(&forge) {
                    findings.push(format!(
                        "main workspace: package {name} lives under {FORGE_DIR}"
                    ));
                }
                for d in deps(p) {
                    let dep = d["name"].as_str().unwrap_or("<unnamed>");
                    if dep == "holden-forge" {
                        findings.push(format!(
                            "main workspace: {name} depends on package holden-forge"
                        ));
                    }
                    if let Some(path) = d["path"].as_str()
                        && canonical(path).starts_with(&forge)
                    {
                        findings.push(format!(
                            "main workspace: {name} has a path dependency on {dep} under {FORGE_DIR}"
                        ));
                    }
                }
            }
        }
        Err(e) => findings.push(format!("main workspace: {e}")),
    }

    match metadata(root, &forge.join("Cargo.toml")) {
        Ok(meta) => {
            if canonical(meta["workspace_root"].as_str().unwrap_or("")) != forge {
                findings.push(format!("forge: {FORGE_DIR} is not its own workspace root"));
            }
            let packages = meta["packages"].as_array().cloned().unwrap_or_default();
            if packages.is_empty() {
                findings.push("forge: cargo metadata lists no package".to_owned());
            }
            for p in &packages {
                let name = p["name"].as_str().unwrap_or("<unnamed>");
                if !canonical(p["manifest_path"].as_str().unwrap_or("")).starts_with(&forge) {
                    findings.push(format!("forge: package {name} lives outside {FORGE_DIR}"));
                }
                for d in deps(p) {
                    if let Some(path) = d["path"].as_str()
                        && !canonical(path).starts_with(&forge)
                    {
                        findings.push(format!(
                            "forge: {name} has a path dependency on {} outside {FORGE_DIR}",
                            d["name"].as_str().unwrap_or("<unnamed>")
                        ));
                    }
                }
            }
        }
        Err(e) => findings.push(format!("forge: {e}")),
    }
    findings
}
