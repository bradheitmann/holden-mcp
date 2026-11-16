//! Embed-time product bindings.
//!
//! v1 discovers Cargo workspaces with at least one real CLI entry point. A
//! host may declare manifests below its root; each is discovered and later
//! built independently. Discovery is offline and locked, and declared paths
//! may not escape the host or pull in packages outside the declared crate.

use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::binding::Probe;
use crate::plant::BoundaryCard;

/// The current public binding format.
pub const PRODUCT_BINDING_FORMAT: &str = "holden.product-binding/1";

/// One real binary target selected through Cargo metadata.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoEntryPoint {
    /// Owning package name.
    pub package: String,
    /// Binary target name.
    pub name: String,
}

/// One declared Cargo manifest and its own real CLI entry points.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoManifestBinding {
    /// Host-relative manifest path.
    pub manifest: PathBuf,
    /// This manifest's real CLI entry points, sorted by package and target.
    pub entry_points: Vec<CargoEntryPoint>,
}

/// A Cargo workspace binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoWorkspaceBinding {
    /// All real CLI entry points, sorted by package and target. For declared
    /// manifests this is the union; `manifests` retains each separate set.
    pub entry_points: Vec<CargoEntryPoint>,
    /// Declared non-root manifests. Empty for the original root binding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manifests: Vec<CargoManifestBinding>,
}

/// An existing, explicitly declared boundary contract. This carries data,
/// not product-name dispatch, and is used to migrate earlier hosts whose
/// fixed boundary probes are still supported by the evaluator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredBinding {
    /// Public product label.
    pub product: String,
    /// Exact promise text and its declared boundary procedure.
    pub promises: Vec<(String, Probe)>,
    /// Builder-visible boundary card.
    pub card: BoundaryCard,
    /// Public configuration files compared between commits.
    pub public_config: Vec<String>,
}

/// The typed product binding written into a host's public config.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "stack", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProductBinding {
    /// Generic Cargo workspace tests and real binary entry points.
    CargoWorkspace(CargoWorkspaceBinding),
    /// An explicitly declared fixed boundary contract.
    Declared(DeclaredBinding),
}

/// Why a host cannot be bound.
#[derive(Debug)]
pub enum ProductBindingError {
    /// Anything except a Cargo workspace with a binary entry point.
    UnsupportedStack,
    /// Cargo metadata could not run or returned invalid output.
    Metadata(String),
    /// The host's declaration is missing or malformed.
    Declaration(String),
}

impl fmt::Display for ProductBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedStack => f.write_str(
                "unsupported_stack: no root Cargo.toml; declare a crate with --manifest <relative-path> (repeatable)",
            ),
            Self::Metadata(why) => write!(f, "cargo_metadata: {why}"),
            Self::Declaration(why) => write!(f, "product_binding: {why}"),
        }
    }
}

impl std::error::Error for ProductBindingError {}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    workspace_root: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    manifest_path: PathBuf,
    targets: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
}

