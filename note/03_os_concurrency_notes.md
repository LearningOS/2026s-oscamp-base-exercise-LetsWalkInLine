# 操作系统并发编程学习笔记 (OS Concurrency)

在完成 `03_os_concurrency` 练习的过程中，我深入学习了无锁编程（Lock-free Programming）、原子操作（Atomic Operations）、内存排序（Memory Ordering）以及底层同步原语的实现。以下是核心知识点及关键讨论的总结。

## 1. 原子操作与 CAS 循环 (01_atomic_counter)

### 核心概念
*   **原子操作**：利用底层硬件指令（如 x86 的 `LOCK XADD` 等），保证“读-改-写”过程不可分割，避免多线程环境下的数据竞争。
*   **CAS (Compare-And-Swap)**：无锁编程的灵魂。对应 Rust 中的 `compare_exchange`。它在修改值前会检查当前值是否符合预期，如果是则修改并返回成功，否则返回失败及当前内存中的最新值。

### 关键讨论：CAS 循环中的活锁 (Live-lock) 陷阱
在实现 `fetch_multiply` 时，我最初的写法导致了死锁/活锁：
```rust
// ❌ 错误写法：活锁陷阱
loop {
    let current = self.value.load(Ordering::Relaxed);
    let new = current * multiplier;
    match self.compare_and_swap(current, new) {
        Ok(current) => return current,
        Err(_) => continue, // 忽略 CAS 返回的最新值，重新 load
    }
}
```
**问题分析**：在高度竞争下，单独的 `load` 指令非常慢。如果 CAS 失败后直接 `continue` 去重新 `load`，在这个时间差内，其他线程可能已经再次修改了值。当前线程会不断地“读到旧值 -> CAS 失败”，永远追不上最新的状态。

**正确写法**：
```rust
// ✅ 正确写法：紧跟最新步伐
let mut current = self.value.load(Ordering::Relaxed);
loop {
    let new = current * multiplier;
    match self.compare_and_swap(current, new) {
        Ok(current) => return current,
        Err(actual) => current = actual, // 核心：直接使用 CAS 失败顺带返回的最鲜活的值
    }
}
```
**硬件视角**：当微架构执行 RMW（原子读改写）指令失败时，硬件会顺便将当时 Cache Line 里的最新值返回。直接利用这个 `actual` 值进入下一次运算，能省去重新发起访存请求的开销，确保线程在激烈竞争中也能像齿轮一样紧密咬合推进。

## 2. 内存排序与屏障 (02_atomic_ordering)

### 核心概念
内存排序 (`Ordering`) 用于限制编译器重排和 CPU 乱序执行（OOO、Load/Store Buffer）。
*   **Release-Acquire 语义**：建立 **Happens-Before (先行发生)** 关系。
    *   `Ordering::Release` (用于 Store)：单向内存屏障，阻止所有在它之前的访存指令被重排到它之后。保证写入标志位之前，数据已经落盘。
    *   `Ordering::Acquire` (用于 Load)：单向内存屏障，阻止所有在它之后的访存指令被重排到它之前。保证读到标志位之后，再去读取数据绝对是最新的。

### 关键讨论：对 Acquire/Release 的精确理解
*   **误区**：认为 `Release` 保证是“最后一个写入”，`Acquire` 保证是“第一个读取”。
*   **真相**：它们不保证时序上的排他性（那是锁或 CAS 的工作）。它们解决的是**内存可见性（Memory Visibility）**问题。它们是强行插在流水线中的篱笆（Fence）。

### 关键讨论：为什么 OnceCell 必须用 CAS 而不能只用 SeqCst Load/Store？
*   **TOCTOU (检查到使用时间差) 问题**：如果仅仅是先 `load` 检查状态，再用 `SeqCst` 进行 `store`，在多核环境下，两个核心可能在同一周期读到 `false`，然后同时执行 `store` 写入自己的数据，导致多次初始化。
*   `SeqCst` 只保证全局执行顺序，不能把独立指令绑成原子操作。必须依靠硬件层的原子指令（如 `CMPXCHG`、`LR/SC`）配合总线/缓存一致性协议来执行独占修改。

## 3. 读写锁与状态压缩 (05_rwlock)

### 核心概念
*   **读写锁 (RwLock)**：允许多个读者并发访问，但写者必须独占。
*   **状态压缩**：为了避免多变量同步的时序漏洞，将所有状态压缩在一个 `AtomicU32` 中。
    *   Low 30 bits：读者计数。
    *   Bit 30：`WRITER_HOLDING` (写者持有中)。
    *   Bit 31：`WRITER_WAITING` (写者排队等待中)。
*   **写者优先 (Writer-Priority)**：一旦写者申请锁（置位 `WRITER_WAITING`），新的读者必须自旋阻塞，防止写者被源源不断的读者饿死 (Starvation)。

### 关键讨论：`WRITER_WAITING` 标志位如何被清除？
在写锁的申请过程中，我曾疑惑：一旦调用了 `fetch_or(WRITER_WAITING)`，这个位是不是永远为 1 了？
其实清除动作巧妙地隐藏在后续操作中：
1.  **CAS 状态跃迁时隐式清除**：当等待的写者发现读者清零，执行 `compare_exchange(current, WRITER_HOLDING)` 时，因为新的状态只包含了 `HOLDING` 没有包含 `WAITING`，这个位在 CAS 成功的瞬间被隐式清零。
2.  **Drop 时显式清除**：在 `RwLockWriteGuard` 的 `Drop` 方法中，使用位掩码 `fetch_and(!(WRITER_HOLDING | WRITER_WAITING))`，写者在释放锁时暴力抹除这两个标志位，彻底敞开大门让读者进入。