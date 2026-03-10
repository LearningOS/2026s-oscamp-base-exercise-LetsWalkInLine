//! # TLB 模拟与刷新
//!
//! 本练习模拟 TLB（Translation Lookaside Buffer，地址翻译后备缓冲区），
//! 帮助你理解 TLB 的查找、插入、替换和刷新机制。
//!
//! ## 知识点
//! - TLB 是页表的硬件缓存，加速虚拟地址翻译
//! - TLB 命中/未命中（hit/miss）
//! - TLB 替换策略（本练习使用 FIFO）
//! - TLB 刷新：全部刷新、按虚拟页刷新、按 ASID 刷新
//! - ASID（Address Space Identifier）区分不同进程的地址空间
//! - MMU 工作流程：先查 TLB，miss 则走页表，再回填 TLB
//!
//! ## TLB 条目结构
//! ```text
//! ┌───────┬──────┬──────┬───────┬───────┐
//! │ valid │ asid │ vpn  │  ppn  │ flags │
//! └───────┴──────┴──────┴───────┴───────┘
//! ```

/// TLB 条目
#[derive(Clone, Debug)]
pub struct TlbEntry {
    pub valid: bool,
    pub asid: u16,
    pub vpn: u64,
    pub ppn: u64,
    pub flags: u64,
}

impl TlbEntry {
    pub fn empty() -> Self {
        Self {
            valid: false,
            asid: 0,
            vpn: 0,
            ppn: 0,
            flags: 0,
        }
    }
}

/// TLB 统计信息
#[derive(Debug, Default)]
pub struct TlbStats {
    pub hits: u64,
    pub misses: u64,
}

impl TlbStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// 模拟 TLB，固定大小，使用 FIFO 替换策略。
pub struct Tlb {
    entries: Vec<TlbEntry>,
    capacity: usize,
    /// FIFO 指针：下次替换的位置
    fifo_ptr: usize,
    pub stats: TlbStats,
}

impl Tlb {
    /// 创建一个容量为 `capacity` 的 TLB。
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: vec![TlbEntry::empty(); capacity],
            capacity,
            fifo_ptr: 0,
            stats: TlbStats::default(),
        }
    }

    /// 在 TLB 中查找匹配 `vpn` 和 `asid` 的条目。
    ///
    /// 查找规则：
    /// - 遍历所有条目
    /// - 条目必须 `valid == true`
    /// - 条目的 `vpn` 和 `asid` 都必须匹配
    /// - 命中时增加 `stats.hits`，未命中增加 `stats.misses`
    ///
    /// 返回匹配条目的 `ppn`，未命中返回 None。
    pub fn lookup(&mut self, vpn: u64, asid: u16) -> Option<u64> {
        // TODO: 遍历 self.entries，查找 valid && vpn 匹配 && asid 匹配的条目
        // 命中：self.stats.hits += 1，返回 Some(entry.ppn)
        // 未命中：self.stats.misses += 1，返回 None
        self.entries
            .iter()
            .find(|entry| entry.valid && entry.vpn == vpn && entry.asid == asid)
            .and_then(|entry| {
                self.stats.hits += 1;
                Some(entry.ppn)
            })
            .or_else(|| {
                self.stats.misses += 1;
                None
            })
    }

    /// 将一条新映射插入 TLB。
    ///
    /// 使用 FIFO 替换策略：
    /// 1. 先检查是否已存在相同 (vpn, asid) 的有效条目，如果有则更新它
    /// 2. 否则，写入 `fifo_ptr` 指向的位置
    /// 3. 将 `` 前进到下一个位置（循环：`(fifo_ptr + 1) % capacity`）
    pub fn insert(&mut self, vpn: u64, ppn: u64, asid: u16, flags: u64) {
        // TODO: 实现 TLB 插入
        // 提示：
        //   先查找已有条目：
        //   for entry in &mut self.entries {
        //       if entry.valid && entry.vpn == vpn && entry.asid == asid { 更新并返回 }
        //   }
        //   写入 fifo_ptr 位置，然后推进指针

        match self
            .entries
            .iter_mut()
            .find(|entry| entry.valid && entry.vpn == vpn && entry.asid == asid)
        {
            Some(entry) => {
                entry.ppn = ppn;
                entry.flags = flags;
            }
            None => {
                // 哪种写法更好
                let entry = self.entries.get_mut(self.fifo_ptr).unwrap();
                entry.valid = true;
                entry.vpn = vpn;
                entry.ppn = ppn;
                entry.asid = asid;
                entry.flags = flags;
                self.fifo_ptr = (self.fifo_ptr + 1) % self.capacity;
            }
        }
        // self.entries
        //     .iter_mut()
        //     .find(|entry| entry.valid && entry.vpn == vpn && entry.asid == asid)
        //     .map(|entry| {
        //         entry.ppn = ppn;
        //         entry.flags = flags;
        //     })
        //     .or_else(|| {
        //         let entry = &mut self.entries[self.fifo_ptr];
        //         entry.valid = true;
        //         entry.vpn = vpn;
        //         entry.ppn = ppn;
        //         entry.asid = asid;
        //         entry.flags = flags;
        //         self.fifo_ptr = (self.fifo_ptr + 1) % self.capacity;
        //         Some(())
        //     });
    }

    /// 刷新整个 TLB（将所有条目标记为无效）。
    ///
    /// 这对应于 RISC-V 的 `sfence.vma`（不带参数）操作。
    pub fn flush_all(&mut self) {
        // TODO: 将所有条目的 valid 设为 false
        self.entries
            .iter_mut()
            .for_each(|entry| entry.valid = false);
    }

    /// 刷新指定虚拟页的 TLB 条目。
    ///
    /// 对应 `sfence.vma vaddr`：只刷新匹配 `vpn` 的条目（任意 ASID）。
    pub fn flush_by_vpn(&mut self, vpn: u64) {
        // TODO: 将所有 vpn 匹配的条目标记为无效
        self.entries
            .iter_mut()
            .filter(|entry| entry.vpn == vpn)
            .for_each(|entry| entry.valid = false);
    }

    /// 刷新指定地址空间（ASID）的所有 TLB 条目。
    ///
    /// 对应 `sfence.vma zero, asid`：刷新该 ASID 的所有条目。
    pub fn flush_by_asid(&mut self, asid: u16) {
        // TODO: 将所有 asid 匹配的条目标记为无效
        self.entries
            .iter_mut()
            .filter(|entry| entry.asid == asid)
            .for_each(|entry| entry.valid = false);
    }

    /// 返回当前有效条目的数量。
    pub fn valid_count(&self) -> usize {
        // TODO: 统计 valid == true 的条目数
        self.entries
            .iter()
            .filter(|entry| entry.valid == true)
            .count()
    }
}

