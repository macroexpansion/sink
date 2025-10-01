pub mod client;
pub mod crdt;
pub mod rpc;
pub mod server;

pub use client::*;
pub use crdt::*;
pub use rpc::*;
pub use server::*;

use std::cell::RefCell;
use std::cmp::Ordering;

use jiff::Timestamp;
use ring::digest::{Context, Digest, SHA1_FOR_LEGACY_USE_ONLY};
use serde::{Deserialize, Serialize};

pub type MessageID = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    Add(String),       // add content
    Remove(MessageID), // remove by message ID
}

#[derive(Debug, Clone)]
pub struct HashBlock {
    id: String,
    hash: String,
}

impl HashBlock {
    pub fn new(id: String, hash: String) -> Self {
        Self { id, hash }
    }

    pub fn compute_from(id: String, prev_hash: &[u8]) -> Self {
        let digest = Self::compute_hash(&id, prev_hash);
        let hash = hex::encode(digest);
        Self { id, hash }
    }

    pub fn compute_hash(id: &str, prev_hash: &[u8]) -> Digest {
        let mut context = Context::new(&SHA1_FOR_LEGACY_USE_ONLY);
        context.update(id.as_bytes());
        context.update(prev_hash);
        context.finish()
    }

    pub fn hash(&self) -> Vec<u8> {
        hex::decode(&self.hash).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub operation: Operation,
    pub timestamp: Timestamp,
}

impl Ord for Message {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.timestamp.cmp(&other.timestamp) {
            Ordering::Equal => self.id.cmp(&other.id),
            other => other,
        }
    }
}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Text {
    pub id: String,
    pub data: String,
    pub timestamp: Timestamp,
    pub removed: RefCell<bool>,
}

impl Ord for Text {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.timestamp.cmp(&other.timestamp) {
            Ordering::Equal => self.id.cmp(&other.id),
            other => other,
        }
    }
}

impl PartialOrd for Text {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<&Message> for Text {
    fn from(value: &Message) -> Self {
        match &value.operation {
            Operation::Add(data) => Self {
                id: value.id.clone(),
                data: data.clone(),
                timestamp: value.timestamp.clone(),
                removed: RefCell::new(false),
            },
            Operation::Remove(_) => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientRequest {
    Sync,
    Update { content: String },
}
