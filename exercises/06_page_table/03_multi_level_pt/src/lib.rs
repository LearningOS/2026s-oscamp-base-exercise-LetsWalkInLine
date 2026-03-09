//! # SV39 三级页表
//!
//! 本练习模拟 RISC-V SV39 三级页表的构造和地址翻译。
//! 注意，实际上的三级页表实现并非如本练习中使用 HashMap 模拟，本练习仅作为模拟帮助学习。
//! 你需要实现页表的创建、映射和地址翻译（页表遍历）。
//!
//! ## 知识点
//! - SV39：39 位虚拟地址，三级页表
//! - VPN 拆分：VPN[2] (9bit) | VPN[1] (9bit) | VPN[0] (9bit)
//! - 页表遍历（page table walk）逐级查找
//! - 大页（2MB superpage）映射
//!
//! ## SV39 虚拟地址布局
//! ```text
//! 38        30 29       21 20       12 11        0
//! ┌──────────┬───────────┬───────────┬───────────┐
//! │ VPN[2]   │  VPN[1]   │  VPN[0]   │  offset   │
//! │  9 bits  │  9 bits   │  9 bits   │  12 bits  │
//! └──────────┴───────────┴───────────┴───────────┘
//! ```

use std::collections::HashMap;

/// 页大小 4KB
pub const PAGE_SIZE: usize = 4096;
/// 每级页表有 512 个条目 (2^9)
pub const PT_ENTRIES: usize = 512;

/// PTE 标志位
pub const PTE_V: u64 = 1 << 0;
pub const PTE_R: u64 = 1 << 1;
pub const PTE_W: u64 = 1 << 2;
pub const PTE_X: u64 = 1 << 3;

/// PPN 在 PTE 中的偏移
const PPN_SHIFT: u32 = 10;

/// 页表节点：一个包含 512 个条目的数组
#[derive(Clone)]
pub struct PageTableNode {
    pub entries: [u64; PT_ENTRIES],
}

impl PageTableNode {
    pub fn new() -> Self {
        Self {
            entries: [0; PT_ENTRIES],
        }
    }
}

impl Default for PageTableNode {
    fn default() -> Self {
        Self::new()
    }
}

/// 模拟的三级页表。
///
/// 使用 HashMap<u64, PageTableNode> 模拟物理内存中的页表页。
/// `root_ppn` 是根页表所在的物理页号。
pub struct Sv39PageTable {
    /// 物理页号 -> 页表节点
    nodes: HashMap<u64, PageTableNode>,
    /// 根页表的物理页号
    pub root_ppn: u64,
    /// 下一个可分配的物理页号（简易分配器）
    next_ppn: u64,
}

/// 翻译结果
#[derive(Debug, PartialEq)]
pub enum TranslateResult {
    Ok(u64),
    PageFault,
}

impl Sv39PageTable {
    pub fn new() -> Self {
        let mut pt = Self {
            nodes: HashMap::new(),
            root_ppn: 0x80000,
            next_ppn: 0x80001,
        };
        pt.nodes.insert(pt.root_ppn, PageTableNode::new());
        pt
    }

    /// 分配一个新的物理页并初始化为空页表节点，返回其 PPN。
    fn alloc_node(&mut self) -> u64 {
        let ppn = self.next_ppn;
        self.next_ppn += 1;
        self.nodes.insert(ppn, PageTableNode::new());
        ppn
    }

    /// 从 39 位虚拟地址中提取第 `level` 级的 VPN。
    ///
    /// - level=2: 取 bits [38:30]
    /// - level=1: 取 bits [29:21]
    /// - level=0: 取 bits [20:12]
    ///
    /// 提示：右移 (12 + level * 9) 位，然后与 0x1FF 做掩码。
    pub fn extract_vpn(va: u64, level: usize) -> usize {
        ((va >> (12 + level * 9)) & 0x1FF) as usize
    }

    /// 建立从虚拟页到物理页的映射（4KB 页）。
    ///
    /// 参数：
    /// - `va`: 虚拟地址（会自动对齐到页边界）
    /// - `pa`: 物理地址（会自动对齐到页边界）
    /// - `flags`: 标志位（如 PTE_V | PTE_R | PTE_W）
    pub fn map_page(&mut self, va: u64, pa: u64, flags: u64) {
        let root_pte = self.nodes.get(&self.root_ppn).unwrap().entries[Self::extract_vpn(va, 2)];
        let mut mid_node_ppn = (root_pte >> PPN_SHIFT) & ((1 << 44) - 1);
        if root_pte & PTE_V == 0 {
            mid_node_ppn = self.alloc_node();
            self.nodes.get_mut(&self.root_ppn).unwrap().entries[Self::extract_vpn(va, 2)] =
                (mid_node_ppn << PPN_SHIFT) | PTE_V;
        }

        let mid_pte = self.nodes.get(&mid_node_ppn).unwrap().entries[Self::extract_vpn(va, 1)];
        let mut last_node_ppn = (mid_pte >> PPN_SHIFT) & ((1 << 44) - 1);
        if mid_pte & PTE_V == 0 {
            last_node_ppn = self.alloc_node();
            self.nodes.get_mut(&mid_node_ppn).unwrap().entries[Self::extract_vpn(va, 1)] =
                (last_node_ppn << PPN_SHIFT) | PTE_V;
        }

        let last_pte = self
            .nodes
            .get_mut(&last_node_ppn)
            .unwrap()
            .entries
            .get_mut(Self::extract_vpn(va, 0))
            .unwrap();
        *last_pte = ((pa >> 12) << PPN_SHIFT) | flags;
    }

