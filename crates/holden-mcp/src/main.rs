//! The `holden-mcp` server binary.
//!
//! ```text
//! holden-mcp serve --config <path>           # Unix socket (and loopback TCP if configured)
//! holden-mcp serve --config <path> --stdio   # one evaluator session on stdio, if enabled
//! ```
//!
//! Exit codes: 0 clean exit, 1 refused at startup, 2 usage.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use holden_mcp::{
    Clock, Core, Log, ServerConfig, StartError, serve_socket, serve_stdio, serve_tcp,
};

const USAGE: &str = "usage: holden-mcp serve --config <path> [--stdio]";

struct Args {
    config: PathBuf,
    stdio: bool,
}

fn parse_args() -> Option<Args> {
    let mut it = std::env::args().skip(1);
    if it.next().as_deref() != Some("serve") {
        return None;
    }
    let mut config = None;
    let mut stdio = false;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--config" => config = Some(PathBuf::from(it.next()?)),
            "--stdio" => stdio = true,
            _ => return None,
        }
    }
    Some(Args {
        config: config?,
        stdio,
    })
}

async fn run(args: Args) -> Result<(), StartError> {
    let config = Arc::new(ServerConfig::load(&args.config)?);
    if args.stdio && !config.stdio_enabled() {
        return Err(holden_mcp::ConfigError::StdioNotEnabled.into());
    }
    let core = Arc::new(Core::open(&config, Clock::System, Log::stderr())?);
    if args.stdio {
        return serve_stdio(core, &config).await;
    }
    let socket = serve_socket(Arc::clone(&core), Arc::clone(&config))?;
    core.log()
        .line(format!("holden-mcp: listening on {}", socket.endpoint));
    if config.tcp().is_some() {
        let tcp = serve_tcp(Arc::clone(&core), Arc::clone(&config)).await?;
        core.log()
            .line(format!("holden-mcp: listening on tcp {}", tcp.endpoint));
    }
    socket.wait().await;
    Ok(())
}

fn main() -> ExitCode {
    let Some(args) = parse_args() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("holden-mcp: runtime failed: {e}");
            return ExitCode::from(1);
        }
    };
    match rt.block_on(run(args)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("holden-mcp: {e}");
            ExitCode::from(1)
        }
    }
}