/// 页表项（简化版，用于 MMU 模拟）
pub struct PageMapping {
    pub vpn: u64,
    pub ppn: u64,
    pub flags: u64,
}

/// 模拟的 MMU：包含 TLB 和一个简单的页表。
///
/// MMU 翻译流程：
/// 1. 先查 TLB（lookup）
/// 2. TLB 命中 → 直接返回物理页号
/// 3. TLB 未命中 → 遍历页表查找（walk page table）
/// 4. 页表命中 → 将结果回填到 TLB（insert），然后返回
/// 5. 页表也未命中 → 缺页（None）
pub struct Mmu {
    pub tlb: Tlb,
    /// 简化的页表：(vpn, asid) -> PageMapping
    page_table: Vec<(u16, PageMapping)>,
    pub current_asid: u16,
}

impl Mmu {
    pub fn new(tlb_capacity: usize) -> Self {
        Self {
            tlb: Tlb::new(tlb_capacity),
            page_table: Vec::new(),
            current_asid: 0,
        }
    }

    /// 在页表中添加一条映射。
    pub fn add_mapping(&mut self, asid: u16, vpn: u64, ppn: u64, flags: u64) {
        self.page_table
            .push((asid, PageMapping { vpn, ppn, flags }));
    }

    /// 切换当前地址空间（ASID）。
    pub fn switch_asid(&mut self, new_asid: u16) {
        self.current_asid = new_asid;
    }

