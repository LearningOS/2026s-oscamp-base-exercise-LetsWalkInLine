//! # Select and Timeout
//!
//! In this exercise, you will use `tokio::select!` macro to implement race selection and timeout control.
//!
//! ## Concepts
//! - `tokio::select!` waits for multiple async operations simultaneously
//! - `tokio::time::timeout` timeout control
//! - The first completed branch is executed, others are cancelled

use std::future::Future;
use tokio::time::{sleep, Duration};

/// Async operation with timeout.
/// If `future` completes within `timeout_ms` milliseconds, returns Some(result).
/// Otherwise returns None.
///
/// Hint: Use `tokio::select!` or `tokio::time::timeout`.
pub async fn with_timeout<F, T>(future: F, timeout_ms: u64) -> Option<T>
where
    F: Future<Output = T>,
{
    // TODO: Use tokio::select! to race between future and sleep
    // Or use tokio::time::timeout
    // tokio::time::timeout(Duration::from_millis(timeout_ms), future)
    //     .await
    //     .ok()
    tokio::select! {
        _ = sleep(Duration::from_millis(timeout_ms)) => None,
            ans = future => Some(ans)
    }
}

/// Race two async tasks, return the result of whichever finishes first.
///
/// Hint: Use `tokio::select!` macro.
pub async fn race<F1, F2, T>(f1: F1, f2: F2) -> T
where
    F1: Future<Output = T>,
    F2: Future<Output = T>,
{
    // TODO: Use tokio::select! to wait for f1 and f2
    // Return the result of whichever completes first
    tokio::select! {
        ans1 = f1 => ans1,
        ans2 = f2 => ans2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timeout_success() {
        let result = with_timeout(async { 42 }, 100).await;
        assert_eq!(result, Some(42));
    }

    #[tokio::test]
    async fn test_timeout_expired() {
        let result = with_timeout(
            async {
                sleep(Duration::from_millis(200)).await;
                42
            },
            50,
        )
        .await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_race_first_wins() {
        let result = race(
            async {
                sleep(Duration::from_millis(10)).await;
                "fast"
            },
            async {
                sleep(Duration::from_millis(200)).await;
                "slow"
            },
        )
        .await;
        assert_eq!(result, "fast");
    }

    #[tokio::test]
    async fn test_race_second_wins() {
        let result = race(
            async {
                sleep(Duration::from_millis(200)).await;
                "slow"
            },
            async {
                sleep(Duration::from_millis(10)).await;
                "fast"
            },
        )
        .await;
        assert_eq!(result, "fast");
    }
}
