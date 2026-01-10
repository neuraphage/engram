//! Background daemon for concurrent access to the engram store.
//!
//! The daemon provides:
//! - Write coalescing (batch multiple writes into single JSONL append)
//! - Lock management (single writer prevents corruption)
//! - Background flush with configurable interval

use crate::protocol::{Request, Response};
use crate::store::Store;
use eyre::{Context, Result};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

/// Socket file name within the .engram directory.
const SOCKET_FILE: &str = "daemon.sock";

/// PID file name within the .engram directory.
const PID_FILE: &str = "daemon.pid";

/// Default flush interval in milliseconds.
const DEFAULT_FLUSH_INTERVAL_MS: u64 = 100;

/// Configuration for the daemon.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Root directory containing .engram
    pub root: PathBuf,

    /// Flush interval for pending writes
    pub flush_interval: Duration,
}

impl DaemonConfig {
    /// Create config with default settings.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            flush_interval: Duration::from_millis(DEFAULT_FLUSH_INTERVAL_MS),
        }
    }

    /// Get the socket path.
    pub fn socket_path(&self) -> PathBuf {
        self.root.join(".engram").join(SOCKET_FILE)
    }

    /// Get the PID file path.
    pub fn pid_path(&self) -> PathBuf {
        self.root.join(".engram").join(PID_FILE)
    }
}

/// The engram daemon.
pub struct Daemon {
    config: DaemonConfig,
    store: Store,
    shutdown: Arc<AtomicBool>,
}

