use anyhow::Result;
use log::LevelFilter;

use sink::SyncServer;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(LevelFilter::Info.as_str()),
    )
    .init();

    let server = SyncServer::new();
    server.start("0.0.0.0:5000").await.unwrap();

    Ok(())
}
