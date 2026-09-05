pub mod server;
pub mod state;
pub mod tui;

use crate::state::{BrowserSession, SharedState};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Parser, Debug)]
#[command(
    name = "qenlo-browser",
    author,
    version,
    about = "QenloDB Collection Browser: Embedded Web UI and Claude Code-style TUI"
)]
struct Cli {
    /// Path to a .qenlo directory or .qn snapshot file to open
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Run as embedded Web UI server (default port 3456)
    #[arg(long)]
    web: bool,

    /// Force Terminal User Interface (TUI) mode
    #[arg(long)]
    tui: bool,

    /// Host address for web server
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port for web server
    #[arg(short, long, default_value_t = 3456)]
    port: u16,

    /// Vector dimension override (default: 384)
    #[arg(short, long, default_value_t = 384)]
    dimension: usize,

    /// Create new collection at path if not found
    #[arg(long)]
    create: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut session = BrowserSession::new();
    session.dimension = cli.dimension;

    if let Some(path) = &cli.path {
        if cli.create && !path.exists() {
            println!(
                "Creating new collection at {} with dimension {}",
                path.display(),
                cli.dimension
            );
            let _ = session.create_collection(path, cli.dimension).await;
        } else {
            match session.open_collection(path, Some(cli.dimension)).await {
                Ok(stats) => {
                    println!(
                        "Opened collection {} (dim: {}, rows: {})",
                        path.display(),
                        stats.dimension,
                        stats.rows
                    );
                }
                Err(e) => {
                    eprintln!("Warning: Could not open {}: {}", path.display(), e);
                }
            }
        }
    }

    let shared_state: SharedState = Arc::new(RwLock::new(session));

    // Choose mode: if --web specified, run web server; else run TUI
    if cli.web {
        server::run_server(shared_state, &cli.host, cli.port).await?;
    } else {
        // Run TUI
        tui::run_tui(shared_state).await?;
    }

    Ok(())
}
