//! # Mutex Shared State
//!
//! In this exercise, you will use `Arc<Mutex<T>>` to safely share and modify data between multiple threads.
//!
//! ## Concepts
//! - `Mutex<T>` mutex protects shared data
//! - `Arc<T>` atomic reference counting enables cross-thread sharing
//! - `lock()` acquires the lock and accesses data

use std::sync::{Arc, Mutex};
use std::thread;

/// Increment a counter concurrently using `n_threads` threads.
/// Each thread increments the counter `count_per_thread` times.
/// Returns the final counter value.
///
/// Hint: Use `Arc<Mutex<usize>>` as the shared counter.
pub fn concurrent_counter(n_threads: usize, count_per_thread: usize) -> usize {
    // TODO: Create Arc<Mutex<usize>> with initial value 0
    // TODO: Spawn n_threads threads
    // TODO: In each thread, lock() and increment count_per_thread times
    // TODO: Join all threads, return final value
    let counter = Arc::new(Mutex::new(0));
    let mut handles = Vec::new();
    for _ in 0..n_threads {
        let c_counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..count_per_thread {
                *c_counter.lock().unwrap() += 1;
            }
        }));
    }
    handles.into_iter().for_each(|h| h.join().unwrap());
    // 所有线程都结束后可以直接收回所有权和锁，不必再lock
    // 1. Arc::try_unwrap(counter) 尝试剥离 Arc，成功返回 Ok(Mutex)，失败返回 Err(Arc)。
    //    这里一定成功，所以 unwrap() 拿到 Mutex。
    // 2. .into_inner() 消耗掉 Mutex（不需要 lock），得到 LockResult<usize>。
    // 3. .unwrap() 处理 LockResult，最终得到内部的 usize
    Arc::try_unwrap(counter).unwrap().into_inner().unwrap()
}

/// Add elements to a shared vector concurrently using multiple threads.
/// Each thread pushes its own id (0..n_threads) to the vector.
/// Returns the sorted vector.
///
/// Hint: Use `Arc<Mutex<Vec<usize>>>`.
pub fn concurrent_collect(n_threads: usize) -> Vec<usize> {
    // TODO: Create Arc<Mutex<Vec<usize>>>
    // TODO: Each thread pushes its own id
    // TODO: After joining all threads, sort the result and return
    let mutex = Arc::new(Mutex::new(Vec::with_capacity(n_threads)));
    let mut handles = Vec::with_capacity(n_threads);
    for id in 0..n_threads {
        let c_mutex = Arc::clone(&mutex);
        handles.push(thread::spawn(move || {
            c_mutex.lock().unwrap().push(id);
        }));
    }

    handles.into_iter().for_each(|h| h.join().unwrap());

    let mut ans = Arc::try_unwrap(mutex).unwrap().into_inner().unwrap();
    ans.sort();
    ans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_single_thread() {
        assert_eq!(concurrent_counter(1, 100), 100);
    }

    #[test]
    fn test_counter_multi_thread() {
        assert_eq!(concurrent_counter(10, 100), 1000);
    }

    #[test]
    fn test_counter_zero() {
        assert_eq!(concurrent_counter(5, 0), 0);
    }

    #[test]
    fn test_collect() {
        let result = concurrent_collect(5);
        assert_eq!(result, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_collect_single() {
        assert_eq!(concurrent_collect(1), vec![0]);
    }
}
