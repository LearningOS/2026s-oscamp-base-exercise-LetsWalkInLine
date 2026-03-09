//! # Atomic Operations Basics
//!
//! In this exercise, you will use atomic types to implement a lock-free thread‑safe counter.
//!
//! ## Key Concepts
//! - `std::sync::atomic::AtomicU64`
//! - `fetch_add`, `fetch_sub`, `load`, `store` operations
//! - `compare_exchange` lock‑free primitive
//! - `Ordering` memory ordering

use std::sync::atomic::{AtomicU64, Ordering};

pub struct AtomicCounter {
    value: AtomicU64,
}

impl AtomicCounter {
    pub const fn new(init: u64) -> Self {
        Self {
            value: AtomicU64::new(init),
        }
    }

    /// Atomically increments by 1, returns the value **before** increment.
    ///
    /// Hint: use `fetch_add` with `Ordering::Relaxed`
    pub fn increment(&self) -> u64 {
        // TODO
        // 如果仅仅是加一则使用最宽松的排序模式最高效
        self.value.fetch_add(1, Ordering::Relaxed)
    }

    /// Atomically decrements by 1, returns the value **before** decrement.
    pub fn decrement(&self) -> u64 {
        // TODO
        self.value.fetch_sub(1, Ordering::Relaxed)
    }

    /// Gets the current value.
    pub fn get(&self) -> u64 {
        // TODO
        // 对于只是读取数据，Relaxed合适吗？
        self.value.load(Ordering::Relaxed)
    }

    /// Atomic CAS (Compare-And-Swap) operation.
    /// If current value equals `expected`, set to `new_val` and return Ok(expected).
    /// Otherwise return Err(actual current value).
    ///
    /// Hint: use `compare_exchange` with success ordering `Ordering::AcqRel` and failure ordering `Ordering::Acquire`
    pub fn compare_and_swap(&self, expected: u64, new_val: u64) -> Result<u64, u64> {
        // TODO
        self.value
            .compare_exchange(expected, new_val, Ordering::AcqRel, Ordering::Acquire)
    }

    /// Multiply the value atomically using a CAS loop.
    /// Returns the value **before** multiplication.
    ///
    /// Hint: read current value in loop, compute new value, try CAS to update, retry on failure.
    pub fn fetch_multiply(&self, multiplier: u64) -> u64 {
        // TODO: CAS loop
        let mut current = self.value.load(Ordering::Relaxed);
        loop {
            let new = current * multiplier;
            match self.compare_and_swap(current, new) {
                Ok(current) => return current,
                Err(actual) => current = actual, // CAS failed, retry
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_basic_ops() {
        let c = AtomicCounter::new(0);
        assert_eq!(c.increment(), 0);
        assert_eq!(c.increment(), 1);
        assert_eq!(c.get(), 2);
        assert_eq!(c.decrement(), 2);
        assert_eq!(c.get(), 1);
    }

    #[test]
    fn test_cas_success() {
        let c = AtomicCounter::new(10);
        assert_eq!(c.compare_and_swap(10, 20), Ok(10));
        assert_eq!(c.get(), 20);
    }

    #[test]
    fn test_cas_failure() {
        let c = AtomicCounter::new(10);
        assert_eq!(c.compare_and_swap(5, 20), Err(10));
        assert_eq!(c.get(), 10);
    }

    #[test]
    fn test_fetch_multiply() {
        let c = AtomicCounter::new(3);
        let old = c.fetch_multiply(4);
        assert_eq!(old, 3);
        assert_eq!(c.get(), 12);
    }

    #[test]
    fn test_concurrent_increment() {
        let counter = Arc::new(AtomicCounter::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let c = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    c.increment();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(counter.get(), 10000);
    }
}
