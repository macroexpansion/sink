use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{anyhow, Result};
use log::debug;
use serde::{Deserialize, Serialize};

use crate::{HashBlock, Message, Operation, Text};

/// Message for comparison
#[derive(Debug, Clone)]
pub struct CompareMessage {
    pub hash_block: HashBlock,
    pub timestamp: jiff::Timestamp,
    pub message_position: usize,
}

/// Message for comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Compare {
    pub diff: Vec<Message>,
}

/// RGA-inspired CRDT implementation
#[derive(Debug, Clone)]
pub struct CRDT {
    ops: Arc<Mutex<BTreeSet<Message>>>,
    content: Arc<Mutex<BTreeSet<Text>>>,
    hash_chain: Arc<Mutex<Vec<HashBlock>>>,
}

impl CRDT {
    pub fn new() -> Self {
        // create genesis block
        let hash_chain = vec![HashBlock::new("0".to_string(), "0".repeat(40))];
        let mut ops = BTreeSet::new();
        ops.insert(Message {
            id: String::from("0"),
            operation: Operation::Add(String::from("0")),
            timestamp: jiff::Timestamp::UNIX_EPOCH,
        });

        Self {
            ops: Arc::new(Mutex::new(ops)),
            content: Arc::new(Mutex::new(BTreeSet::new())),
            hash_chain: Arc::new(Mutex::new(hash_chain)),
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
        let mut locked_hash_chain = self.hash_chain.lock().map_err(|_| anyhow!("lock error"))?;

        for op in ops {
            Self::update(&mut locked_ops, &mut locked_content, op)?;
        }

        // skip the first one because it's the genesis block
        for op in locked_ops.iter().skip(1) {
            let prev_hash = locked_hash_chain.last().unwrap().hash();
            locked_hash_chain.push(HashBlock::compute_from(op.id.clone(), &prev_hash));
        }

        Ok(())
    }

    pub fn text(&self) -> Result<String> {
        let mut text = String::new();
        let locked_ops = self.content.lock().map_err(|_| anyhow!("lock error"))?;
        for op in locked_ops.iter().rev() {
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

    pub fn hash_chain(&self) -> Result<Vec<HashBlock>> {
        let locked_hash_chain = self.hash_chain.lock().map_err(|_| anyhow!("lock error"))?;
        Ok(locked_hash_chain.clone())
    }

    pub fn compare(&self, other: CompareMessage) -> Result<Compare> {
        let locked_ops = self.ops.lock().map_err(|_| anyhow!("lock error"))?;
        let locked_hash_chain = self.hash_chain.lock().map_err(|_| anyhow!("lock error"))?;
        let hash_chain_len = locked_hash_chain.len();

        match other.message_position.cmp(&hash_chain_len) {
            Ordering::Equal => {
                debug!("Equal");
                if locked_hash_chain.last().unwrap().hash != other.hash_block.hash {
                    return Ok(Compare {
                        diff: vec![locked_ops.last().unwrap().clone()],
                    });
                }
                return Ok(Compare { diff: Vec::new() });
            }
            Ordering::Less => {
                debug!("Less");
                let num_diff = hash_chain_len - other.message_position;
                let pos = locked_hash_chain
                    .iter()
                    .rev()
                    .skip(num_diff)
                    .position(|hash_block| hash_block.hash == other.hash_block.hash);

                let diff = match pos {
                    Some(position) => {
                        debug!("Position: {}", position);
                        let num_skip = hash_chain_len - num_diff - position;
                        debug!("Num skip: {}", num_skip);
                        let diff = locked_ops.iter().skip(num_skip).cloned().collect();
                        Compare { diff }
                    }
                    None => {
                        debug!("cannot find hash block, returning all messages");
                        let diff = locked_ops.iter().cloned().collect();
                        Compare { diff }
                    }
                };

                return Ok(diff);
            }
            Ordering::Greater => {
                return Err(anyhow!("index out of bounds"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use jiff::Timestamp;
    use rand::rng;
    use rand::seq::SliceRandom;

    use crate::*;

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
# id: 6, timestamp: 2025-08-11T15:48:06.693605086Z
6
# id: 4, timestamp: 2025-08-11T15:48:04.693605086Z
4
# id: 3, timestamp: 2025-08-11T15:48:03.693605086Z
3
# id: 2, timestamp: 2025-08-11T15:48:02.693605086Z
2
# id: 1, timestamp: 2025-08-11T15:48:01.693605086Z
1"###
            )
        );

        let hash_chain = crdt.hash_chain().unwrap();
        println!("{:#?}", hash_chain);

        let expected_hashes = vec![
            "0000000000000000000000000000000000000000",
            "1eb53bed2bf35a2c9f8e608f0ecb1485e03e01e3",
            "fdb99193d4eabcfadbf04b8d3dbda0fdd459d9d1",
            "17350e1bad4ace58a036c628e3e369b7d69732f2",
            "2811b0fce0fe29a7f9a43899b2b4077b085e0607",
            "c5a19a2a549b4209bfee4a0fe7626349ba43d8af",
            "090f7f7aa9d47cec924ffa911afabffb91cb9b0d",
            "25bbbc46c4ec1068202563dad3e78b0ab7ed55fb",
        ];
        for (block, expected_hash) in hash_chain.iter().zip(expected_hashes.iter()) {
            assert_eq!(block.hash, *expected_hash);
        }
        assert_eq!(hash_chain.len(), 8);
    }

    #[test]
    fn test_compare_less() {
        let mut rng = rng();

        let crdt_1 = CRDT::new();
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
        ops.shuffle(&mut rng);
        crdt_1.merge(ops).unwrap();

        let crdt_2 = CRDT::new();
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
        ];
        ops.shuffle(&mut rng);
        crdt_2.merge(ops).unwrap();
        let compare_message = CompareMessage {
            hash_block: crdt_2.hash_chain().unwrap()[4].clone(),
            timestamp: crdt_2.list().unwrap().last().unwrap().timestamp,
            message_position: crdt_2.list().unwrap().len(),
        };

        let compare = crdt_1.compare(compare_message).unwrap();
        for (result, expect) in compare.diff.iter().zip(vec!["5", "6", "7"]) {
            assert_eq!(result.id, expect);
        }
    }
}