impl Daemon {
    /// Create a new daemon instance.
    pub fn new(config: DaemonConfig) -> Result<Self> {
        let store = Store::open(&config.root).context("Failed to open store")?;

        Ok(Self {
            config,
            store,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Get a shutdown handle that can be used to signal shutdown.
    pub fn shutdown_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    /// Run the daemon (blocking).
    pub async fn run(&mut self) -> Result<()> {
        // Clean up any stale socket
        let socket_path = self.config.socket_path();
        if socket_path.exists() {
            fs::remove_file(&socket_path).ok();
        }

        // Write PID file
        let pid_path = self.config.pid_path();
        fs::write(&pid_path, std::process::id().to_string()).context("Failed to write PID file")?;

        // Create Unix socket listener
        let listener = UnixListener::bind(&socket_path).context("Failed to bind to Unix socket")?;
        listener
            .set_nonblocking(true)
            .context("Failed to set socket to non-blocking")?;

        log::info!("Daemon listening on {:?}", socket_path);

        // Create channel for client requests
        let (tx, mut rx) = mpsc::channel::<(Request, mpsc::Sender<Response>)>(100);

        // Spawn connection acceptor task
        let shutdown_flag = Arc::clone(&self.shutdown);
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            Self::accept_connections(listener, tx_clone, shutdown_flag).await;
        });

        // Main event loop
        let mut flush_interval = interval(self.config.flush_interval);

        loop {
            tokio::select! {
                // Handle incoming request
                Some((request, response_tx)) = rx.recv() => {
                    let response = self.handle_request(request);
                    let _ = response_tx.send(response).await;
                }

                // Periodic flush (for future write coalescing)
                _ = flush_interval.tick() => {
                    // Currently writes are immediate, but this is where
                    // we would flush pending writes in a batched mode
                }
            }

            // Check shutdown flag
            if self.shutdown.load(Ordering::Relaxed) {
                log::info!("Daemon shutting down");
                break;
            }
        }

        // Cleanup
        fs::remove_file(&socket_path).ok();
        fs::remove_file(&pid_path).ok();

        Ok(())
    }

    /// Accept connections in a background task.
    async fn accept_connections(
        listener: UnixListener,
        tx: mpsc::Sender<(Request, mpsc::Sender<Response>)>,
        shutdown: Arc<AtomicBool>,
    ) {
        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            // Try to accept connection with a small delay to allow checking shutdown
            match listener.accept() {
                Ok((stream, _)) => {
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, tx_clone).await {
                            log::warn!("Connection error: {}", e);
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No pending connections, sleep briefly
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(e) => {
                    log::error!("Accept error: {}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Handle a single client connection.
    async fn handle_connection(stream: UnixStream, tx: mpsc::Sender<(Request, mpsc::Sender<Response>)>) -> Result<()> {
        stream.set_nonblocking(false)?;

        let reader = BufReader::new(stream.try_clone()?);
        let mut writer = stream;

        for line in reader.lines() {
            let line = line.context("Failed to read line")?;
            if line.is_empty() {
                continue;
            }

            let request: Request = serde_json::from_str(&line).context("Failed to parse request")?;

            // Check for shutdown request
            let is_shutdown = matches!(request, Request::Shutdown);

            // Send to main loop and wait for response
            let (resp_tx, mut resp_rx) = mpsc::channel(1);
            tx.send((request, resp_tx))
                .await
                .context("Failed to send request to daemon")?;

            if let Some(response) = resp_rx.recv().await {
                let response_json = serde_json::to_string(&response)?;
                writeln!(writer, "{}", response_json)?;
                writer.flush()?;
            }

            if is_shutdown {
                break;
            }
        }

        Ok(())
    }

    /// Handle a single request.
    fn handle_request(&mut self, request: Request) -> Response {
        match request {
            Request::Create {
                title,
                priority,
                labels,
                description,
            } => {
                let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
                match self.store.create(&title, priority, &label_refs, description.as_deref()) {
                    Ok(item) => Response::Item { item },
                    Err(e) => Response::error(e.to_string()),
                }
            }

            Request::Update {
                id,
                title,
                description,
                priority,
                labels,
            } => {
                let labels_refs: Option<Vec<&str>> = labels.as_ref().map(|l| l.iter().map(|s| s.as_str()).collect());
                // Convert Option<Option<String>> to Option<Option<&str>>
                let desc_refs: Option<Option<&str>> = match &description {
                    Some(Some(s)) => Some(Some(s.as_str())),
                    Some(None) => Some(None),
                    None => None,
                };
                match self
                    .store
                    .update(&id, title.as_deref(), desc_refs, priority, labels_refs.as_deref())
                {
                    Ok(item) => Response::Item { item },
                    Err(e) => Response::error(e.to_string()),
                }
            }

            Request::SetStatus { id, status } => match self.store.set_status(&id, status) {
                Ok(item) => Response::Item { item },
                Err(e) => Response::error(e.to_string()),
            },

            Request::Close { id, reason } => match self.store.close(&id, reason.as_deref()) {
                Ok(item) => Response::Item { item },
                Err(e) => Response::error(e.to_string()),
            },

            Request::AddEdge { from_id, to_id, kind } => match self.store.add_edge(&from_id, &to_id, kind) {
                Ok(edge) => Response::Edge { edge },
                Err(e) => Response::error(e.to_string()),
            },

            Request::RemoveEdge { from_id, to_id, kind } => match self.store.remove_edge(&from_id, &to_id, kind) {
                Ok(()) => Response::Ok,
                Err(e) => Response::error(e.to_string()),
            },

            Request::Get { id } => match self.store.get(&id) {
                Ok(Some(item)) => Response::Item { item },
                Ok(None) => Response::NotFound { id },
                Err(e) => Response::error(e.to_string()),
            },

            Request::List { status } => match self.store.list(status) {
                Ok(items) => Response::Items { items },
                Err(e) => Response::error(e.to_string()),
            },

            Request::Ready => match self.store.ready() {
                Ok(items) => Response::Items { items },
                Err(e) => Response::error(e.to_string()),
            },

            Request::Flush => {
                // Currently a no-op since writes are immediate
                // In future, this would flush any pending buffered writes
                Response::Ok
            }

            Request::Shutdown => {
                self.shutdown.store(true, Ordering::Relaxed);
                Response::Ok
            }

            Request::Ping => Response::Pong,
        }
    }
}

/// Check if a daemon is running for the given store path.
pub fn is_daemon_running(root: &Path) -> bool {
    let config = DaemonConfig::new(root);
    let socket_path = config.socket_path();
    let pid_path = config.pid_path();

    // Check if socket exists
    if !socket_path.exists() {
        return false;
    }

    // Check if PID file exists and process is alive
    if let Ok(pid_str) = fs::read_to_string(&pid_path)
        && let Ok(pid) = pid_str.trim().parse::<i32>()
    {
        // Check if process exists (signal 0 doesn't send a signal but checks existence)
        unsafe {
            if libc::kill(pid, 0) == 0 {
                return true;
            }
        }
    }

    // Stale socket, clean up
    fs::remove_file(&socket_path).ok();
    fs::remove_file(&pid_path).ok();
    false
}

/// Start the daemon as a background process.
pub fn start_daemon(root: &Path) -> Result<()> {
    use std::process::Command;

    // Get the path to the current executable
    let exe = std::env::current_exe().context("Failed to get current executable")?;

    // Start daemon in background
    Command::new(exe)
        .args(["--dir", root.to_str().unwrap_or("."), "daemon"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to spawn daemon process")?;

    // Wait a bit for daemon to start
    std::thread::sleep(Duration::from_millis(100));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_store() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();
        Store::init(&root).unwrap();
        (temp_dir, root)
    }

    #[test]
    fn test_daemon_config() {
        let config = DaemonConfig::new("/test/path");
        assert_eq!(config.socket_path(), PathBuf::from("/test/path/.engram/daemon.sock"));
        assert_eq!(config.pid_path(), PathBuf::from("/test/path/.engram/daemon.pid"));
    }

    #[test]
    fn test_daemon_creation() {
        let (_temp_dir, root) = setup_test_store();
        let config = DaemonConfig::new(&root);
        let daemon = Daemon::new(config);
        assert!(daemon.is_ok());
    }

    #[test]
    fn test_is_daemon_running_false() {
        let (_temp_dir, root) = setup_test_store();
        assert!(!is_daemon_running(&root));
    }
}
