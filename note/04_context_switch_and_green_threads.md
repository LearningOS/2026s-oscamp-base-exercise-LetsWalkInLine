# 上下文切换与绿色线程 (Context Switch & Green Threads)

本笔记总结了操作系统/协程库底层中两个核心机制的实现：**任务的上下文切换 (Context Switch)** 与 **协作式绿色线程调度器 (Cooperative Green Thread Scheduler)**。

## 1. 上下文切换与有栈协程 (Stackful Coroutine)

上下文切换是实现多任务并发的基石，其本质是：**保存当前任务的 CPU 寄存器状态到内存中，并从内存中加载下一个任务的寄存器状态到 CPU 中。**

### 1.1 寄存器与调用约定 (Calling Convention / ABI)
在 RISC-V 64 架构中，实现上下文切换需要特别关注以下几类寄存器：
*   **参数寄存器 (`a0`-`a7`)**：函数参数由此传入。在 `switch_context(old, new)` 中，`old` 的指针位于 `a0`，`new` 的指针位于 `a1`。
*   **返回地址寄存器 (`ra`)**：记录函数执行完毕后应跳转到的地址。在 RISC-V 中，`ret` 指令本质上是 `jalr zero, 0(ra)`。
*   **栈指针寄存器 (`sp`)**：指向当前函数调用栈的栈顶。
*   **Callee-saved 寄存器 (`s0`-`s11`)**：被调用者负责保存的寄存器。在执行上下文切换时，我们必须在汇编中手动保存和恢复它们，以保证被切换走的任务在恢复时“感觉就像刚刚完成了一个普通函数调用”。（注：Caller-saved 寄存器已经在调用 `switch_context` 前由编译器自动压栈保存了）。

### 1.2 裸函数 (`#[unsafe(naked)]` / `naked_asm!`)
正常的 Rust 函数会由编译器自动生成 Prologue (分配栈、保存寄存器) 和 Epilogue (恢复寄存器、销毁栈)。
在实现 `switch_context` 时，我们正在手动替换栈指针 `sp`，如果编译器介入修改栈将导致崩溃。因此需要使用 `#[unsafe(naked)]` 宏，配合 `core::arch::naked_asm!` 手写纯汇编。

**踩坑点**：在较新的 Rust 版本中，裸函数必须指定稳定的 ABI（如 `extern "C"`），以防止 Rust 编译器为了优化而改变参数存放的寄存器（打破 `a0`、`a1` 的假设）。
```rust
#[unsafe(naked)]
pub unsafe extern "C" fn switch_context(old: &mut TaskContext, new: &TaskContext) { ... }
```

### 1.3 栈的分配与对齐
*   **方向**：大多数架构（包括 RISC-V）中栈是**向下生长**的。因此当我们分配一块内存（如 `Vec<u8>`）作为栈时，**栈顶 (Stack Top)** 是这块内存的最高地址（即 `buf.as_ptr() + size`）。
*   **对齐**：RISC-V ABI 严格要求函数入口处 `sp` 必须是 **16 字节对齐**的。通过位运算 `& !15` (即清除低 4 位) 可以强制向下对齐。

---

## 2. 绿色线程与协作式调度 (Green Threads & Cooperative Scheduling)

在拥有了上下文切换的“引擎”后，我们需要一个调度器来管理多个执行流。绿色线程是由用户态调度的极轻量级线程。

### 2.1 协作式调度 (Cooperative) vs 抢占式调度 (Preemptive)
*   **协作式**：调度器不强行打断线程。线程必须主动调用 `yield_now()` 来让出 CPU 执行权。
*   **抢占式**：依靠硬件定时器中断，强制打断超时线程（常用于现代 OS 内核）。

### 2.2 线程的生命周期与状态
一个协程的状态可以用 `ThreadState` 枚举表示：
*   **Ready**：已就绪，等待被调度运行。
*   **Running**：当前正在 CPU 上执行。
*   **Finished**：任务已执行完毕。

### 2.3 线程入口包装器 (Thread Wrapper)
当我们第一次调度到一个新生成的线程时，不能直接把用户的入口函数 `entry` 作为其上下文的 `ra` 寄存器。
*   **原因**：普通函数在执行结束时会调用 `ret`。如果直接跳转到用户函数，用户函数执行完 `ret` 时，因为没有上一层调用者，CPU 会跳往未知地址导致崩溃。
*   **解决方案**：引入一个 `thread_wrapper` 函数作为所有新线程的真正入口（初始 `ra`）。
    *   该 Wrapper 首先读取目标用户的 `entry` 函数并执行。
    *   用户函数正常返回后，继续在 Wrapper 中执行 `thread_finished()`。
    *   `thread_finished()` 将当前线程标记为 `Finished` 并主动调用调度器切换到下一个线程。

### 2.4 绕过借用检查器：操作数组内的多个可变元素
在调度循环 `schedule_next` 中，我们需要同时获取当前线程的上下文引用（保存旧状态）和下一个线程的上下文引用（加载新状态），并将它们同时传递给 `switch_context`。
直接写 `&mut self.threads[current]` 和 `&self.threads[next]` 会触发 Rust 借用检查器报错，因为编译器认为你同时进行了对同一个 `Vec` 的可变借用和不可变借用。

**解决方案**：使用裸指针 (Raw Pointers)。
```rust
let old_ptr = self.threads[out_current].ctx.as_mut_ptr(); // 借用生命周期立即结束
let new_ptr = self.threads[next].ctx.as_ptr();

unsafe {
    // 将裸指针重新解引用，绕过了对 self.threads 的整体借用限制
    switch_context(&mut *old_ptr, &*new_ptr);
}
```
这种分离“获取指针”与“使用指针”的方法，是在 Rust 中编写底层数据结构（如链表、调度器等）的经典范式。