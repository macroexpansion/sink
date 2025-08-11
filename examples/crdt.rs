use std::str::FromStr;

use anyhow::Result;
use jiff::Timestamp;
use rand::rng;
use rand::seq::SliceRandom;

use sink::{Message, Operation, CRDT};

#[tokio::main]
async fn main() -> Result<()> {
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
            operation: Operation::Remove(String::from("5")),
            timestamp: Timestamp::from_str("2025-08-11T15:48:07.693605086Z").unwrap(),
        },
    ];

    let mut rng = rng();
    ops.shuffle(&mut rng);

    crdt.merge(ops)?;

    let a = crdt.list()?;
    println!("{:#?}", a);

    let text = crdt.text()?;
    println!("{:?}", text);

    Ok(())
}
