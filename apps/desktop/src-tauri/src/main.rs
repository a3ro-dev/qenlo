use std::sync::Arc;
use tokio::sync::RwLock;
use qenlo_browser::state::{BrowserSession, SharedState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting QenloDB Desktop Browser engine...");
    let session = BrowserSession::new();
    let shared_state: SharedState = Arc::new(RwLock::new(session));

    // Start background local service on port 3456
    let host = "127.0.0.1";
    let port = 3456;
    println!("QenloDB Desktop core running on http://{host}:{port}");
    
    qenlo_browser::server::run_server(shared_state, host, port).await?;
    Ok(())
}
