//! # RAII Spin Lock Guard
//!
//! In this exercise, you will implement an RAII guard for a spin lock, causing the lock to be automatically released when leaving scope.
//! This is a classic application of Rust's ownership system in systems programming.
//!
//! ## Key Points
//! - RAII (Resource Acquisition Is Initialization) pattern
//! - `Deref` / `DerefMut` traits for transparent access
//! - `Drop` trait for automatic release
//! - Why manual lock/unlock is unsafe (forgetting unlock, panic without release)

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

/// Spin lock guard: RAII handle holding the lock.
/// Automatically releases the lock when SpinGuard is dropped.
pub struct SpinGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> SpinLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire lock, returning SpinGuard.
    ///
    /// TODO: Spin-wait to acquire lock (compare_exchange), return SpinGuard on success.
    pub fn lock(&self) -> SpinGuard<'_, T> {
        // TODO: Spin-wait to acquire lock
        // TODO: Return SpinGuard { lock: self }
        loop {
            match self.locked.compare_exchange_weak(
                false,
                true,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return SpinGuard { lock: self },
                Err(_) => core::hint::spin_loop(),
            }
        }
    }
}

// TODO: Implement Deref trait for SpinGuard
// Return &T, obtained via self.lock.data.get()
impl<T> Deref for SpinGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        //这里用unwrap的写法真的合适吗？
        unsafe { self.lock.data.get().as_ref().unwrap() }
    }
}

// TODO: Implement DerefMut trait for SpinGuard
// Return &mut T
impl<T> DerefMut for SpinGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.lock.data.get().as_mut().unwrap() }
    }
}

// TODO: Implement Drop trait for SpinGuard
// Set lock.locked to false (Release ordering)
impl<T> Drop for SpinGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_guard_auto_release() {
        let lock = SpinLock::new(0u32);
        {
            let mut guard = lock.lock();
            *guard = 42;
            // guard drops here, automatically releasing lock
        }
        // Should be able to acquire lock again
        let guard = lock.lock();
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_guard_deref() {
        let lock = SpinLock::new(String::from("hello"));
        let guard = lock.lock();
        assert_eq!(guard.len(), 5);
        assert_eq!(&*guard, "hello");
    }

    #[test]
    fn test_guard_deref_mut() {
        let lock = SpinLock::new(Vec::<i32>::new());
        {
            let mut guard = lock.lock();
            guard.push(1);
            guard.push(2);
            guard.push(3);
        }
        let guard = lock.lock();
        assert_eq!(&*guard, &[1, 2, 3]);
    }

    #[test]
    fn test_concurrent_with_guard() {
        let lock = Arc::new(SpinLock::new(0u64));
        let mut handles = vec![];

        for _ in 0..10 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    let mut guard = l.lock();
                    *guard += 1;
                    // guard automatically released
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(*lock.lock(), 10000);
    }

    #[test]
    fn test_panic_safety() {
        let lock = Arc::new(SpinLock::new(0u32));
        let l = Arc::clone(&lock);

        let result = thread::spawn(move || {
            let mut guard = l.lock();
            *guard = 42;
            panic!("intentional panic");
        })
        .join();

        assert!(result.is_err());
        // Even if thread panics, guard's Drop should release lock
        // Note: this test may have different results due to panic unwind behavior
    }
}
