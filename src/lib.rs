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
use serde::{Deserialize, Serialize};

pub type MessageID = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    Add(String),       // add content
    Remove(MessageID), // remove by message ID
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
