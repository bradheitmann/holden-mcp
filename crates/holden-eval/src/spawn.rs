//! Process spawning with a scrubbed environment and enforced limits.
//!
//! Every process holden-eval starts goes through [`run`] (or, for git, through
//! [`scrubbed`]). Each one gets an empty environment plus an explicit
//! allow-list chosen by holden, never a variable inherited from the caller:
//! the implementer's environment never reaches a process holden starts.
//!
//! Limits are enforced, not computed: the run is killed (its whole process
//! group) when its wall-clock time or its captured output exceeds the limit,
//! and the captured bytes never exceed the capture limit.

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The limits of one process run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// Wall-clock time before the process group is killed, in milliseconds.
    pub run_ms: u64,
    /// Bytes captured from each of stdout and stderr; more output kills the run.
    pub capture_bytes: u64,
    /// `RLIMIT_CPU`, seconds.
    pub cpu_secs: u64,
    /// `RLIMIT_FSIZE`, bytes: the largest file the process may write.
    pub file_bytes: u64,
    /// `RLIMIT_NOFILE`.
    pub open_files: u64,
}

impl Limits {
    /// The limits of a product run.
    pub const PRODUCT: Self = Self {
        run_ms: 20_000,
        capture_bytes: 64 * 1024,
        cpu_secs: 20,
        file_bytes: 16 * 1024 * 1024,
        open_files: 256,
    };

    /// The limits of a product build.
    pub const BUILD: Self = Self {
        run_ms: 600_000,
        capture_bytes: 64 * 1024,
        cpu_secs: 600,
        file_bytes: 1024 * 1024 * 1024,
        open_files: 1024,
    };
}

/// Which limit ended a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitHit {
    /// The wall-clock limit.
    RunTime,
    /// The capture limit.
    CaptureBytes,
}

/// How a process ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Exit {
    /// It exited with a code.
    Code {
        /// The exit code.
        code: i32,
    },
    /// A signal ended it (not one holden sent).
    Signal {
        /// The signal number.
        signal: i32,
    },
    /// holden killed it because a limit was reached.
    Killed {
        /// The limit.
        limit: LimitHit,
    },
}

impl Exit {
    /// Whether the process exited with code 0.
    #[must_use]
    pub fn success(&self) -> bool {
        matches!(self, Self::Code { code: 0 })
    }
}

/// Captured bytes, bounded: at most the capture limit is kept, with the total
/// length seen and the SHA-256 of the kept bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundedBytes {
    /// The kept bytes, hex.
    pub hex: String,
    /// How many bytes were seen in total.
    pub len: u64,
    /// Whether bytes were dropped.
    pub truncated: bool,
    /// SHA-256 of the kept bytes, hex.
    pub sha256: String,
}

impl BoundedBytes {
    /// Keep at most `cap` bytes of `bytes`, of which `total` were seen.
    #[must_use]
    pub fn keep(bytes: &[u8], total: u64, cap: u64) -> Self {
        let n = usize::try_from(cap).unwrap_or(usize::MAX).min(bytes.len());
        let kept = &bytes[..n];
        Self {
            hex: hex::encode(kept),
            len: total.max(kept.len() as u64),
            truncated: (kept.len() as u64) < total.max(kept.len() as u64),
            sha256: hex::encode(Sha256::digest(kept)),
        }
    }

    /// The kept bytes, or `None` when the hex or its hash does not hold.
    #[must_use]
    pub fn bytes(&self) -> Option<Vec<u8>> {
        let b = hex::decode(&self.hex).ok()?;
        (hex::encode(Sha256::digest(&b)) == self.sha256).then_some(b)
    }

    /// The kept bytes as text (lossy), empty when they do not decode.
    #[must_use]
    pub fn text(&self) -> String {
        self.bytes()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default()
    }
}

/// What to run.
#[derive(Clone, Debug)]
pub struct Spawn {
    /// The program.
    pub program: PathBuf,
    /// Its arguments.
    pub args: Vec<OsString>,
    /// Its working directory.
    pub cwd: PathBuf,
    /// Its whole environment (nothing else is inherited).
    pub env: Vec<(String, String)>,
    /// Bytes written to its stdin, which is then closed.
    pub stdin: Vec<u8>,
}

