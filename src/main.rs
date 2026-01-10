//! Engram CLI - A minimal git-backed task graph for AI orchestration.

use clap::Parser;
use colored::*;
use engram::{Client, Daemon, DaemonConfig, EdgeKind, Status, Store, is_daemon_running};
use eyre::{Context, Result};
use log::info;
use std::fs;
use std::path::PathBuf;

mod cli;

use cli::{Cli, Command};

fn setup_logging() -> Result<()> {
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("engram")
        .join("logs");

    fs::create_dir_all(&log_dir).context("Failed to create log directory")?;

    let log_file = log_dir.join("engram.log");

    let target = Box::new(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .context("Failed to open log file")?,
    );

    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(target))
        .init();

    info!("Logging initialized, writing to: {}", log_file.display());
    Ok(())
}

fn get_store_dir(cli: &Cli) -> PathBuf {
    cli.dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn format_status(status: &Status) -> ColoredString {
    match status {
        Status::Open => "open".green(),
        Status::InProgress => "in_progress".yellow(),
        Status::Blocked => "blocked".red(),
        Status::Closed => "closed".blue(),
    }
}

fn run(cli: Cli) -> Result<()> {
    let store_dir = get_store_dir(&cli);

    match cli.command {
        Command::Init => {
            Store::init(&store_dir).context("Failed to initialize engram store")?;
            println!("{} Initialized engram store in {}", "✓".green(), store_dir.display());
        }

        Command::Create {
            title,
            priority,
            labels,
            description,
        } => {
            let mut store = Store::open(&store_dir).context("Failed to open store")?;
            let label_refs: Vec<&str> = labels
                .as_ref()
                .map(|l| l.iter().map(|s| s.as_str()).collect())
                .unwrap_or_default();

            let item = store
                .create(&title, priority, &label_refs, description.as_deref())
                .context("Failed to create item")?;

            println!("{} Created: {} {}", "✓".green(), item.id.cyan(), item.title);
        }

        Command::List { status } => {
            let store = Store::open(&store_dir).context("Failed to open store")?;
            let status_filter = status.as_ref().and_then(|s| match s.as_str() {
                "open" => Some(Status::Open),
                "in_progress" => Some(Status::InProgress),
                "blocked" => Some(Status::Blocked),
                "closed" => Some(Status::Closed),
                _ => None,
            });

            let items = store.list(status_filter).context("Failed to list items")?;

            if items.is_empty() {
                println!("{}", "No items found".dimmed());
            } else {
                for item in items {
                    let labels = if item.labels.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", item.labels.join(", "))
                    };
                    println!(
                        "{} {} P{} {} {}{}",
                        format_status(&item.status),
                        item.id.cyan(),
                        item.priority,
                        item.title,
                        labels.dimmed(),
                        item.description
                            .map(|d| format!("\n    {}", d.dimmed()))
                            .unwrap_or_default()
                    );
                }
            }
        }

        Command::Ready => {
            let store = Store::open(&store_dir).context("Failed to open store")?;
            let items = store.ready().context("Failed to get ready items")?;

            if items.is_empty() {
                println!("{}", "No ready items".dimmed());
            } else {
                println!("{} {} item(s) ready to work on:", "→".blue(), items.len());
                for item in items {
                    println!("  {} P{} {}", item.id.cyan(), item.priority, item.title);
                }
            }
        }

        Command::Blocked => {
            let store = Store::open(&store_dir).context("Failed to open store")?;
            let items = store.blocked().context("Failed to get blocked items")?;

            if items.is_empty() {
                println!("{}", "No blocked items".dimmed());
            } else {
                println!("{} {} item(s) blocked:", "⊘".red(), items.len());
                for item in items {
                    println!("  {} P{} {}", item.id.cyan(), item.priority, item.title);
                }
            }
        }

        Command::Close { id, reason } => {
            let mut store = Store::open(&store_dir).context("Failed to open store")?;
            let item = store.close(&id, reason.as_deref()).context("Failed to close item")?;

            println!("{} Closed: {} {}", "✓".green(), item.id.cyan(), item.title);
        }

        Command::Get { id } => {
            let store = Store::open(&store_dir).context("Failed to open store")?;
            let item = store.get(&id).context("Failed to get item")?;

            match item {
                Some(item) => {
                    println!("{}: {}", "ID".bold(), item.id.cyan());
                    println!("{}: {}", "Title".bold(), item.title);
                    println!("{}: {}", "Status".bold(), format_status(&item.status));
                    println!("{}: P{}", "Priority".bold(), item.priority);
                    if !item.labels.is_empty() {
                        println!("{}: {}", "Labels".bold(), item.labels.join(", "));
                    }
                    if let Some(desc) = &item.description {
                        println!("{}: {}", "Description".bold(), desc);
                    }
                    println!("{}: {}", "Created".bold(), item.created_at);
                    println!("{}: {}", "Updated".bold(), item.updated_at);
                    if let Some(closed_at) = &item.closed_at {
                        println!("{}: {}", "Closed".bold(), closed_at);
                    }
                    if let Some(reason) = &item.close_reason {
                        println!("{}: {}", "Close Reason".bold(), reason);
                    }
                }
                None => {
                    eprintln!("{} Item not found: {}", "✗".red(), id);
                    std::process::exit(1);
                }
            }
        }

        Command::Start { id } => {
            let mut store = Store::open(&store_dir).context("Failed to open store")?;
            let item = store
                .set_status(&id, Status::InProgress)
                .context("Failed to start item")?;

            println!("{} Started: {} {}", "→".blue(), item.id.cyan(), item.title);
        }

        Command::Block { blocked_id, blocker_id } => {
            let mut store = Store::open(&store_dir).context("Failed to open store")?;
            store
                .add_edge(&blocked_id, &blocker_id, EdgeKind::Blocks)
                .context("Failed to add blocking edge")?;

            println!(
                "{} {} is now blocked by {}",
                "✓".green(),
                blocked_id.cyan(),
                blocker_id.cyan()
            );
        }

        Command::Child { parent_id, child_id } => {
            let mut store = Store::open(&store_dir).context("Failed to open store")?;
            store
                .add_edge(&child_id, &parent_id, EdgeKind::ParentChild)
                .context("Failed to add parent-child relationship")?;

            println!(
                "{} {} is now a child of {}",
                "✓".green(),
                child_id.cyan(),
                parent_id.cyan()
            );
        }

        Command::Daemon => {
            println!("{} Starting daemon for {}", "→".blue(), store_dir.display());

            let config = DaemonConfig::new(&store_dir);
            let mut daemon = Daemon::new(config).context("Failed to create daemon")?;

            // Run daemon in async runtime
            let rt = tokio::runtime::Runtime::new().context("Failed to create runtime")?;
            rt.block_on(async { daemon.run().await }).context("Daemon error")?;
        }

        Command::DaemonStop => {
            if !is_daemon_running(&store_dir) {
                println!("{} Daemon is not running", "✗".red());
                std::process::exit(1);
            }

            let mut client = Client::connect(&store_dir, false).context("Failed to connect to daemon")?;
            client.shutdown().context("Failed to shutdown daemon")?;
            println!("{} Daemon stopped", "✓".green());
        }

        Command::DaemonStatus => {
            if is_daemon_running(&store_dir) {
                println!("{} Daemon is running", "✓".green());

                // Try to ping
                if let Ok(mut client) = Client::connect(&store_dir, false)
                    && client.ping().is_ok()
                {
                    println!("  {} Responding to requests", "✓".green());
                }
            } else {
                println!("{} Daemon is not running", "✗".red());
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    setup_logging().context("Failed to setup logging")?;

    let cli = Cli::parse();
    info!("Command: {:?}", std::env::args().collect::<Vec<_>>());

    if let Err(e) = run(cli) {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }

    Ok(())
}
