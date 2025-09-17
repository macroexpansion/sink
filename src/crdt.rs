use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{anyhow, Result};
use jiff::Timestamp;

pub type MessageID = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Add(String),       // add content
    Remove(MessageID), // remove by message ID
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

// RGA-inspired CRDT implementation
pub struct CRDT {
    replica_id: String,
    ops: Arc<Mutex<BTreeSet<Message>>>,
    content: Arc<Mutex<BTreeSet<Text>>>,
}

impl CRDT {
    pub fn new() -> Self {
        Self {
            replica_id: nanoid::nanoid!(),
            ops: Arc::new(Mutex::new(BTreeSet::new())),
            content: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }

    fn update(
        locked_ops: &mut MutexGuard<'_, BTreeSet<Message>>,
        locked_content: &mut MutexGuard<'_, BTreeSet<Text>>,
        message: Message,
    ) -> Result<()> {
        match &message.operation {
            Operation::Add(_) => {
                locked_content.insert(Text::from(&message));
            }
            Operation::Remove(id) => {
                if let Some(item) = locked_content.iter().find(|p| p.id == *id) {
                    *item.removed.borrow_mut() = true;
                }
            }
        }
        locked_ops.insert(message);

        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Message>> {
        let locked_ops = self.ops.lock().map_err(|_| anyhow!("lock error"))?;
        Ok(locked_ops.iter().cloned().collect())
    }

    pub fn merge(&self, mut ops: Vec<Message>) -> Result<()> {
        ops.sort_by(|x, y| x.timestamp.cmp(&y.timestamp));

        let mut locked_ops = self.ops.lock().map_err(|_| anyhow!("lock error"))?;
        let mut locked_content = self.content.lock().map_err(|_| anyhow!("lock error"))?;

        for op in ops {
            Self::update(&mut locked_ops, &mut locked_content, op)?;
        }

        Ok(())
    }

    pub fn text(&self) -> Result<String> {
        let mut text = String::new();
        let locked_ops = self.content.lock().map_err(|_| anyhow!("lock error"))?;
        for op in locked_ops.iter() {
            if !*op.removed.borrow() {
                let element = format!(
                    "\n# id: {id}, timestamp: {timestamp}\n{data}",
                    id = op.id,
                    timestamp = op.timestamp,
                    data = op.data
                );
                text.push_str(&element);
            }
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use jiff::Timestamp;
    use rand::rng;
    use rand::seq::SliceRandom;

    use super::*;

    #[test]
    fn test_crdt() {
        let crdt = CRDT::new();

        let mut ops = vec![
            Message {
                id: String::from("1"),
                operation: Operation::Add(String::from("1")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:01.693605086Z").unwrap(),
            },
            Message {
                id: String::from("2"),
                operation: Operation::Add(String::from("2")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:02.693605086Z").unwrap(),
            },
            Message {
                id: String::from("3"),
                operation: Operation::Add(String::from("3")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:03.693605086Z").unwrap(),
            },
            Message {
                id: String::from("4"),
                operation: Operation::Add(String::from("4")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:04.693605086Z").unwrap(),
            },
            Message {
                id: String::from("5"),
                operation: Operation::Add(String::from("5")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:05.693605086Z").unwrap(),
            },
            Message {
                id: String::from("6"),
                operation: Operation::Add(String::from("6")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:06.693605086Z").unwrap(),
            },
            Message {
                id: String::from("7"),
                operation: Operation::Remove(MessageID::from("5")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:07.693605086Z").unwrap(),
            },
        ];

        let mut rng = rng();
        ops.shuffle(&mut rng);

        crdt.merge(ops).unwrap();

        let text = crdt.text().unwrap();

        assert_eq!(
            text,
            String::from(
                r###"
# id: 1, timestamp: 2025-08-11T15:48:01.693605086Z
1
# id: 2, timestamp: 2025-08-11T15:48:02.693605086Z
2
# id: 3, timestamp: 2025-08-11T15:48:03.693605086Z
3
# id: 4, timestamp: 2025-08-11T15:48:04.693605086Z
4
# id: 6, timestamp: 2025-08-11T15:48:06.693605086Z
6"###
            )
        );
    }
}
