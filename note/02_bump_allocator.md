# 📝 Bump Allocator (no_std) 学习笔记

在实现最简单的基于 `no_std` 的 Bump Allocator（撞针分配器）时，涉及到了许多底层的系统编程和并发控制概念。这篇笔记记录了核心知识点以及在编写 CAS 循环时踩过的坑。

---

## 1. 核心概念解析

### 1.1 内存对齐 (Memory Alignment)
现代 CPU 读取内存时，为了性能和硬件限制，通常要求数据的起始地址是特定数值（如 2, 4, 8, 16 等 2的幂次）的整数倍。
* **`Layout`**：Rust 标准库传递给 `alloc` 的参数，包含 `size()`（需要分配的字节数）和 `align()`（要求的对齐字节数）。
* **对齐位运算魔法**：
  公式：`align_up(addr, align) = (addr + align - 1) & !(align - 1)`
  **原理解析**：因为 `align` 是 2 的幂（例如 8，二进制 `1000`），`align - 1` 就是 `0111`。取反 `!(align - 1)` 就是 `...11111000`。
  将 `addr + align - 1` 之后再进行按位与（`&`）操作，会强制把低位清零，从而完美实现向**上**取整到最接近的对齐地址。

### 1.2 Bump Allocator 工作原理
Bump Allocator 是最简单的分配器，也叫线性分配器。
* 内部维护一个 `next` 指针，初始指向 `heap_start`。
* 每次分配时，先将 `next` 向上对齐，然后跨过 `size` 大小的内存，将这块内存的起始地址返回。
* **特点**：只分配，不释放（`dealloc` 为空实现）。只能通过 `reset` 方法将指针一键拨回起点，回收所有内存。

---

## 2. 并发基石：Atomics 与 CAS 循环

`GlobalAlloc` 的 `alloc` 方法签名接收的是不可变引用 `&self`，但全局分配器随时可能被多线程并发调用。为了在不加锁的情况下安全地修改 `next` 状态，必须使用**原子操作 (Atomics)**。

### 2.1 CAS 操作：`compare_exchange` vs `weak`
CAS（Compare-And-Swap，比较并交换）是无锁编程的灵魂。它的核心逻辑是不可分割的原子指令：“检查当前内存值是否为 A；如果是，则替换为 B并返回成功；如果不是，则不替换，并返回当前真实值”。

* **`compare_exchange` (强版本)**：严格保证精确性。只要期望值匹配，就必定成功。底层在某些架构上指令较重。
* **`compare_exchange_weak` (弱版本)**：允许“伪失败”（Spurious Failure）。哪怕期望值匹配，也可能因为硬件微小干扰返回失败。**底层指令更轻量，性能更好。**
* **最佳实践**：如果只尝试一次，用强版本；如果在 `loop` 死循环中不断重试（如本题），**使用弱版本（weak）**，因为失败了反正会进入下一次循环重试。

### 2.2 `Ordering` (内存顺序)
编译器和 CPU 会重排指令以优化性能。多线程下，这可能导致致命的可见性错误。`Ordering` 用来限制这种重排：
* `Relaxed`：最宽松，只保证当前变量原子性，不保证其他操作的先后顺序。
* `Acquire / Release`：成对出现，保证特定代码块前后的读写顺序，常用于锁或消息传递。
* **`SeqCst` (Sequential Consistency)**：最严格。不仅包含前后屏障，还建立全局的强一致性顺序，所有线程看到的顺序一致。**在不确定时，无脑使用 `SeqCst` 是保证内存安全的最稳妥选择。**

---

## 3. 踩坑实录：CAS 死循环之谜

在初次实现 CAS 循环时，我遇到了评测死循环的问题。

### 🚨 错误代码示例

```rust
let mut next = self.next.load(Ordering::SeqCst);
loop {
    // 错误点 1：直接修改了用于 CAS 期望值的变量
    next = (next + layout.align() - 1) & !(layout.align() - 1);
    
    if next + layout.size() > self.heap_end { return null_mut(); }
    
    // 错误点 2：CAS 的期望值 (next) 变成了计算后的对齐值，而非内存中的真实原始值
    let result = self.next.compare_exchange(
        next, 
        next + layout.size(),
        Ordering::SeqCst, Ordering::SeqCst,
    );
    match result {
        Ok(_) => break,
        Err(actual) => next = actual, // 真实值赋给 next，但下一次循环又被强制修改了
    }
}
```

### 🔍 错误推演（为什么会死循环？）
假设 `self.next` 内存真实值是 `1`，对齐要求是 `8`。
1. `next` 读到 `1`。
2. 循环内对齐计算：`next` 变成了 `8`。
3. CAS 操作：期望内存里是 `8`，如果是一致的就改成 `8 + size`。
4. **冲突爆发**：内存里其实根本没变，还是 `1`！CAS 发现 `1 != 8`，必定失败，并返回 `actual = 1`。
5. `next` 被重置为 `1`，进入下一次循环。
6. 再次对齐变 `8`，期望是 `8` 但内存是 `1`，再次失败…… **无限死循环**。

### ✅ 正确思路：状态分离
必须把“内存原始快照”和“计算出的目标值”严格分开！

1. 维护 `current` 变量记录读出的**原始值**。
2. 根据 `current` 算出**对齐值** `aligned_start` 和**新指针值** `new_next`。
3. CAS 比较时，`expected` 必须是 `current`，新值是 `new_next`。
4. 返回时直接将 `aligned_start` 转换为指针。

---

## 4. 总结

实现一个看似极简的 Bump Allocator，实际上打通了 **内存布局（指针与对齐）** -> **并发竞争（原子变量）** -> **无锁算法（CAS Loop 与状态分离）** -> **内存模型（Ordering）** 的完整链路。尤其是在写 CAS 循环时，一定要时刻清晰地知道：**哪个变量代表过去的快照，哪个变量代表未来的期望。**