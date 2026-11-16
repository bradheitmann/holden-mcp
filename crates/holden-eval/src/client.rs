//! The evaluator's MCP client: a real `rmcp` client over the server's Unix
//! socket. The server reads this process's uid from the kernel and maps it
//! to the evaluator role; there is no other way to obtain parameters.

use std::path::Path;

use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::{RoleClient, ServiceExt};
use serde_json::Value;
use tokio::runtime::Runtime;

use crate::EvalError;

/// A refusal from the server (or a failed call).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolRefusal {
    /// The refusal code (`transport` when the call itself failed).
    pub code: String,
    /// The message.
    pub message: String,
}

impl std::fmt::Display for ToolRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

/// A connected client. Its calls are blocking; it owns a private runtime.
pub struct McpClient {
    rt: Runtime,
    service: Option<RunningService<RoleClient, ()>>,
}

impl std::fmt::Debug for McpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClient").finish_non_exhaustive()
    }
}

impl McpClient {
    /// Connect to the server's Unix socket.
    ///
    /// # Errors
    /// [`EvalError::Client`] when the runtime, the socket or the MCP
    /// handshake fails.
    pub fn connect(socket: &Path) -> Result<Self, EvalError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| EvalError::Client(format!("runtime: {e}")))?;
        let service = rt.block_on(async {
            let stream = tokio::net::UnixStream::connect(socket)
                .await
                .map_err(|e| EvalError::Client(format!("connect: {e}")))?;
            ().serve(stream)
                .await
                .map_err(|e| EvalError::Client(format!("handshake: {e}")))
        })?;
        Ok(Self {
            rt,
            service: Some(service),
        })
    }

    /// Call `tool` with `args` (a JSON object). Returns the structured
    /// answer, or the server's refusal.
    ///
    /// # Errors
    /// A [`ToolRefusal`].
    pub fn call(&self, tool: &str, args: Value) -> Result<Value, ToolRefusal> {
        let Some(service) = &self.service else {
            return Err(ToolRefusal {
                code: "transport".to_owned(),
                message: "the client is closed".to_owned(),
            });
        };
        let mut p = CallToolRequestParams::new(tool.to_owned());
        if let Value::Object(m) = args {
            p = p.with_arguments(m);
        }
        let r = self
            .rt
            .block_on(service.call_tool(p))
            .map_err(|e| ToolRefusal {
                code: "transport".to_owned(),
                message: e.to_string(),
            })?;
        let body = r.structured_content.clone().unwrap_or(Value::Null);
        if r.is_error.unwrap_or(false) {
            return Err(ToolRefusal {
                code: body["refusal"].as_str().unwrap_or("error").to_owned(),
                message: body["message"].as_str().unwrap_or_default().to_owned(),
            });
        }
        Ok(body)
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        if let Some(s) = self.service.take() {
            let _ = self.rt.block_on(s.cancel());
        }
    }
}
