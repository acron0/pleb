use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

/// Message from hook to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMessage {
    /// The raw Claude Code event name (e.g., "UserPromptSubmit", "Stop", "PostToolUse", "PermissionRequest")
    pub event_name: String,
    /// Issue number extracted from the cwd
    pub issue_number: u64,
    /// Full JSON payload from Claude Code hook stdin
    pub payload: serde_json::Value,
}

/// Response from daemon to hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResponse {
    pub success: bool,
    pub message: Option<String>,
}

/// Server that listens for hook messages
pub struct IpcServer {
    socket_path: PathBuf,
}

impl IpcServer {
    /// Create a new IPC server (doesn't start listening yet)
    pub fn new(daemon_dir: &Path) -> Self {
        let socket_path = daemon_dir.join("pleb.sock");
        Self { socket_path }
    }

    /// Get the socket path
    #[allow(dead_code)]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Start listening on the socket and return a channel for receiving messages
    pub async fn start(&mut self) -> Result<mpsc::Receiver<HookMessage>> {
        // Remove stale socket if exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .with_context(|| format!("Failed to remove stale socket: {:?}", self.socket_path))?;
        }

        // Create parent directory if needed
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create socket directory: {:?}", parent))?;
        }

        let listener = UnixListener::bind(&self.socket_path)
            .with_context(|| format!("Failed to bind to socket: {:?}", self.socket_path))?;

        tracing::info!("IPC server listening on: {:?}", self.socket_path);

        // Start accept loop in background
        let (tx, rx) = mpsc::channel(32);
        let socket_path = self.socket_path.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, tx).await {
                                tracing::warn!("Error handling IPC connection: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Error accepting IPC connection: {}", e);
                        break;
                    }
                }
            }
            // Clean up socket when done
            let _ = std::fs::remove_file(&socket_path);
        });

        Ok(rx)
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        // Clean up socket file
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}

async fn handle_connection(mut stream: UnixStream, tx: mpsc::Sender<HookMessage>) -> Result<()> {
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    reader.read_line(&mut line).await?;

    let message: HookMessage = serde_json::from_str(line.trim())
        .context("Failed to parse hook message")?;

    tracing::debug!("Received hook message: {:?}", message);

    // Send to main loop
    if tx.send(message).await.is_err() {
        // Channel closed, daemon is shutting down
        let response = HookResponse {
            success: false,
            message: Some("Daemon is shutting down".to_string()),
        };
        let response_json = serde_json::to_string(&response)?;
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        return Ok(());
    }

    // Send success response
    let response = HookResponse {
        success: true,
        message: None,
    };
    let response_json = serde_json::to_string(&response)?;
    writer.write_all(response_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;

    Ok(())
}

/// Client for sending messages to the daemon from hooks
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    /// Create a client from a daemon directory
    pub fn new(daemon_dir: &Path) -> Self {
        Self {
            socket_path: daemon_dir.join("pleb.sock"),
        }
    }

    /// Send a hook message to the daemon
    pub async fn send(&self, message: &HookMessage) -> Result<HookResponse> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .with_context(|| format!("Failed to connect to daemon socket: {:?}", self.socket_path))?;

        let message_json = serde_json::to_string(message)?;
        stream.write_all(message_json.as_bytes()).await?;
        stream.write_all(b"\n").await?;

        let (reader, _) = stream.split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let response: HookResponse = serde_json::from_str(line.trim())
            .context("Failed to parse daemon response")?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ipc_roundtrip() {
        let dir = std::env::temp_dir().join(format!("pleb-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let mut server = IpcServer::new(&dir);
        let mut rx = server.start().await.unwrap();

        let client = IpcClient::new(&dir);
        let payload = serde_json::json!({
            "cwd": "/path/to/worktree",
            "session_id": "test-session",
            "hook_event_name": "UserPromptSubmit"
        });
        let message = HookMessage {
            event_name: "UserPromptSubmit".to_string(),
            issue_number: 42,
            payload,
        };

        // Spawn client in background
        let client_handle = tokio::spawn(async move {
            client.send(&message).await.unwrap()
        });

        // Receive on server
        let received: HookMessage = rx.recv().await.unwrap();
        assert_eq!(received.issue_number, 42);
        assert_eq!(received.event_name, "UserPromptSubmit");
        assert_eq!(received.payload["session_id"], "test-session");

        // Client should get response
        let response = client_handle.await.unwrap();
        assert!(response.success);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }
}
