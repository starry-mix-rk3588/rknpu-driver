use tock_registers::{
    register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

register_structs! {
    pub RknpuCruRegisters {
        (0x0000 => _reserved0),
        
        /// NPU 软复位控制寄存器 (偏移 0x0A00)
        /// 
        /// RK 芯片的写保护机制：
        /// - 高 16 位为写使能掩码 (write mask)
        /// - 低 16 位为实际的复位控制位
        /// - 写入时需要同时设置对应的掩码位
        /// 
        /// 复位位定义：
        /// - Bit 0: NPU0 AXI 复位
        /// - Bit 1: NPU0 AHB 复位
        /// - Bit 2: NPU1 AXI 复位
        /// - Bit 3: NPU1 AHB 复位
        /// - Bit 4: NPU2 AXI 复位
        /// - Bit 5: NPU2 AHB 复位
        (0x0A00 => pub softrst_con_npu: ReadWrite<u32>),
        
        (0x0A04 => @END),
    }
}

register_structs! {
    pub RknpuRegisters {
        /// Version register
        (0x0000 => pub version: ReadOnly<u32>),

        /// Version number register
        (0x0004 => pub version_num: ReadOnly<u32>),

        /// PC operation enable register
        (0x0008 => pub pc_op_en: ReadWrite<u32>),

        (0x000C => _reserved0),

        /// PC data address register
        (0x0010 => pub pc_data_addr: ReadWrite<u32>),

        /// PC data amount register
        (0x0014 => pub pc_data_amount: ReadWrite<u32>),

        (0x0018 => _reserved1),

        /// Interrupt mask register
        (0x0020 => pub int_mask: ReadWrite<u32>),

        /// Interrupt clear register
        (0x0024 => pub int_clear: WriteOnly<u32>),

        /// Interrupt status register
        (0x0028 => pub int_status: ReadOnly<u32>),

        /// Interrupt raw status register
        (0x002C => pub int_raw_status: ReadOnly<u32>),

        /// PC task control register
        (0x0030 => pub pc_task_control: ReadWrite<u32>),

        /// PC DMA base address register
        (0x0034 => pub pc_dma_base_addr: ReadWrite<u32>),

        (0x0038 => _reserved2),

        /// PC task status register
        (0x003C => pub pc_task_status: ReadOnly<u32>),

        (0x0040 => _reserved3),

        (0x8010 => pub clr_all_rw_amount: WriteOnly<u32>),

        (0x8014 => _reserved4),

        /// Data write amount register
        (0x8034 => pub dt_wr_amount: ReadOnly<u32>),

        /// Data read amount register
        (0x8038 => pub dt_rd_amount: ReadOnly<u32>),

        /// Weight read amount register
        (0x803C => pub wt_rd_amount: ReadOnly<u32>),

        (0x8040 => _reserved5),

        /// Enable mask register (at offset 0xF008)
        (0xF008 => pub enable_mask: ReadWrite<u32>),

        (0xF00C => _reserved6),

        (0xF010 => @END),
    }
}
