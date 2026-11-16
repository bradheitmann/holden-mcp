//! Boundary observation: the product is run as a process and read only
//! through its exit status, its stdout and stderr, and the files it leaves
//! in its world. Every observation is bounded (see [`crate::spawn`]).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::EvalError;
use crate::binding::Probe;
use crate::confine::{Confiner, HelperReport, Profile};
use crate::plant::Planted;
use crate::spawn::{BoundedBytes, Exit, Limits, Spawn};
use crate::world::World;

/// What was observed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observed {
    /// A process run.
    Process {
        /// Its arguments, with the product shown by name and paths relative
        /// to the world's run directory.
        argv: Vec<String>,
        /// What was written to its stdin.
        stdin: BoundedBytes,
        /// How it ended.
        exit: Exit,
        /// Its stdout.
        stdout: BoundedBytes,
        /// Its stderr.
        stderr: BoundedBytes,
        /// Wall time.
        elapsed_ms: u64,
    },
    /// A file in the world after the runs.
    File {
        /// Path relative to the world's run directory.
        path: String,
        /// Whether it exists.
        present: bool,
        /// Its contents, bounded.
        content: Option<BoundedBytes>,
    },
}

/// One boundary observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    /// `obs-<n>`.
    pub id: String,
    /// The probe it belongs to; `None` for the build.
    pub probe: Option<Probe>,
    /// Its label within the probe.
    pub label: String,
    /// What was seen.
    pub observed: Observed,
}

impl Observation {
    /// SHA-256 of the observation's canonical JSON, hex.
    #[must_use]
    pub fn digest(&self) -> String {
        hex::encode(Sha256::digest(serde_json::to_vec(self).unwrap_or_default()))
    }

    /// Its canonical length in bytes.
    #[must_use]
    pub fn len(&self) -> u64 {
        serde_json::to_vec(self).map_or(0, |v| v.len() as u64)
    }

    /// Never empty; present for clippy's `len_without_is_empty`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }
}

/// Runs probes against a built product inside one world.
#[derive(Debug)]
pub struct Driver<'a> {
    world: &'a World,
    confiner: &'a Confiner,
    product: PathBuf,
    limits: Limits,
    /// Observations so far.
    pub observations: Vec<Observation>,
    /// Launcher reports so far.
    pub reports: Vec<HelperReport>,
}

impl<'a> Driver<'a> {
    /// A driver for the product binary at `product`.
    #[must_use]
    pub fn new(world: &'a World, confiner: &'a Confiner, product: PathBuf, limits: Limits) -> Self {
        Self {
            world,
            confiner,
            product,
            limits,
            observations: Vec::new(),
            reports: Vec::new(),
        }
    }

    fn next_id(&self) -> String {
        format!("obs-{}", self.observations.len() + 1)
    }

    /// Record an observation made elsewhere (the build).
    pub fn push(&mut self, probe: Option<Probe>, label: &str, observed: Observed) {
        let id = self.next_id();
        self.observations.push(Observation {
            id,
            probe,
            label: label.to_owned(),
            observed,
        });
    }