    /// 遍历三级页表，将虚拟地址翻译为物理地址。
    ///
    /// 步骤：
    /// 1. 从根页表（root_ppn）开始
    /// 2. 对每一级（2, 1, 0）：
    ///    a. 用 VPN[level] 索引当前页表节点
    ///    b. 如果 PTE 无效（!PTE_V），返回 PageFault
    ///    c. 如果 PTE 是叶节点（R|W|X 有任一置位），提取 PPN 计算物理地址
    ///    d. 否则用 PTE 中的 PPN 进入下一级页表
    /// 3. level 0 的 PTE 必须是叶节点
    pub fn translate(&self, va: u64) -> TranslateResult {
        let root_pte = self.nodes.get(&self.root_ppn).unwrap().entries[Self::extract_vpn(va, 2)];
        if root_pte & PTE_V == 0 {
            return TranslateResult::PageFault;
        }
        if root_pte & (PTE_R | PTE_W | PTE_X) != 0 {
            let ppn = (root_pte >> PPN_SHIFT) & ((1 << 44) - 1);
            return TranslateResult::Ok((ppn << 12) | (va & ((1 << 30) - 1)));
        }

        let mid_pte = self
            .nodes
            .get(&((root_pte >> PPN_SHIFT) & ((1 << 44) - 1)))
            .unwrap()
            .entries[Self::extract_vpn(va, 1)];
        if mid_pte & PTE_V == 0 {
            return TranslateResult::PageFault;
        }
        if mid_pte & (PTE_R | PTE_W | PTE_X) != 0 {
            let ppn = (mid_pte >> PPN_SHIFT) & ((1 << 44) - 1);
            return TranslateResult::Ok((ppn << 12) | (va & ((1 << 21) - 1)));
        }

        let last_pte = self
            .nodes
            .get(&((mid_pte >> PPN_SHIFT) & ((1 << 44) - 1)))
            .unwrap()
            .entries[Self::extract_vpn(va, 0)];
        if last_pte & PTE_V == 0 || (last_pte & (PTE_R | PTE_W | PTE_X) == 0) {
            return TranslateResult::PageFault;
        }

        let ppn = (last_pte >> PPN_SHIFT) & ((1 << 44) - 1);
        TranslateResult::Ok((ppn << 12) | (va & 0xFFF))
    }

    /// 建立大页映射（2MB superpage，在 level 1 设叶子 PTE）。
    ///
    /// 2MB = 512 × 4KB，对齐要求：va 和 pa 都必须 2MB 对齐。
    ///
    /// 与 map_page 类似，但只遍历到 level 1 就写入叶子 PTE。
    pub fn map_superpage(&mut self, va: u64, pa: u64, flags: u64) {
        let mega_size: u64 = (PAGE_SIZE * PT_ENTRIES) as u64; // 2MB
        assert_eq!(va % mega_size, 0, "va must be 2MB-aligned");
        assert_eq!(pa % mega_size, 0, "pa must be 2MB-aligned");

        let root_pte = self.nodes.get(&self.root_ppn).unwrap().entries[Self::extract_vpn(va, 2)];
        let mut mid_node_ppn = (root_pte >> PPN_SHIFT) & ((1 << 44) - 1);
        if root_pte & PTE_V == 0 {
            mid_node_ppn = self.alloc_node();
            self.nodes.get_mut(&self.root_ppn).unwrap().entries[Self::extract_vpn(va, 2)] =
                (mid_node_ppn << PPN_SHIFT) | PTE_V;
        }

        let last_pte = self
            .nodes
            .get_mut(&mid_node_ppn)
            .unwrap()
            .entries
            .get_mut(Self::extract_vpn(va, 1))
            .unwrap();
        *last_pte = ((pa >> 12) << PPN_SHIFT) | flags;
    }
}

