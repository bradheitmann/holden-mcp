//! The product binding: how each approved promise is observed at the notes
//! product's boundary.
//!
//! A generated plan's checks each name an approved promise and quote it
//! verbatim. The binding maps the promise text (the owner's words, exactly)
//! to a [`Probe`]: a fixed procedure of process runs and file reads at the
//! boundary. A check whose promise has no probe cannot be observed, and an
//! unobserved check is unsure, which never passes.

use holden_gen::GenPlan;
use serde::{Deserialize, Serialize};

use crate::plant::{BoundaryCard, notes_card};

/// A boundary procedure for one promise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Probe {
    /// Import notes, cut the stream off halfway through one record, then read
    /// back every note completed before the cut.
    InterruptedSave,
    /// Import every note, then list them through the public command.
    ListEvery,
    /// Export to a folder that does not exist, then retry as advised.
    ExportFailureExplained,
    /// Delete a note without, then with, confirmation.
    DeleteAsksFirst,
    /// A Cargo workspace's real tests and CLI entry points.
    CargoWorkspace,
}

impl Probe {
    /// Every probe, in run order.
    pub const ALL: [Self; 4] = [
        Self::InterruptedSave,
        Self::ListEvery,
        Self::ExportFailureExplained,
        Self::DeleteAsksFirst,
    ];

    /// Its stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InterruptedSave => "interrupted_save",
            Self::ListEvery => "list_every",
            Self::ExportFailureExplained => "export_failure_explained",
            Self::DeleteAsksFirst => "delete_asks_first",
            Self::CargoWorkspace => "cargo_workspace",
        }
    }
}

/// A product binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    /// The product name.
    pub product: &'static str,
    /// Exact promise text → probe.
    pub promises: Vec<(&'static str, Probe)>,
    /// The builder-visible card.
    pub card: BoundaryCard,
    /// The project's public config paths, diffed between the baseline and
    /// the evaluated commit.
    pub public_config: Vec<&'static str>,
}

/// The notes product's binding, over the synthetic planning document's
/// promises.
#[must_use]
pub fn notes_binding() -> Binding {
    Binding {
        product: "notes",
        promises: vec![
            (
                "A saved note is never lost, even if the app closes halfway through saving.",
                Probe::InterruptedSave,
            ),
            (
                "When an export fails, the person sees why it failed and how to retry.",
                Probe::ExportFailureExplained,
            ),
            (
                "An agent can list every note through the public API without reading the screen.",
                Probe::ListEvery,
            ),
            (
                "Deleting a note asks for confirmation before anything is removed.",
                Probe::DeleteAsksFirst,
            ),
        ],
        card: notes_card(),
        public_config: vec![
            "Cargo.toml",
            "Cargo.lock",
            "build.rs",
            ".cargo/config.toml",
            ".cargo/config",
            "rust-toolchain.toml",
            "rust-toolchain",
            "notes.toml",
        ],
    }
}

/// The binding selected for this host and evaluation.
#[derive(Clone, Debug)]
pub enum ActiveBinding {
    /// A directly supplied established binding (used by the evaluator's
    /// existing fixture/control callers).
    Direct(Binding),
    /// A boundary contract loaded from public host configuration.
    Declared(crate::product::DeclaredBinding),
    /// A generic Cargo workspace loaded from public host configuration.
    Cargo(crate::product::CargoWorkspaceBinding),
}

impl ActiveBinding {
    /// Resolve the public binding written by embed. Older direct evaluator
    /// callers, which do not create a host config, keep their explicit input.
    ///
    /// # Errors
    /// A present malformed declaration is refused; there is no fallback.
    pub fn load(repo: &std::path::Path, direct: Binding) -> Result<Self, crate::EvalError> {
        if !repo.join(".holden/config.json").is_file() {
            return Ok(Self::Direct(direct));
        }
        match crate::product::from_host(repo)
            .map_err(|e| crate::EvalError::Fixture(e.to_string()))?
        {
            crate::ProductBinding::CargoWorkspace(binding) => Ok(Self::Cargo(binding)),
            crate::ProductBinding::Declared(binding) => Ok(Self::Declared(binding)),
        }
    }

