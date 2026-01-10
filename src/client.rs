//! Client for connecting to the engram daemon.

use crate::daemon::{DaemonConfig, is_daemon_running, start_daemon};
use crate::protocol::{Request, Response};
use crate::types::{Edge, EdgeKind, Item, Status};
use eyre::{Context, Result, bail};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Client for communicating with the engram daemon.
pub struct Client {
    root: PathBuf,
    stream: UnixStream,
}

impl Client {
    /// Connect to the daemon, optionally auto-starting it if not running.
    pub fn connect(root: &Path, auto_start: bool) -> Result<Self> {
        let config = DaemonConfig::new(root);
        let socket_path = config.socket_path();

        // Try to connect, auto-start if needed
        let stream = match UnixStream::connect(&socket_path) {
            Ok(stream) => stream,
            Err(_) if auto_start => {
                if !is_daemon_running(root) {
                    start_daemon(root).context("Failed to auto-start daemon")?;

                    // Wait for daemon to be ready
                    let mut attempts = 0;
                    loop {
                        if attempts > 20 {
                            bail!("Daemon failed to start in time");
                        }
                        std::thread::sleep(Duration::from_millis(50));
                        if let Ok(stream) = UnixStream::connect(&socket_path) {
                            break stream;
                        }
                        attempts += 1;
                    }
                } else {
                    UnixStream::connect(&socket_path).context("Failed to connect to daemon")?
                }
            }
            Err(e) => {
                bail!("Failed to connect to daemon: {}. Is it running?", e);
            }
        };

        // Set read timeout
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .context("Failed to set read timeout")?;

        Ok(Self {
            root: root.to_path_buf(),
            stream,
        })
    }

    /// Get the store root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Send a request and receive a response.
    fn request(&mut self, request: Request) -> Result<Response> {
        let request_json = serde_json::to_string(&request)?;
        writeln!(self.stream, "{}", request_json)?;
        self.stream.flush()?;

        let mut reader = BufReader::new(&self.stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;

        let response: Response = serde_json::from_str(&response_line)?;
        Ok(response)
    }

    /// Create a new item.
    pub fn create(&mut self, title: &str, priority: u8, labels: &[&str], description: Option<&str>) -> Result<Item> {
        let response = self.request(Request::Create {
            title: title.to_string(),
            priority,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            description: description.map(String::from),
        })?;

        match response {
            Response::Item { item } => Ok(item),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }

    /// Update an existing item.
    pub fn update(
        &mut self,
        id: &str,
        title: Option<&str>,
        description: Option<Option<&str>>,
        priority: Option<u8>,
        labels: Option<&[&str]>,
    ) -> Result<Item> {
        let response = self.request(Request::Update {
            id: id.to_string(),
            title: title.map(String::from),
            description: description.map(|d| d.map(String::from)),
            priority,
            labels: labels.map(|l| l.iter().map(|s| s.to_string()).collect()),
        })?;

        match response {
            Response::Item { item } => Ok(item),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }

    /// Set an item's status.
    pub fn set_status(&mut self, id: &str, status: Status) -> Result<Item> {
        let response = self.request(Request::SetStatus {
            id: id.to_string(),
            status,
        })?;

        match response {
            Response::Item { item } => Ok(item),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }

    /// Close an item.
    pub fn close(&mut self, id: &str, reason: Option<&str>) -> Result<Item> {
        let response = self.request(Request::Close {
            id: id.to_string(),
            reason: reason.map(String::from),
        })?;

        match response {
            Response::Item { item } => Ok(item),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }

    /// Add an edge between items.
    pub fn add_edge(&mut self, from_id: &str, to_id: &str, kind: EdgeKind) -> Result<Edge> {
        let response = self.request(Request::AddEdge {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            kind,
        })?;

        match response {
            Response::Edge { edge } => Ok(edge),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }

    /// Remove an edge between items.
    pub fn remove_edge(&mut self, from_id: &str, to_id: &str, kind: EdgeKind) -> Result<()> {
        let response = self.request(Request::RemoveEdge {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            kind,
        })?;

        match response {
            Response::Ok => Ok(()),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }

    /// Get an item by ID.
    pub fn get(&mut self, id: &str) -> Result<Option<Item>> {
        let response = self.request(Request::Get { id: id.to_string() })?;

        match response {
            Response::Item { item } => Ok(Some(item)),
            Response::NotFound { .. } => Ok(None),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }

    /// List items with optional status filter.
    pub fn list(&mut self, status: Option<Status>) -> Result<Vec<Item>> {
        let response = self.request(Request::List { status })?;

        match response {
            Response::Items { items } => Ok(items),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }

    /// Get ready items.
    pub fn ready(&mut self) -> Result<Vec<Item>> {
        let response = self.request(Request::Ready)?;

        match response {
            Response::Items { items } => Ok(items),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }

    /// Get blocked items.
    pub fn blocked(&mut self) -> Result<Vec<Item>> {
        let response = self.request(Request::Blocked)?;

        match response {
            Response::Items { items } => Ok(items),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }

    /// Flush pending writes to disk.
    pub fn flush(&mut self) -> Result<()> {
        let response = self.request(Request::Flush)?;

        match response {
            Response::Ok => Ok(()),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }

    /// Shutdown the daemon.
    pub fn shutdown(&mut self) -> Result<()> {
        let response = self.request(Request::Shutdown)?;

        match response {
            Response::Ok => Ok(()),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }

    /// Ping the daemon.
    pub fn ping(&mut self) -> Result<()> {
        let response = self.request(Request::Ping)?;

        match response {
            Response::Pong => Ok(()),
            Response::Error { message } => bail!("{}", message),
            _ => bail!("Unexpected response"),
        }
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require a running daemon
    // Unit tests for the client are limited without mocking
}
