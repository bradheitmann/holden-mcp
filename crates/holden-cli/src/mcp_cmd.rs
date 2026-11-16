//! `holden mcp`: a stdio MCP endpoint backed by the sealed Unix server.
//!
//! The relay starts no service manager. It spawns `holden-mcp serve` as a
//! child only when no server accepts the socket. Every wait is bounded and
//! every spawned child is killed and reaped when the relay exits.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use holden_embed::host::HostConfig;
use holden_embed::private_root;
use holden_embed::state::State;
use holden_mcp::Role;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::time::{sleep, timeout};

const START_DEADLINE: Duration = Duration::from_secs(10);
const IDLE_DEADLINE: Duration = Duration::from_secs(120);
const REAP_DEADLINE: Duration = Duration::from_secs(5);

/// A typed relay failure; its text never includes configuration contents.
#[derive(Debug)]
pub enum RelayError {
    /// The machine has not been initialized or its config is invalid.
    NotInitialized,
    /// The server binary could not be started.
    Spawn(std::io::Error),
    /// A running child failed before accepting its socket.
    ServerExited,
    /// The socket or stdio exchange failed.
    Io(std::io::Error),
    /// A named deadline elapsed.
    Deadline(&'static str),
    /// Child termination or reaping failed.
    Reap(std::io::Error),
}

impl fmt::Display for RelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInitialized => f.write_str("holden is not initialized; run holden init"),
            Self::Spawn(e) => write!(f, "could not start the holden MCP server: {e}"),
            Self::ServerExited => f.write_str("the holden MCP server exited before accepting"),
            Self::Io(e) => write!(f, "MCP stdio relay I/O failed: {e}"),
            Self::Deadline(phase) => write!(f, "MCP relay deadline exceeded: {phase}"),
            Self::Reap(e) => write!(f, "could not kill and reap the MCP server: {e}"),
        }
    }
}

struct ChildGuard(Option<Child>);

/// Reap a detached child without allowing its wait to outlive the relay's
/// reap deadline. The caller owns any later kill-and-wait cleanup after a
/// deadline refusal.
pub(crate) async fn reap_detached(child: &mut Child) -> Result<(), RelayError> {
    timeout(REAP_DEADLINE, child.wait())
        .await
        .map_err(|_| RelayError::Deadline("server reap"))?
        .map_err(RelayError::Reap)?;
    Ok(())
}

impl ChildGuard {
    async fn shutdown(&mut self) -> Result<(), RelayError> {
        let Some(mut child) = self.0.take() else {
            return Ok(());
        };
        if child.try_wait().map_err(RelayError::Reap)?.is_none() {
            timeout(REAP_DEADLINE, child.kill())
                .await
                .map_err(|_| RelayError::Deadline("server kill and reap"))?
                .map_err(RelayError::Reap)?;
        } else {
            timeout(REAP_DEADLINE, child.wait())
                .await
                .map_err(|_| RelayError::Deadline("server reap"))?
                .map_err(RelayError::Reap)?;
        }
        Ok(())
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let Some(mut child) = self.0.take() else {
            return;
        };
        let _ = child.start_kill();
        // Drop runs during unwalking too. Reap on a dedicated deadline-bound
        // OS thread, never by blocking a tokio runtime worker.
        let _ = std::thread::Builder::new()
            .name("holden-mcp-reap".to_owned())
            .spawn(move || {
                if let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    drop(runtime.block_on(reap_detached(&mut child)));
                }
            });
    }
}

fn server_binary() -> PathBuf {
    if let Some(path) = std::env::var_os("HOLDEN_MCP_SERVER_BIN") {
        return PathBuf::from(path);
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("holden-mcp")))
        .unwrap_or_else(|| PathBuf::from("holden-mcp"))
}

fn spawn_server(config: &Path) -> Result<ChildGuard, RelayError> {
    let child = Command::new(server_binary())
        .args(["serve", "--config"])
        .arg(config)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .map_err(RelayError::Spawn)?;
    Ok(ChildGuard(Some(child)))
}

async fn connect(
    socket: &Path,
    config: &Path,
) -> Result<(tokio::net::UnixStream, ChildGuard), RelayError> {
    if let Ok(stream) = tokio::net::UnixStream::connect(socket).await {
        return Ok((stream, ChildGuard(None)));
    }
    let mut child = spawn_server(config)?;
    let wait = async {
        loop {
            if let Some(_status) = child
                .0
                .as_mut()
                .ok_or(RelayError::ServerExited)?
                .try_wait()
                .map_err(RelayError::Spawn)?
            {
                return Err(RelayError::ServerExited);
            }
            match tokio::net::UnixStream::connect(socket).await {
                Ok(stream) => return Ok(stream),
                Err(_) => sleep(Duration::from_millis(25)).await,
            }
        }
    };
    match timeout(START_DEADLINE, wait).await {
        Ok(Ok(stream)) => Ok((stream, child)),
        Ok(Err(e)) => {
            child.shutdown().await?;
            Err(e)
        }
        Err(_) => {
            child.shutdown().await?;
            Err(RelayError::Deadline("server startup"))
        }
    }
}

fn mark_activity(started: Instant, activity: &AtomicU64) {
    let now = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    activity.store(now, Ordering::SeqCst);
}