    /// 模拟 MMU 地址翻译。
    ///
    /// 流程：
    /// 1. 使用 `self.current_asid` 和 `vpn` 查找 TLB
    /// 2. TLB 命中 → 返回 Some(ppn)
    /// 3. TLB 未命中 → 在 `self.page_table` 中查找匹配 (current_asid, vpn) 的条目
    /// 4. 页表命中 → 回填 TLB（insert），返回 Some(ppn)
    /// 5. 页表未命中 → 返回 None（缺页）
    pub fn translate(&mut self, vpn: u64) -> Option<u64> {
        // TODO: 实现 TLB + 页表的二级查找
        if let Some(ppn) = self.tlb.lookup(vpn, self.current_asid) {
            return Some(ppn);
        }

        let (_, page_map) = self
            .page_table
            .iter()
            .find(|(p_asid, p_map)| *p_asid == self.current_asid && p_map.vpn == vpn)?;

        self.tlb
            .insert(vpn, page_map.ppn, self.current_asid, page_map.flags);
        Some(page_map.ppn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ──────── TLB 基础测试 ────────

    #[test]
    fn test_tlb_empty_lookup() {
        let mut tlb = Tlb::new(4);
        assert_eq!(tlb.lookup(0x100, 0), None);
        assert_eq!(tlb.stats.misses, 1);
        assert_eq!(tlb.stats.hits, 0);
    }

    #[test]
    fn test_tlb_insert_and_lookup() {
        let mut tlb = Tlb::new(4);
        tlb.insert(0x100, 0x200, 1, 0x7);
        assert_eq!(tlb.lookup(0x100, 1), Some(0x200));
        assert_eq!(tlb.stats.hits, 1);
    }

    #[test]
    fn test_tlb_asid_isolation() {
        let mut tlb = Tlb::new(4);
        tlb.insert(0x100, 0x200, 1, 0x7);
        tlb.insert(0x100, 0x300, 2, 0x7);

        // 同一 VPN，不同 ASID 应返回不同 PPN
        assert_eq!(tlb.lookup(0x100, 1), Some(0x200));
        assert_eq!(tlb.lookup(0x100, 2), Some(0x300));
    }

    #[test]
    fn test_tlb_miss_wrong_asid() {
        let mut tlb = Tlb::new(4);
        tlb.insert(0x100, 0x200, 1, 0x7);

        // ASID 不匹配应该 miss
        assert_eq!(tlb.lookup(0x100, 99), None);
        assert_eq!(tlb.stats.misses, 1);
    }

    #[test]
    fn test_tlb_fifo_eviction() {
        let mut tlb = Tlb::new(2); // 只有 2 个槽位
        tlb.insert(0x10, 0x20, 0, 0x7);
        tlb.insert(0x30, 0x40, 0, 0x7);
        // TLB 满了，再插入应该淘汰最先插入的
        tlb.insert(0x50, 0x60, 0, 0x7);

        // 0x10 应该被淘汰
        assert_eq!(tlb.lookup(0x10, 0), None);
        // 0x30 和 0x50 应该还在
        assert_eq!(tlb.lookup(0x30, 0), Some(0x40));
        assert_eq!(tlb.lookup(0x50, 0), Some(0x60));
    }

    #[test]
    fn test_tlb_update_existing() {
        let mut tlb = Tlb::new(4);
        tlb.insert(0x100, 0x200, 1, 0x3);
        tlb.insert(0x100, 0x999, 1, 0x7); // 更新同一条目

        assert_eq!(tlb.lookup(0x100, 1), Some(0x999));
        assert_eq!(tlb.valid_count(), 1); // 不应该多出一条
    }

    #[test]
    fn test_tlb_valid_count() {
        let mut tlb = Tlb::new(4);
        assert_eq!(tlb.valid_count(), 0);

        tlb.insert(0x1, 0x10, 0, 0x7);
        assert_eq!(tlb.valid_count(), 1);

        tlb.insert(0x2, 0x20, 0, 0x7);
        assert_eq!(tlb.valid_count(), 2);
    }

    // ──────── TLB 刷新测试 ────────

    #[test]
    fn test_flush_all() {
        let mut tlb = Tlb::new(4);
        tlb.insert(0x1, 0x10, 0, 0x7);
        tlb.insert(0x2, 0x20, 1, 0x7);
        tlb.insert(0x3, 0x30, 2, 0x7);
        assert_eq!(tlb.valid_count(), 3);

        tlb.flush_all();
        assert_eq!(tlb.valid_count(), 0);
        assert_eq!(tlb.lookup(0x1, 0), None);
    }

    #[test]
    fn test_flush_by_vpn() {
        let mut tlb = Tlb::new(4);
        tlb.insert(0x100, 0x200, 1, 0x7);
        tlb.insert(0x100, 0x300, 2, 0x7); // 同 VPN 不同 ASID
        tlb.insert(0x999, 0x400, 1, 0x7);

        tlb.flush_by_vpn(0x100);

        // VPN=0x100 的两条都应被刷掉
        assert_eq!(tlb.lookup(0x100, 1), None);
        assert_eq!(tlb.lookup(0x100, 2), None);
        // VPN=0x999 不受影响
        assert_eq!(tlb.lookup(0x999, 1), Some(0x400));
    }

    #[test]
    fn test_flush_by_asid() {
        let mut tlb = Tlb::new(4);
        tlb.insert(0x1, 0x10, 1, 0x7);
        tlb.insert(0x2, 0x20, 1, 0x7);
        tlb.insert(0x3, 0x30, 2, 0x7);

        tlb.flush_by_asid(1);

        // ASID=1 的条目被刷掉
        assert_eq!(tlb.lookup(0x1, 1), None);
        assert_eq!(tlb.lookup(0x2, 1), None);
        // ASID=2 不受影响
        assert_eq!(tlb.lookup(0x3, 2), Some(0x30));
    }

    #[test]
    fn test_flush_by_vpn_then_reinsert() {
        let mut tlb = Tlb::new(4);
        tlb.insert(0x100, 0x200, 1, 0x7);
        tlb.flush_by_vpn(0x100);
        assert_eq!(tlb.lookup(0x100, 1), None);

        // 重新插入后应该能找到
        tlb.insert(0x100, 0x500, 1, 0x7);
        assert_eq!(tlb.lookup(0x100, 1), Some(0x500));
    }

    // ──────── MMU 集成测试 ────────

    #[test]
    fn test_mmu_basic_translate() {
        let mut mmu = Mmu::new(4);
        mmu.current_asid = 1;
        mmu.add_mapping(1, 0x100, 0x200, 0x7);

        // 第一次：TLB miss，走页表
        let ppn = mmu.translate(0x100);
        assert_eq!(ppn, Some(0x200));
        assert_eq!(mmu.tlb.stats.misses, 1);
        assert_eq!(mmu.tlb.stats.hits, 0);

        // 第二次：TLB hit
        let ppn = mmu.translate(0x100);
        assert_eq!(ppn, Some(0x200));
        assert_eq!(mmu.tlb.stats.hits, 1);
    }

    #[test]
    fn test_mmu_page_fault() {
        let mut mmu = Mmu::new(4);
        mmu.current_asid = 1;
        // 没有添加任何映射
        assert_eq!(mmu.translate(0x999), None);
    }

    #[test]
    fn test_mmu_asid_switch() {
        let mut mmu = Mmu::new(4);
        mmu.add_mapping(1, 0x100, 0x200, 0x7);
        mmu.add_mapping(2, 0x100, 0x300, 0x7);

        mmu.switch_asid(1);
        assert_eq!(mmu.translate(0x100), Some(0x200));

        mmu.switch_asid(2);
        assert_eq!(mmu.translate(0x100), Some(0x300));
    }

    #[test]
    fn test_mmu_flush_on_asid_switch() {
        let mut mmu = Mmu::new(4);
        mmu.add_mapping(1, 0x100, 0x200, 0x7);
        mmu.add_mapping(2, 0x100, 0x300, 0x7);

        mmu.switch_asid(1);
        assert_eq!(mmu.translate(0x100), Some(0x200));

        // 切换 ASID 后刷新 TLB 中旧 ASID 的条目
        mmu.switch_asid(2);
        mmu.tlb.flush_by_asid(1);

        // 应该 TLB miss 然后走页表
        let old_misses = mmu.tlb.stats.misses;
        assert_eq!(mmu.translate(0x100), Some(0x300));
        assert_eq!(mmu.tlb.stats.misses, old_misses + 1);
    }

    #[test]
    fn test_mmu_hit_rate() {
        let mut mmu = Mmu::new(4);
        mmu.current_asid = 0;
        mmu.add_mapping(0, 0x1, 0x10, 0x7);

        // 第一次 miss
        mmu.translate(0x1);
        // 后续 9 次 hit
        for _ in 0..9 {
            mmu.translate(0x1);
        }

        assert_eq!(mmu.tlb.stats.hits, 9);
        assert_eq!(mmu.tlb.stats.misses, 1);
        let rate = mmu.tlb.stats.hit_rate();
        assert!(
            (rate - 0.9).abs() < 1e-9,
            "hit rate should be 0.9, got {rate}"
        );
    }

    #[test]
    fn test_mmu_thrashing() {
        // TLB 只有 2 个槽，但交替访问 3 个不同的页
        let mut mmu = Mmu::new(2);
        mmu.current_asid = 0;
        mmu.add_mapping(0, 0x1, 0x10, 0x7);
        mmu.add_mapping(0, 0x2, 0x20, 0x7);
        mmu.add_mapping(0, 0x3, 0x30, 0x7);

        // 访问 1, 2, 3, 1, 2, 3 — 由于容量只有 2，会持续 miss（thrashing）
        for vpn in [1, 2, 3, 1, 2, 3] {
            mmu.translate(vpn);
        }

        // 前两次一定 miss（冷启动），第三次也 miss（淘汰 vpn=1），
        // 第四次 vpn=1 被淘汰了所以 miss ... 全部 miss
        assert_eq!(mmu.tlb.stats.misses, 6);
        assert_eq!(mmu.tlb.stats.hits, 0);
    }
}