    /// Public product label.
    #[must_use]
    pub fn product(&self) -> &str {
        match self {
            Self::Direct(b) => b.product,
            Self::Declared(b) => &b.product,
            Self::Cargo(_) => "cargo-workspace",
        }
    }

    /// Public config paths.
    #[must_use]
    pub fn public_config(&self) -> Vec<&str> {
        match self {
            Self::Direct(b) => b.public_config.clone(),
            Self::Declared(b) => b.public_config.iter().map(String::as_str).collect(),
            Self::Cargo(binding) if binding.manifests.is_empty() => vec![
                "Cargo.toml",
                "Cargo.lock",
                "build.rs",
                ".cargo/config.toml",
                ".cargo/config",
                "rust-toolchain.toml",
                "rust-toolchain",
            ],
            Self::Cargo(binding) => {
                let mut paths = vec![
                    ".cargo/config.toml",
                    ".cargo/config",
                    "rust-toolchain.toml",
                    "rust-toolchain",
                ];
                for manifest in &binding.manifests {
                    paths.push(manifest.manifest.to_str().unwrap_or_default());
                }
                paths
            }
        }
    }

    /// Cargo entry points, when this is the generic workspace binding.
    #[must_use]
    pub const fn cargo(&self) -> Option<&crate::product::CargoWorkspaceBinding> {
        match self {
            Self::Cargo(binding) => Some(binding),
            _ => None,
        }
    }

    /// Each Cargo manifest to build independently. The original root binding
    /// is represented as one `Cargo.toml` entry; declared bindings retain
    /// their own manifest and entry-point set.
    #[must_use]
    pub fn cargo_manifests(&self) -> Vec<crate::product::CargoManifestBinding> {
        match self {
            Self::Cargo(binding) if binding.manifests.is_empty() => {
                vec![crate::product::CargoManifestBinding {
                    manifest: std::path::PathBuf::from("Cargo.toml"),
                    entry_points: binding.entry_points.clone(),
                }]
            }
            Self::Cargo(binding) => binding.manifests.clone(),
            _ => Vec::new(),
        }
    }

    /// Bind every generated check. Generic probes come from the admitted
    /// plan itself: every approved check drives the workspace's tests and
    /// actual CLI entry points, never an invented product-specific probe.
    #[must_use]
    pub fn bind(&self, plan: &GenPlan) -> Vec<BoundCheck> {
        match self {
            Self::Direct(binding) => binding.bind(plan),
            Self::Declared(binding) => bind_with(plan, |text| {
                binding
                    .promises
                    .iter()
                    .find(|(candidate, _)| candidate == text)
                    .map(|(_, probe)| *probe)
            }),
            Self::Cargo(_) => bind_with(plan, |_| Some(Probe::CargoWorkspace)),
        }
    }
}

/// One plan check, bound to its probe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundCheck {
    /// The plan's check id.
    pub id: String,
    /// The promise id, hex.
    pub promise: String,
    /// The promise text, as quoted by the check.
    pub promise_text: String,
    /// The probe, or `None` when the binding cannot observe this promise.
    pub probe: Option<Probe>,
}

/// The text between the first and the last double quote of `expect`.
fn quoted(expect: &str) -> &str {
    match (expect.find('"'), expect.rfind('"')) {
        (Some(a), Some(b)) if b > a => &expect[a + 1..b],
        _ => "",
    }
}

fn bind_with(plan: &GenPlan, probe_for: impl Fn(&str) -> Option<Probe>) -> Vec<BoundCheck> {
    plan.checks()
        .into_iter()
        .map(|c| {
            let text = quoted(&c.expect).to_owned();
            BoundCheck {
                id: c.id.clone(),
                promise: c.promise.to_hex(),
                probe: probe_for(&text),
                promise_text: text,
            }
        })
        .collect()
}

impl Binding {
    /// The probe for `promise_text`, if any.
    #[must_use]
    pub fn probe_for(&self, promise_text: &str) -> Option<Probe> {
        self.promises
            .iter()
            .find(|(t, _)| *t == promise_text)
            .map(|(_, p)| *p)
    }

    /// Bind every check of `plan`.
    #[must_use]
    pub fn bind(&self, plan: &GenPlan) -> Vec<BoundCheck> {
        bind_with(plan, |text| self.probe_for(text))
    }
}
