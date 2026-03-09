//! # Green Thread Scheduler (riscv64)
//!
//! In this exercise, you build a simple cooperative (green) thread scheduler on top of context switching.
//! This crate is **riscv64 only**; run with the repo's normal flow (`./check.sh` / `oscamp`) or natively on riscv64.
//!
//! ## Key Concepts
//! - Cooperative vs preemptive scheduling
//! - Thread state: `Ready`, `Running`, `Finished`
//! - `yield_now()`: current thread voluntarily gives up the CPU
//! - Scheduler loop: pick next ready thread and switch to it
//!
//! ## Design
//! Each green thread has its own stack and `TaskContext`. Threads call `yield_now()` to yield.
//! The scheduler round-robins among ready threads. User entry is wrapped by `thread_wrapper`, which
//! calls the entry then marks the thread `Finished` and switches back.

#![cfg(target_arch = "riscv64")]

use core::arch::naked_asm;

/// Per-thread stack size. Slightly larger to avoid overflow under QEMU / test harness.
const STACK_SIZE: usize = 1024 * 128;

/// Task context (riscv64); layout must match `01_stack_coroutine::TaskContext` and the asm below.
#[repr(C)]
#[derive(Debug, Default, Clone)]
pub struct TaskContext {
    sp: u64,
    ra: u64,
    s0: u64,
    s1: u64,
    s2: u64,
    s3: u64,
    s4: u64,
    s5: u64,
    s6: u64,
    s7: u64,
    s8: u64,
    s9: u64,
    s10: u64,
    s11: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThreadState {
    Ready,
    Running,
    Finished,
}

struct GreenThread {
    ctx: TaskContext,
    state: ThreadState,
    _stack: Option<Vec<u8>>,
    /// User entry; taken once when the thread is first scheduled and passed to `thread_wrapper`.
    entry: Option<extern "C" fn()>,
}

/// Set by the scheduler before switching to a new thread; `thread_wrapper` reads and calls it once.
static mut CURRENT_THREAD_ENTRY: Option<extern "C" fn()> = None;

/// Wrapper run as the initial `ra` for each green thread: call the user entry (from `CURRENT_THREAD_ENTRY`), then mark Finished and switch back.
extern "C" fn thread_wrapper() {
    let entry = unsafe { core::ptr::read(&raw const CURRENT_THREAD_ENTRY) };
    if let Some(f) = entry {
        unsafe { CURRENT_THREAD_ENTRY = None };
        f();
    }
    thread_finished();
}

/// Save current callee-saved regs into `old`, load from `new`, then `ret` to `new.ra`.
/// Zero `a0`/`a1` before `ret` so we don't leak pointers into the new context.
///
/// Must be `#[unsafe(naked)]` to prevent the compiler from generating a prologue/epilogue.
#[unsafe(naked)]
unsafe extern "C" fn switch_context(_old: &mut TaskContext, _new: &TaskContext) {
    naked_asm!(
        "sd sp, 0(a0)",
        "sd ra, 8(a0)",
        "sd s0, 16(a0)",
        "sd s1, 24(a0)",
        "sd s2, 32(a0)",
        "sd s3, 40(a0)",
        "sd s4, 48(a0)",
        "sd s5, 56(a0)",
        "sd s6, 64(a0)",
        "sd s7, 72(a0)",
        "sd s8, 80(a0)",
        "sd s9, 88(a0)",
        "sd s10, 96(a0)",
        "sd s11, 104(a0)",
        "ld sp, 0(a1)",
        "ld ra, 8(a1)",
        "ld s0, 16(a1)",
        "ld s1, 24(a1)",
        "ld s2, 32(a1)",
        "ld s3, 40(a1)",
        "ld s4, 48(a1)",
        "ld s5, 56(a1)",
        "ld s6, 64(a1)",
        "ld s7, 72(a1)",
        "ld s8, 80(a1)",
        "ld s9, 88(a1)",
        "ld s10, 96(a1)",
        "ld s11, 104(a1)",
        "li a0, 0",
        "li a1, 0",
        "ret",
    );
}

pub struct Scheduler {
    threads: Vec<GreenThread>,
    current: usize,
}

impl Scheduler {
    pub fn new() -> Self {
        let main_thread = GreenThread {
            ctx: TaskContext::default(),
            state: ThreadState::Running,
            _stack: None,
            entry: None,
        };

        Self {
            threads: vec![main_thread],
            current: 0,
        }
    }

    /// Register a new green thread that will run `entry` when first scheduled.
    ///
    /// 1. Allocate a stack of `STACK_SIZE` bytes; compute `stack_top` (high address).
    /// 2. Set up the context: `ra = thread_wrapper` so the first switch jumps to the wrapper;
    ///    `sp` must be 16-byte aligned (e.g. `(stack_top - 16) & !15` to leave headroom).
    /// 3. Push a `GreenThread` with this context, state `Ready`, and `entry` stored for the wrapper to call.
    pub fn spawn(&mut self, entry: extern "C" fn()) {
        let stack = vec![0u8; STACK_SIZE];
        let stack_top = stack.as_ptr() as usize + STACK_SIZE;
        let ctx = TaskContext {
            sp: ((stack_top - 16) & !15) as u64, //感觉不一定有这个必要
            ra: thread_wrapper as u64,
            ..Default::default()
        };
        self.threads.push(GreenThread {
            ctx,
            state: ThreadState::Ready,
            _stack: Some(stack),
            entry: Some(entry),
        });
    }

