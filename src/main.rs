use anyhow::{Ok, Result};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let addr = "0.0.0.0:6379";
    info!("Simple-Redis-Server is listening on {}", addr);
    warn!("This is a warning message");
    Ok(())
}