/// Read the declared binding from `.holden/config.json`.
///
/// # Errors
/// A missing or malformed declaration is never replaced by a fallback.
pub fn from_host(host: &Path) -> Result<ProductBinding, ProductBindingError> {
    let path = host.join(".holden/config.json");
    let bytes =
        std::fs::read(&path).map_err(|e| ProductBindingError::Declaration(e.kind().to_string()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| ProductBindingError::Declaration(e.to_string()))?;
    serde_json::from_value(value["product_binding"].clone())
        .map_err(|e| ProductBindingError::Declaration(e.to_string()))
}

fn metadata(host: &Path, cargo: &Path, relative: &Path) -> Result<Metadata, ProductBindingError> {
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--offline",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(host.join(relative))
        .current_dir(host)
        .output()
        .map_err(|e| ProductBindingError::Metadata(e.kind().to_string()))?;
    if !output.status.success() {
        return Err(ProductBindingError::Metadata(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| ProductBindingError::Metadata(e.to_string()))
}

fn entry_points(
    metadata: &Metadata,
    declared_dir: Option<&Path>,
) -> Result<Vec<CargoEntryPoint>, ProductBindingError> {
    let mut entry_points = Vec::new();
    for package in metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
    {
        if declared_dir.is_some_and(|dir| !package.manifest_path.starts_with(dir)) {
            return Err(ProductBindingError::Declaration(format!(
                "cargo metadata reports package {} outside the declared manifest",
                package.name
            )));
        }
        for target in &package.targets {
            if target.kind.iter().any(|k| k == "bin") {
                entry_points.push(CargoEntryPoint {
                    package: package.name.clone(),
                    name: target.name.clone(),
                });
            }
        }
    }
    entry_points.sort();
    entry_points.dedup();
    if entry_points.is_empty() {
        return Err(ProductBindingError::UnsupportedStack);
    }
    Ok(entry_points)
}

/// Discover the root Cargo workspace through `cargo metadata --offline
/// --locked`.
///
/// # Errors
/// [`ProductBindingError::UnsupportedStack`] for a non-Cargo host or one
/// without a CLI, and [`ProductBindingError::Metadata`] for discovery errors.
pub fn discover(host: &Path, cargo: &Path) -> Result<ProductBinding, ProductBindingError> {
    if !host.join("Cargo.toml").is_file() {
        return Err(ProductBindingError::UnsupportedStack);
    }
    let metadata = metadata(host, cargo, Path::new("Cargo.toml"))?;
    Ok(ProductBinding::CargoWorkspace(CargoWorkspaceBinding {
        entry_points: entry_points(&metadata, None)?,
        manifests: Vec::new(),
    }))
}

fn declared_relative(host: &Path, value: &Path) -> Result<PathBuf, ProductBindingError> {
    if value.is_absolute()
        || value.file_name().is_none_or(|name| name != "Cargo.toml")
        || value
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(ProductBindingError::Declaration(
            "--manifest must be a host-relative path to Cargo.toml without ..".to_owned(),
        ));
    }
    let canonical_host = std::fs::canonicalize(host)
        .map_err(|e| ProductBindingError::Declaration(e.kind().to_string()))?;
    let manifest = std::fs::canonicalize(host.join(value))
        .map_err(|e| ProductBindingError::Declaration(e.kind().to_string()))?;
    if !manifest.starts_with(&canonical_host) || !manifest.is_file() {
        return Err(ProductBindingError::Declaration(
            "--manifest escapes the host or is not a file".to_owned(),
        ));
    }
    Ok(value.to_path_buf())
}

/// Discover each declared non-root Cargo manifest independently.
///
/// # Errors
/// Absolute paths, `..`, symlink escapes, manifests outside the host, and
/// Cargo workspaces that include packages outside the declared directory
/// are refused. Every declaration must have a real binary entry point.
pub fn discover_declared(
    host: &Path,
    cargo: &Path,
    declarations: &[PathBuf],
) -> Result<ProductBinding, ProductBindingError> {
    if declarations.is_empty() {
        return discover(host, cargo);
    }
    let canonical_host = std::fs::canonicalize(host)
        .map_err(|e| ProductBindingError::Declaration(e.kind().to_string()))?;
    let mut manifests = Vec::new();
    let mut all_entry_points = Vec::new();
    for declaration in declarations {
        let relative = declared_relative(host, declaration)?;
        if manifests
            .iter()
            .any(|m: &CargoManifestBinding| m.manifest == relative)
        {
            return Err(ProductBindingError::Declaration(
                "--manifest given twice for the same crate".to_owned(),
            ));
        }
        let metadata = metadata(host, cargo, &relative)?;
        let declared_dir = canonical_host.join(relative.parent().unwrap_or(Path::new("")));
        let canonical_workspace = std::fs::canonicalize(&metadata.workspace_root)
            .map_err(|e| ProductBindingError::Declaration(e.kind().to_string()))?;
        if canonical_workspace != declared_dir {
            return Err(ProductBindingError::Declaration(
                "cargo metadata reports a workspace outside the declared manifest".to_owned(),
            ));
        }
        let entry_points = entry_points(&metadata, Some(&declared_dir))?;
        all_entry_points.extend(entry_points.iter().cloned());
        manifests.push(CargoManifestBinding {
            manifest: relative,
            entry_points,
        });
    }
    all_entry_points.sort();
    all_entry_points.dedup();
    Ok(ProductBinding::CargoWorkspace(CargoWorkspaceBinding {
        entry_points: all_entry_points,
        manifests,
    }))
}

/// Convert an established fixed binding into portable declaration data.
#[must_use]
pub fn declare(binding: &crate::binding::Binding) -> ProductBinding {
    ProductBinding::Declared(DeclaredBinding {
        product: binding.product.to_owned(),
        promises: binding
            .promises
            .iter()
            .map(|(text, probe)| ((*text).to_owned(), *probe))
            .collect(),
        card: binding.card.clone(),
        public_config: binding
            .public_config
            .iter()
            .map(|path| (*path).to_owned())
            .collect(),
    })
}

/// The public path a test may use to expose unsupported-stack behavior.
#[must_use]
pub fn manifest_path(host: &Path) -> PathBuf {
    host.join("Cargo.toml")
}