    /// Run the scheduler until all threads (except the main one) are `Finished`.
    ///
    /// 1. Set the global `SCHEDULER` pointer to `self` so that `yield_now` and `thread_finished` can call back.
    /// 2. Loop: if all threads in `threads[1..]` are `Finished`, break; otherwise call `schedule_next()` (which may switch away and later return).
    /// 3. Clear `SCHEDULER` when done.
    pub fn run(&mut self) {
        unsafe { SCHEDULER = self as *mut Scheduler };
        while self.threads[1..]
            .iter()
            .any(|t| t.state != ThreadState::Finished)
        {
            self.schedule_next();
        }
        unsafe { SCHEDULER = std::ptr::null_mut() };
    }

    /// Find the next ready thread (starting from `current + 1` round-robin), mark current as `Ready` (if not `Finished`), mark next as `Running`, set `CURRENT_THREAD_ENTRY` if the next thread has an entry, then switch to it.
    fn schedule_next(&mut self) {
        for i in 0..self.threads.len() {
            let next = (i + self.current + 1) % self.threads.len();
            if self.threads[next].state == ThreadState::Ready {
                if self.threads[self.current].state != ThreadState::Finished {
                    self.threads[self.current].state = ThreadState::Ready;
                }
                let out_current = self.current;
                self.threads[next].state = ThreadState::Running;
                self.current = next;
                unsafe {
                    CURRENT_THREAD_ENTRY = self.threads[next].entry;
                }
                let old_ptr = self.threads[out_current].ctx.as_mut_ptr();
                let new_ptr = self.threads[next].ctx.as_ptr();
                unsafe {
                    switch_context(&mut *old_ptr, &*new_ptr);
                }
            }
        }
    }
}

impl TaskContext {
    fn as_mut_ptr(&mut self) -> *mut TaskContext {
        self as *mut TaskContext
    }
    fn as_ptr(&self) -> *const TaskContext {
        self as *const TaskContext
    }
}

static mut SCHEDULER: *mut Scheduler = std::ptr::null_mut();

/// Current thread voluntarily yields; the scheduler will pick the next ready thread.
pub fn yield_now() {
    unsafe {
        if !SCHEDULER.is_null() {
            (*SCHEDULER).schedule_next();
        }
    }
}

/// Mark current thread as `Finished` and switch to the next (called by `thread_wrapper` after the user entry returns).
fn thread_finished() {
    unsafe {
        if !SCHEDULER.is_null() {
            let sched = &mut *SCHEDULER;
            sched.threads[sched.current].state = ThreadState::Finished;
            sched.schedule_next();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;

    /// Tests must run serially: the scheduler uses global state (SCHEDULER, CURRENT_THREAD_ENTRY).
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    static EXEC_ORDER: AtomicU32 = AtomicU32::new(0);

    extern "C" fn task_a() {
        EXEC_ORDER.fetch_add(1, Ordering::SeqCst);
        yield_now();
        EXEC_ORDER.fetch_add(10, Ordering::SeqCst);
        yield_now();
        EXEC_ORDER.fetch_add(100, Ordering::SeqCst);
    }

    extern "C" fn task_b() {
        EXEC_ORDER.fetch_add(1, Ordering::SeqCst);
        yield_now();
        EXEC_ORDER.fetch_add(10, Ordering::SeqCst);
    }

    #[test]
    fn test_scheduler_runs_all() {
        let _guard = TEST_LOCK.lock().unwrap();
        EXEC_ORDER.store(0, Ordering::SeqCst);

        let mut sched = Scheduler::new();
        sched.spawn(task_a);
        sched.spawn(task_b);
        sched.run();

        let got = EXEC_ORDER.load(Ordering::SeqCst);
        if got != 122 {
            panic!(
                "EXEC_ORDER: expected 122, got {} (run with --nocapture to see stderr)",
                got
            );
        }
    }

    static SIMPLE_FLAG: AtomicU32 = AtomicU32::new(0);

    extern "C" fn simple_task() {
        SIMPLE_FLAG.store(42, Ordering::SeqCst);
    }

    #[test]
    fn test_single_thread() {
        let _guard = TEST_LOCK.lock().unwrap();
        SIMPLE_FLAG.store(0, Ordering::SeqCst);

        let mut sched = Scheduler::new();
        sched.spawn(simple_task);
        sched.run();

        assert_eq!(SIMPLE_FLAG.load(Ordering::SeqCst), 42);
    }
}
