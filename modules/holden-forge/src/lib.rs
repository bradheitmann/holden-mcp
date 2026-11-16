//! holden-forge: a self-contained domain forge.
//!
//! - [`pack`]: the typed domain-pack schema, its strict reader and canonical writer;
//! - [`constitution`]: agent constitutions derived from a pack, rendered and read back;
//! - [`export`]: the module file set, `SHA256SUMS`, the content scan and `forge export`;
//! - [`json`], [`sha256`]: dependency-free support, so the module builds offline.
//!
//! The methodology it implements is vendored under `methodology/`.

pub mod constitution;
pub mod export;
pub mod json;
pub mod pack;
pub mod sha256;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

/// The module root this binary was built from.
pub fn module_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The holdout-engineering pack, relative to the module root.
pub const DEFAULT_PACK: &str = "packs/holdout-engineering/PACK.json";

/// Where the emitted constitution of `role` lives, relative to the module root.
pub fn constitution_path(pack_name: &str, role: pack::Role) -> String {
    format!("agents/{pack_name}/{}.md", role.as_str())
}

/// Read and validate a pack file.
pub fn load_pack(path: &Path) -> Result<pack::LoadedPack, Vec<String>> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| vec![format!("{}: cannot be read: {e}", path.display())])?;
    pack::load(&text).map_err(|errs| errs.iter().map(ToString::to_string).collect())
}
