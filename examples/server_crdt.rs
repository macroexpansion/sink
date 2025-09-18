use std::str::FromStr;

use anyhow::Result;
use jiff::Timestamp;
use rand::rng;
use rand::seq::SliceRandom;

use sink::{Message, Operation, ServerState, SyncServer, CRDT};

#[tokio::main]
async fn main() -> Result<()> {
    let server = SyncServer::new();
    server.start("0.0.0.0:5000").await.unwrap();

    Ok(())
}