    /// Run the product with `args` and `stdin`, confined, and record it.
    ///
    /// # Errors
    /// [`EvalError`] when the confined run could not start or was refused.
    pub fn run(
        &mut self,
        probe: Probe,
        label: &str,
        args: &[String],
        stdin: &[u8],
    ) -> Result<(), EvalError> {
        let spec = Spawn {
            program: self.product.clone(),
            args: args.iter().map(Into::into).collect(),
            cwd: self.world.path("run"),
            env: self.world.product_env(),
            stdin: stdin.to_vec(),
        };
        let run = self
            .confiner
            .run_as(self.world, spec, &self.limits, Profile::Product)?;
        self.reports.push(run.report);
        let mut argv = vec![
            self.product
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "product".to_owned()),
        ];
        argv.extend(args.iter().cloned());
        let stdin = BoundedBytes::keep(stdin, stdin.len() as u64, self.limits.capture_bytes);
        self.push(
            Some(probe),
            label,
            Observed::Process {
                argv,
                stdin,
                exit: run.outcome.exit,
                stdout: run.outcome.stdout,
                stderr: run.outcome.stderr,
                elapsed_ms: run.outcome.elapsed_ms,
            },
        );
        Ok(())
    }

    /// Read `rel` (relative to the run directory) and record it.
    pub fn file(&mut self, probe: Probe, label: &str, rel: &str) {
        let path = self.world.path("run").join(rel);
        let content = fs::read(&path)
            .ok()
            .map(|b| BoundedBytes::keep(&b, b.len() as u64, self.limits.capture_bytes));
        self.push(
            Some(probe),
            label,
            Observed::File {
                path: rel.to_owned(),
                present: content.is_some(),
                content,
            },
        );
    }

    /// Run probe `p` with the planted values.
    ///
    /// # Errors
    /// [`EvalError`] when a confined run could not start or was refused.
    pub fn probe(&mut self, p: Probe, planted: &Planted) -> Result<(), EvalError> {
        let store = p.as_str().to_owned();
        let args = |rest: &[&str]| -> Vec<String> {
            let mut a = vec!["--store".to_owned(), store.clone()];
            a.extend(rest.iter().map(|s| (*s).to_owned()));
            a
        };
        let all = records(planted, planted.notes.len(), false);
        match p {
            Probe::InterruptedSave => {
                let cut = records(planted, planted.interrupt_after, true);
                self.run(p, "import", &args(&["import"]), &cut)?;
                self.run(p, "list", &args(&["list"]), &[])?;
                for (k, n) in planted
                    .notes
                    .iter()
                    .take(planted.interrupt_after)
                    .enumerate()
                {
                    self.run(p, &format!("get-{k}"), &args(&["get", &n.name]), &[])?;
                }
            }
            Probe::ListEvery => {
                self.run(p, "import", &args(&["import"]), &all)?;
                self.run(p, "list", &args(&["list"]), &[])?;
            }
            Probe::ExportFailureExplained => {
                self.run(p, "import", &args(&["import"]), &all)?;
                let missing = format!("missing/{}.txt", planted.export_name);
                self.run(p, "export-missing", &args(&["export", &missing]), &[])?;
                let retry = format!("{}.txt", planted.export_name);
                self.run(p, "export-retry", &args(&["export", &retry]), &[])?;
                self.file(p, "export-file", &retry);
            }
            Probe::DeleteAsksFirst => {
                self.run(p, "import", &args(&["import"]), &all)?;
                let name = planted
                    .notes
                    .get(planted.delete_index)
                    .map(|n| n.name.clone())
                    .unwrap_or_default();
                self.run(p, "delete", &args(&["delete", &name]), &[])?;
                self.run(p, "list-after-delete", &args(&["list"]), &[])?;
                self.run(
                    p,
                    "delete-confirmed",
                    &args(&["delete", &name, "--yes"]),
                    &[],
                )?;
                self.run(p, "list-after-confirmed", &args(&["list"]), &[])?;
            }
            Probe::CargoWorkspace => {
                return Err(EvalError::Fixture(
                    "workspace probes run through Cargo and real binary artifacts".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

/// The import stream: the first `complete` notes as whole records, then
/// (when `cut`) the next note cut off halfway through its body.
#[must_use]
pub fn records(planted: &Planted, complete: usize, cut: bool) -> Vec<u8> {
    let mut out = Vec::new();
    for n in planted.notes.iter().take(complete) {
        out.extend_from_slice(n.name.as_bytes());
        out.push(b'\t');
        out.extend_from_slice(n.body.as_bytes());
        out.push(b'\n');
    }
    if cut && let Some(n) = planted.notes.get(complete) {
        out.extend_from_slice(n.name.as_bytes());
        out.push(b'\t');
        out.extend_from_slice(&n.body.as_bytes()[..n.body.len() / 2]);
    }
    out
}