impl Default for Sv39PageTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_vpn() {
        // VA = 0x0000_003F_FFFF_F000 (最大的 39 位地址的页边界)
        // VPN[2] = 0xFF (bits 38:30)
        // VPN[1] = 0x1FF (bits 29:21)
        // VPN[0] = 0x1FF (bits 20:12)
        let va: u64 = 0x7FFFFFF000;
        assert_eq!(Sv39PageTable::extract_vpn(va, 2), 0x1FF);
        assert_eq!(Sv39PageTable::extract_vpn(va, 1), 0x1FF);
        assert_eq!(Sv39PageTable::extract_vpn(va, 0), 0x1FF);
    }

    #[test]
    fn test_extract_vpn_simple() {
        // VA = 0x00000000 + page 1 = 0x1000
        // VPN[2] = 0, VPN[1] = 0, VPN[0] = 1
        let va: u64 = 0x1000;
        assert_eq!(Sv39PageTable::extract_vpn(va, 2), 0);
        assert_eq!(Sv39PageTable::extract_vpn(va, 1), 0);
        assert_eq!(Sv39PageTable::extract_vpn(va, 0), 1);
    }

    #[test]
    fn test_extract_vpn_level2() {
        // VPN[2] = 1 means bit 30 set -> VA >= 0x40000000
        let va: u64 = 0x40000000;
        assert_eq!(Sv39PageTable::extract_vpn(va, 2), 1);
        assert_eq!(Sv39PageTable::extract_vpn(va, 1), 0);
        assert_eq!(Sv39PageTable::extract_vpn(va, 0), 0);
    }

    #[test]
    fn test_map_and_translate_single() {
        let mut pt = Sv39PageTable::new();
        // 映射：VA 0x1000 -> PA 0x80001000
        pt.map_page(0x1000, 0x80001000, PTE_V | PTE_R);

        let result = pt.translate(0x1000);
        assert_eq!(result, TranslateResult::Ok(0x80001000));
    }

    #[test]
    fn test_translate_with_offset() {
        let mut pt = Sv39PageTable::new();
        pt.map_page(0x2000, 0x90000000, PTE_V | PTE_R | PTE_W);

        // 访问 VA 0x2ABC -> PA 应为 0x90000ABC
        let result = pt.translate(0x2ABC);
        assert_eq!(result, TranslateResult::Ok(0x90000ABC));
    }

    #[test]
    fn test_translate_page_fault() {
        let pt = Sv39PageTable::new();
        assert_eq!(pt.translate(0x1000), TranslateResult::PageFault);
    }

    #[test]
    fn test_multiple_mappings() {
        let mut pt = Sv39PageTable::new();
        pt.map_page(0x0000_1000, 0x8000_1000, PTE_V | PTE_R);
        pt.map_page(0x0000_2000, 0x8000_5000, PTE_V | PTE_R | PTE_W);
        pt.map_page(0x0040_0000, 0x9000_0000, PTE_V | PTE_R);

        assert_eq!(pt.translate(0x1234), TranslateResult::Ok(0x80001234));
        assert_eq!(pt.translate(0x2000), TranslateResult::Ok(0x80005000));
        assert_eq!(pt.translate(0x400100), TranslateResult::Ok(0x90000100));
    }

    #[test]
    fn test_map_overwrite() {
        let mut pt = Sv39PageTable::new();
        pt.map_page(0x1000, 0x80001000, PTE_V | PTE_R);
        assert_eq!(pt.translate(0x1000), TranslateResult::Ok(0x80001000));

        pt.map_page(0x1000, 0x90002000, PTE_V | PTE_R);
        assert_eq!(pt.translate(0x1000), TranslateResult::Ok(0x90002000));
    }

    #[test]
    fn test_superpage_mapping() {
        let mut pt = Sv39PageTable::new();
        // 2MB 大页映射：VA 0x200000 -> PA 0x80200000
        pt.map_superpage(0x200000, 0x80200000, PTE_V | PTE_R | PTE_W);

        // 大页内不同偏移都应命中
        assert_eq!(pt.translate(0x200000), TranslateResult::Ok(0x80200000));
        assert_eq!(pt.translate(0x200ABC), TranslateResult::Ok(0x80200ABC));
        assert_eq!(pt.translate(0x2FF000), TranslateResult::Ok(0x802FF000));
    }

    #[test]
    fn test_superpage_and_normal_coexist() {
        let mut pt = Sv39PageTable::new();
        // 大页映射在第一个 2MB 区域
        pt.map_superpage(0x0, 0x80000000, PTE_V | PTE_R);
        // 普通页在不同的 VPN[2] 区域
        pt.map_page(0x40000000, 0x90001000, PTE_V | PTE_R);

        assert_eq!(pt.translate(0x100), TranslateResult::Ok(0x80000100));
        assert_eq!(pt.translate(0x40000000), TranslateResult::Ok(0x90001000));
    }
}
