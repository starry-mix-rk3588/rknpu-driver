use super::types::RkBoard;

pub mod addresses {
    /// NPU 核心寄存器基地址
    ///
    /// 来自设备树: npu@fdab0000
    /// reg = <0x00 0xfdab0000 0x00 0x10000    # NPU0 核心
    ///        0x00 0xfdac0000 0x00 0x10000    # NPU1 核心  
    ///        0x00 0xfdad0000 0x00 0x10000>;  # NPU2 核心
    pub const NPU0_BASE: usize = 0xFDAB0000;
    pub const NPU1_BASE: usize = 0xFDAC0000;
    pub const NPU2_BASE: usize = 0xFDAD0000;

    /// 每个核心的寄存器空间大小 (64KB)
    pub const NPU_CORE_SIZE: usize = 0x10000;

    /// PMU1 (电源管理单元) 基地址
    pub const PMU1_BASE: usize = 0xFD8D8000;

    /// CRU (时钟复位单元) 基地址
    pub const CRU_BASE: usize = 0xFD7C0000;

    /// GPIO3 基地址
    pub const GPIO3_BASE: usize = 0xFEC40000;
}

/// CRU 软复位控制寄存器偏移和位定义
pub mod cru_softrst {
    /// NPU 软复位控制寄存器偏移
    pub const SOFTRST_CON_NPU: u32 = 0x0A00;

    /// NPU0 AXI 复位位
    pub const NPU0_AXI_SRST: u32 = 0;
    /// NPU0 AHB 复位位
    pub const NPU0_AHB_SRST: u32 = 1;

    /// NPU1 AXI 复位位
    pub const NPU1_AXI_SRST: u32 = 2;
    /// NPU1 AHB 复位位
    pub const NPU1_AHB_SRST: u32 = 3;

    /// NPU2 AXI 复位位
    pub const NPU2_AXI_SRST: u32 = 4;
    /// NPU2 AHB 复位位
    pub const NPU2_AHB_SRST: u32 = 5;

    /// 写使能掩码位移（RK 芯片特有的写保护机制）
    pub const WRITE_MASK_SHIFT: u32 = 16;
}

/// 中断清除值
pub const INT_CLEAR_VALUE: u32 = 0x1ffff;

pub const RK3588_NPU_VERSION: u32 = 0x46495245;

/// RKNPU 硬件配置
#[derive(Debug, Clone, Copy)]
pub struct RknpuConfig {
    /// 带宽优先级寄存器地址
    pub bw_priority_addr: u32,
    /// 带宽优先级寄存器长度
    pub bw_priority_length: u32,
    /// DMA 掩码位数
    pub dma_mask_bits: u32,
    /// PC 数据量缩放比例
    pub pc_data_amount_scale: u32,
    /// PC 任务编号位数
    pub pc_task_number_bits: u32,
    /// PC 任务编号掩码
    pub pc_task_number_mask: u32,
    /// PC 任务状态偏移
    pub pc_task_status_offset: u32,
    /// PC DMA 控制
    pub pc_dma_ctrl: u32,
    /// 带宽使能
    pub bw_enable: bool,
    /// 中断数量
    pub num_irqs: usize,
    /// 复位数量
    pub num_resets: usize,
    /// NBUF 物理地址
    pub nbuf_phyaddr: u64,
    /// NBUF 大小
    pub nbuf_size: u64,
    /// 最大提交数量
    pub max_submit_number: u64,
    /// 核心掩码
    pub core_mask: u32,
}

