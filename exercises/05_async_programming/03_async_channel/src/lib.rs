//! # Async Channel
//!
//! In this exercise, you will use `tokio::sync::mpsc` async channels to implement producer-consumer pattern.
//!
//! ## Concepts
//! - `tokio::sync::mpsc::channel` creates bounded async channels
//! - Async `send` and `recv`
//! - Channel closing mechanism (receiver returns None after all senders are dropped)

use tokio::sync::mpsc;

/// Async producer-consumer:
/// - Create a producer task that sends each element from items sequentially
/// - Create a consumer task that receives all elements and collects them into Vec for return
///
/// Hint: Set channel capacity to items.len().max(1)
pub async fn producer_consumer(items: Vec<String>) -> Vec<String> {
    // TODO: Create channel with mpsc::channel
    // TODO: Spawn producer task: iterate through items, send each one
    // TODO: Spawn consumer task: loop recv until channel closes, collect results
    // TODO: Wait for consumer to complete and return results
    let (tx, mut rx) = mpsc::channel(items.len().max(1));
    tokio::spawn(async move {
        for item in items {
            tx.send(item).await.unwrap();
        }
    });
    tokio::spawn(async move {
        let mut ans = Vec::new();
        while let Some(item) = rx.recv().await {
            ans.push(item);
        }
        ans
    })
    .await
    .unwrap()
}

/// Fan‑in pattern: multiple producers, one consumer.
/// Create `n_producers` producers, each sending `"producer {id}: message"`.
/// Consumer collects all messages, sorts them, and returns.
pub async fn fan_in(n_producers: usize) -> Vec<String> {
    // TODO: Create mpsc channel
    // TODO: Spawn n_producers producer tasks
    //       Each sends format!("producer {id}: message")
    // TODO: Drop the original sender (important! otherwise channel won't close)
    // TODO: Consumer loops receiving, collects and sorts
    let (tx, mut rx) = mpsc::channel(n_producers);
    for id in 0..n_producers {
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tx_clone
                .send(format!("producer {}: message", id))
                .await
                .unwrap()
        });
    }
    drop(tx);
    tokio::spawn(async move {
        let mut ans = Vec::new();
        while let Some(msg) = rx.recv().await {
            ans.push(msg);
        }
        ans.sort();
        ans
    })
    .await
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_producer_consumer() {
        let items = vec!["hello".into(), "async".into(), "world".into()];
        let result = producer_consumer(items.clone()).await;
        assert_eq!(result, items);
    }

    #[tokio::test]
    async fn test_producer_consumer_empty() {
        let result = producer_consumer(vec![]).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_fan_in() {
        let result = fan_in(3).await;
        assert_eq!(
            result,
            vec![
                "producer 0: message",
                "producer 1: message",
                "producer 2: message",
            ]
        );
    }

    #[tokio::test]
    async fn test_fan_in_single() {
        let result = fan_in(1).await;
        assert_eq!(result, vec!["producer 0: message"]);
    }
}