/// Test-only acceleration for a spawned debug binary. Release builds always
/// use the production deadline, and invalid or zero values are ignored.
fn idle_deadline_limit() -> Duration {
    #[cfg(debug_assertions)]
    if let Some(milliseconds) = std::env::var_os("HOLDEN_RELAY_IDLE_DEADLINE_MS")
        .and_then(|raw| raw.into_string().ok())
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|milliseconds| *milliseconds > 0)
    {
        return Duration::from_millis(milliseconds);
    }
    IDLE_DEADLINE
}

/// The only relay session timeout: no bytes in either direction for 120s.
/// Any completed byte operation refreshes this shared deadline.
async fn idle_deadline(started: Instant, activity: &AtomicU64) -> RelayError {
    let deadline = idle_deadline_limit();
    loop {
        let last = activity.load(Ordering::SeqCst);
        let now = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let idle = now.saturating_sub(last);
        let remaining = deadline.saturating_sub(Duration::from_millis(idle));
        if remaining.is_zero() {
            return RelayError::Deadline("relay idle exchange");
        }
        tokio::time::sleep(remaining).await;
    }
}

async fn wait_io<T>(
    started: Instant,
    activity: &AtomicU64,
    operation: impl std::future::Future<Output = std::io::Result<T>>,
) -> Result<T, RelayError> {
    tokio::select! {
        biased;
        result = operation => result.map_err(RelayError::Io),
        error = idle_deadline(started, activity) => Err(error),
    }
}

async fn forward_idle<R, W>(
    mut reader: R,
    mut writer: W,
    started: Instant,
    activity: Arc<AtomicU64>,
) -> Result<(), RelayError>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buffer = [0_u8; 8192];
    loop {
        let read = wait_io(started, &activity, reader.read(&mut buffer)).await?;
        if read == 0 {
            break;
        }
        mark_activity(started, &activity);
        let mut written = 0;
        while written < read {
            let n = wait_io(started, &activity, writer.write(&buffer[written..read])).await?;
            if n == 0 {
                return Err(RelayError::Io(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "relay writer accepted no bytes",
                )));
            }
            written += n;
            mark_activity(started, &activity);
        }
    }
    Ok(())
}

/// Run one idle-bounded stdio-to-socket relay session.
pub async fn relay(root: &Path) -> Result<(), RelayError> {
    let state = State::new(root);
    let config = state.server_config(Role::Implementer);
    if !config.is_file() {
        return Err(RelayError::NotInitialized);
    }
    let parsed = holden_mcp::ServerConfig::load(&config).map_err(|_| RelayError::NotInitialized)?;
    let socket = parsed.socket_path();
    let (mut stream, mut child) = connect(&socket, &config).await?;
    let started = Instant::now();
    let activity = Arc::new(AtomicU64::new(0));
    let exchange = async {
        let mut input = tokio::io::stdin();
        let mut output = tokio::io::stdout();
        let (mut socket_read, mut socket_write) = stream.split();
        let requests = async {
            forward_idle(
                &mut input,
                &mut socket_write,
                started,
                Arc::clone(&activity),
            )
            .await?;
            wait_io(started, &activity, socket_write.shutdown()).await
        };
        let responses = async {
            forward_idle(
                &mut socket_read,
                &mut output,
                started,
                Arc::clone(&activity),
            )
            .await?;
            wait_io(started, &activity, output.flush()).await
        };
        tokio::try_join!(requests, responses).map(|_| ())
    };
    let result = exchange.await;
    // Errors from relay and reap are both typed. A relay error remains the
    // primary error after a successful reaping.
    child.shutdown().await?;
    result
}

/// Dispatch `holden init` or `holden mcp`.
pub fn dispatch(args: &[String]) -> Option<Result<ExitCode, String>> {
    let (command, rest) = args.split_first()?;
    match command.as_str() {
        "init" => Some(init(rest)),
        "mcp" => Some(run(rest)),
        _ => None,
    }
}

fn init(args: &[String]) -> Result<ExitCode, String> {
    let root = match args {
        [] => private_root().ok_or_else(|| "no private root could be resolved".to_owned())?,
        [flag, value] if flag == "--root" => PathBuf::from(value),
        _ => return Err("init takes only --root <dir>".to_owned()),
    };
    Ok(match holden_embed::distribution::init(&root) {
        Ok(report) => {
            println!("initialized: {}", report.config.display());
            if !report.keys_created.is_empty() {
                println!("keys created: {}", report.keys_created.join(", "));
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    })
}

/// Dispatch the CLI command.
pub fn run(args: &[String]) -> Result<ExitCode, String> {
    let root = match args {
        [] => match HostConfig::read(Path::new(".")).and_then(|config| config.state_dir()) {
            Ok(state) => state.dir().to_path_buf(),
            Err(_) => {
                private_root().ok_or_else(|| "no private root could be resolved".to_owned())?
            }
        },
        [flag, value] if flag == "--root" => PathBuf::from(value),
        _ => return Err("mcp takes only --root <dir>".to_owned()),
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("relay runtime failed: {e}"))?;
    let outcome = runtime.block_on(relay(&root));
    // tokio's stdin keeps a reader task alive. Explicitly shut the runtime
    // down after the EOF-led relay exchange rather than relying on Runtime's
    // blocking drop while that reader still exists.
    runtime.shutdown_timeout(REAP_DEADLINE);
    Ok(match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("refused: {e}");
            ExitCode::from(1)
        }
    })
}