/// What a run produced.
#[derive(Clone, Debug)]
pub struct Outcome {
    /// How it ended.
    pub exit: Exit,
    /// Stdout, bounded.
    pub stdout: BoundedBytes,
    /// Stderr, bounded.
    pub stderr: BoundedBytes,
    /// Wall time.
    pub elapsed_ms: u64,
}

/// A command with an empty environment plus exactly `env`.
#[must_use]
pub fn scrubbed(program: &str, env: &[(String, String)]) -> Command {
    let mut cmd = Command::new(program);
    cmd.env_clear();
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd
}

fn reader(
    mut pipe: impl Read + Send + 'static,
    cap: u64,
    over: Arc<AtomicBool>,
) -> thread::JoinHandle<(Vec<u8>, u64)> {
    thread::spawn(move || {
        let cap_usize = usize::try_from(cap).unwrap_or(usize::MAX);
        let mut kept = Vec::new();
        let mut total: u64 = 0;
        let mut buf = [0u8; 8192];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    total = total.saturating_add(n as u64);
                    let room = cap_usize.saturating_sub(kept.len());
                    kept.extend_from_slice(&buf[..n.min(room)]);
                    if total > cap {
                        // Enforced: the run is killed; nothing more is read.
                        over.store(true, Ordering::SeqCst);
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        (kept, total)
    })
}

fn kill_group(child: &Child) {
    let Ok(raw) = i32::try_from(child.id()) else {
        return;
    };
    if let Some(pid) = rustix::process::Pid::from_raw(raw) {
        // The group may already be gone; that is the goal.
        let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
    }
}

/// Run `spec` under `limits`: its own process group, an environment of
/// exactly `spec.env`, stdin fed then closed, and stdout and stderr captured
/// up to `limits.capture_bytes` each. The group is killed when the run time
/// or the capture limit is exceeded, and again after the process exits, so
/// no process it started outlives the run.
///
/// # Errors
/// The I/O error when the process cannot be started.
pub fn run(spec: &Spawn, limits: &Limits) -> io::Result<Outcome> {
    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args)
        .current_dir(&spec.cwd)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }
    let start = Instant::now();
    let mut child = cmd.spawn()?;
    let over = Arc::new(AtomicBool::new(false));
    let input = spec.stdin.clone();
    let writer = child.stdin.take().map(|mut s| {
        thread::spawn(move || {
            // A product that stops reading early is not an error here.
            let _ = s.write_all(&input);
        })
    });
    let out = child
        .stdout
        .take()
        .map(|p| reader(p, limits.capture_bytes, Arc::clone(&over)));
    let err = child
        .stderr
        .take()
        .map(|p| reader(p, limits.capture_bytes, Arc::clone(&over)));
    let deadline = Duration::from_millis(limits.run_ms);
    let mut hit = None;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if over.load(Ordering::SeqCst) {
            hit = Some(LimitHit::CaptureBytes);
        } else if start.elapsed() >= deadline {
            hit = Some(LimitHit::RunTime);
        }
        if hit.is_some() {
            kill_group(&child);
            break child.wait()?;
        }
        thread::sleep(Duration::from_millis(5));
    };
    let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    kill_group(&child);
    if let Some(w) = writer {
        let _ = w.join();
    }
    let take = |h: Option<thread::JoinHandle<(Vec<u8>, u64)>>| {
        h.and_then(|h| h.join().ok()).unwrap_or_default()
    };
    let (out_bytes, out_total) = take(out);
    let (err_bytes, err_total) = take(err);
    if hit.is_none() && (out_total > limits.capture_bytes || err_total > limits.capture_bytes) {
        hit = Some(LimitHit::CaptureBytes);
    }
    let exit = match hit {
        Some(limit) => Exit::Killed { limit },
        None => match (status.code(), status.signal()) {
            (Some(code), _) => Exit::Code { code },
            (None, Some(signal)) => Exit::Signal { signal },
            (None, None) => Exit::Code { code: -1 },
        },
    };
    Ok(Outcome {
        exit,
        stdout: BoundedBytes::keep(&out_bytes, out_total, limits.capture_bytes),
        stderr: BoundedBytes::keep(&err_bytes, err_total, limits.capture_bytes),
        elapsed_ms,
    })
}
