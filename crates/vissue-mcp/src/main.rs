//! `vissue-mcp`: a Model Context Protocol server over vissue-core.

use rmcp::ServiceExt;

mod server;
mod tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = server::VissueServer::from_env()?;
    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