impl RknpuConfig {
    /// RK3562 配置
    ///
    /// 特性:
    /// - 1 个 NPU 核心
    /// - 40 位 DMA 地址
    /// - NBUF 支持
    pub const RK3562: Self = Self {
        bw_priority_addr: 0x0,
        bw_priority_length: 0x0,
        dma_mask_bits: 40,
        pc_data_amount_scale: 2,
        pc_task_number_bits: 16,
        pc_task_number_mask: 0xffff,
        pc_task_status_offset: 0x48,
        pc_dma_ctrl: 1,
        bw_enable: true,
        num_irqs: 1,
        num_resets: 1,
        nbuf_phyaddr: 0xfe400000,
        nbuf_size: 256 * 1024,
        max_submit_number: (1 << 16) - 1,
        core_mask: 0x1,
    };
    /// RK3568 配置
    ///
    /// 特性:
    /// - 1 个 NPU 核心
    /// - 32 位 DMA 地址
    /// - 支持带宽控制
    pub const RK3568: Self = Self {
        bw_priority_addr: 0xfe180008,
        bw_priority_length: 0x10,
        dma_mask_bits: 32,
        pc_data_amount_scale: 1,
        pc_task_number_bits: 12,
        pc_task_number_mask: 0xfff,
        pc_task_status_offset: 0x3c,
        pc_dma_ctrl: 0,
        bw_enable: true,
        num_irqs: 1,
        num_resets: 1,
        nbuf_phyaddr: 0,
        nbuf_size: 0,
        max_submit_number: (1 << 12) - 1,
        core_mask: 0x1,
    };
    /// RK3583 配置
    ///
    /// 特性:
    /// - 2 个 NPU 核心
    /// - 40 位 DMA 地址
    pub const RK3583: Self = Self {
        bw_priority_addr: 0x0,
        bw_priority_length: 0x0,
        dma_mask_bits: 40,
        pc_data_amount_scale: 2,
        pc_task_number_bits: 12,
        pc_task_number_mask: 0xfff,
        pc_task_status_offset: 0x3c,
        pc_dma_ctrl: 0,
        bw_enable: false,
        num_irqs: 2,
        num_resets: 2,
        nbuf_phyaddr: 0,
        nbuf_size: 0,
        max_submit_number: (1 << 12) - 1,
        core_mask: 0x3,
    };
    /// RK3588 配置
    ///
    /// 特性:
    /// - 3 个 NPU 核心
    /// - 40 位 DMA 地址
    /// - 不支持带宽控制
    pub const RK3588: Self = Self {
        bw_priority_addr: 0x0,
        bw_priority_length: 0x0,
        dma_mask_bits: 40,
        pc_data_amount_scale: 2,
        pc_task_number_bits: 12,
        pc_task_number_mask: 0xfff,
        pc_task_status_offset: 0x3c,
        pc_dma_ctrl: 0,
        bw_enable: false,
        num_irqs: 3,
        num_resets: 3,
        nbuf_phyaddr: 0,
        nbuf_size: 0,
        max_submit_number: (1 << 12) - 1,
        core_mask: 0x7,
    };
    /// RV1106 配置
    ///
    /// 特性:
    /// - 1 个 NPU 核心
    /// - 32 位 DMA 地址
    /// - 16 位任务编号
    pub const RV1106: Self = Self {
        bw_priority_addr: 0x0,
        bw_priority_length: 0x0,
        dma_mask_bits: 32,
        pc_data_amount_scale: 2,
        pc_task_number_bits: 16,
        pc_task_number_mask: 0xffff,
        pc_task_status_offset: 0x3c,
        pc_dma_ctrl: 0,
        bw_enable: true,
        num_irqs: 1,
        num_resets: 1,
        nbuf_phyaddr: 0,
        nbuf_size: 0,
        max_submit_number: (1 << 16) - 1,
        core_mask: 0x1,
    };

    /// 根据板型获取配置
    pub const fn from_board(board: RkBoard) -> Self {
        match board {
            RkBoard::Rk3588 => Self::RK3588,
            RkBoard::Rk3568 => Self::RK3568,
            RkBoard::Rv1106 => Self::RV1106,
            RkBoard::Rk3562 => Self::RK3562,
            RkBoard::Rk3583 => Self::RK3583,
        }
    }

    /// 获取核心数量
    pub const fn num_cores(&self) -> usize {
        match self.core_mask {
            0x7 => 3, // RK3588
            0x3 => 2, // RK3583
            0x1 => 1, // RK3568, RV1106, RK3562
            _ => 0,
        }
    }

    /// 检查核心是否可用
    pub const fn is_core_available(&self, core: usize) -> bool {
        if core >= 3 {
            return false;
        }
        (self.core_mask & (1 << core)) != 0
    }
}
