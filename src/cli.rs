//! CLI argument parsing for Engram.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "engram",
    about = "A minimal git-backed task graph for AI orchestration",
    version = env!("GIT_DESCRIBE"),
    after_help = "Logs are written to: ~/.local/share/engram/logs/engram.log"
)]
pub struct Cli {
    /// Path to the engram store directory (default: current directory)
    #[arg(short = 'd', long, global = true)]
    pub dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new engram store in the current directory
    Init,

    /// Create a new task
    Create {
        /// Task title
        title: String,

        /// Priority (0=critical, 4=low)
        #[arg(short, long, default_value = "2")]
        priority: u8,

        /// Labels (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        labels: Option<Vec<String>>,

        /// Description
        #[arg(short = 'D', long)]
        description: Option<String>,
    },

    /// List tasks
    List {
        /// Filter by status (open, in_progress, blocked, closed)
        #[arg(short, long)]
        status: Option<String>,
    },

    /// Show tasks that are ready to work on
    Ready,

    /// Close a task
    Close {
        /// Task ID
        id: String,

        /// Reason for closing
        #[arg(short, long)]
        reason: Option<String>,
    },

    /// Get a task by ID
    Get {
        /// Task ID
        id: String,
    },

    /// Start working on a task (set status to in_progress)
    Start {
        /// Task ID
        id: String,
    },

    /// Add a blocking dependency
    Block {
        /// Task that is blocked
        blocked_id: String,

        /// Task that is blocking (must be completed first)
        blocker_id: String,
    },

    /// Run the daemon in foreground
    Daemon,

    /// Stop the running daemon
    DaemonStop,

    /// Check daemon status
    DaemonStatus,
}
