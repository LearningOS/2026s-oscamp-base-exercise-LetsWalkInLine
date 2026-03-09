//! # Tokio Async Tasks
//!
//! In this exercise, you will use `tokio::spawn` to create concurrent asynchronous tasks.
//!
//! ## Concepts
//! - `tokio::spawn` creates asynchronous tasks
//! - `JoinHandle` waits for task completion
//! - Concurrent execution between asynchronous tasks

use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

/// Concurrently compute the square of each number in 0..n, collect results and return in order.
///
/// Hint: Create `tokio::spawn` task for each i, collect JoinHandle, await them sequentially.
pub async fn concurrent_squares(n: usize) -> Vec<usize> {
    // TODO: Create n asynchronous tasks, each computing i * i
    // TODO: Collect all JoinHandle
    // TODO: Await each one to get result
    let mut handles: Vec<JoinHandle<usize>> = Vec::with_capacity(n);
    for i in 0..n {
        handles.push(tokio::spawn(async move { i * i }));
    }
    let mut ans = Vec::with_capacity(n);
    for h in handles {
        ans.push(h.await.unwrap());
    }
    ans
}

/// Concurrently execute multiple "time-consuming" tasks (simulated with sleep), return all results.
/// Each task sleeps `duration_ms` milliseconds and then returns its `task_id`.
///
/// Key: All tasks should execute concurrently, total duration should be close to single task duration, not sum of all tasks.
pub async fn parallel_sleep_tasks(n: usize, duration_ms: u64) -> Vec<usize> {
    // TODO: Create asynchronous task for each id in 0..n
    // TODO: Each task sleeps specified duration and returns its own id
    // TODO: Collect all results and sort
    let mut handles = Vec::with_capacity(n);
    for i in 0..n {
        handles.push(tokio::spawn(async move {
            sleep(Duration::from_millis(duration_ms)).await;
            i
        }));
    }
    let mut ans = Vec::with_capacity(n);
    for h in handles {
        ans.push(h.await.unwrap());
    }
    ans.sort();
    ans
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;

    #[tokio::test]
    async fn test_squares_basic() {
        let result = concurrent_squares(5).await;
        assert_eq!(result, vec![0, 1, 4, 9, 16]);
    }

    #[tokio::test]
    async fn test_squares_zero() {
        let result = concurrent_squares(0).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_squares_one() {
        let result = concurrent_squares(1).await;
        assert_eq!(result, vec![0]);
    }

    #[tokio::test]
    async fn test_parallel_sleep() {
        let start = Instant::now();
        let result = parallel_sleep_tasks(5, 100).await;
        let elapsed = start.elapsed();

        assert_eq!(result, vec![0, 1, 2, 3, 4]);
        // Concurrent execution, total time should be much less than 5 * 100ms
        assert!(
            elapsed.as_millis() < 400,
            "Tasks should run concurrently, took {}ms",
            elapsed.as_millis()
        );
    }
}
