use core::ptr::{NonNull, addr_of};

use log::{debug, error, info};
use memory_addr::{PhysAddr, VirtAddr, pa};
use rk3588_rs::{
    RKNPU_JOB_PINGPONG, RKNPU_PC_DATA_EXTRA_AMOUNT, RknpuAction, RknpuMemSync, RknpuSubmit,
    RknpuTask,
};
use rockchip_pm::{PD, RockchipPM};
use tock_registers::interfaces::{Readable, Writeable};

use crate::{
    configs::{RK3588_NPU_VERSION, RknpuConfig},
    registers::{RknpuCruRegisters, RknpuRegisters},
    types::{NpuCore, RkBoard, RkNpuError, RkNpuResult, RknpuActionFlag},
};

pub struct RknpuDev {
    config: RknpuConfig,
    core_base: usize,
    cru_base: usize,
    pm_base: usize
}

#[inline(always)]
pub unsafe fn dcache_flush_range(start: usize, size: usize) {
    let mut addr = start & !0x3F; // cache line 对齐
    let end = start + size;

    while addr < end {
        unsafe {
            core::arch::asm!(
                "dc cvac, {0}",
                in(reg) addr,
                options(nostack, preserves_flags)
            );
        }

        addr += 64; // 每次 64 bytes (cache line)
    }
    unsafe {
        core::arch::asm!("dsb ish", "isb", options(nostack, preserves_flags));
    }
}

#[inline(always)]
pub unsafe fn dcache_invalidate_range(start: usize, size: usize) {
    let mut addr = start & !0x3F;
    let end = start + size;

    while addr < end {
        unsafe {
            core::arch::asm!(
                "dc ivac, {0}",
                in(reg) addr,
                options(nostack, preserves_flags)
            );
        }
        addr += 64;
    }
    unsafe {
        core::arch::asm!("dsb ish", "isb", options(nostack, preserves_flags));
    }
}

/// NPU 主电源域
pub const NPU: PD = PD(8);
/// NPU TOP 电源域  
pub const NPUTOP: PD = PD(9);
/// NPU1 电源域
pub const NPU1: PD = PD(10);
/// NPU2 电源域
pub const NPU2: PD = PD(11);

impl RknpuDev {
    pub fn new(base: usize, cru_base: usize, pm_base: usize, board: RkBoard) -> Self {
        RknpuDev {
            config: RknpuConfig::from_board(board),
            core_base: base,
            cru_base,
            pm_base,
        }
    }

    const fn core_regs(&self) -> &RknpuRegisters {
        unsafe { &*(self.core_base as *const _) }
    }

    const fn cru_regs(&self) -> &RknpuCruRegisters {
        unsafe { &*(self.cru_base as *const _) }
    }

    pub fn initialize(&mut self) -> RkNpuResult<()> {
        // Convert pm_base (usize) to NonNull<u8> expected by RockchipPM::new
        let base_ptr = NonNull::new(self.pm_base as *mut u8)
            .ok_or(RkNpuError::InvalidInput)?;
        let mut pm = RockchipPM::new(base_ptr, rockchip_pm::RkBoard::Rk3588);
        pm.power_domain_on(NPU1).unwrap();
        pm.power_domain_on(NPU2).unwrap();
        pm.power_domain_on(NPU).unwrap();
        pm.power_domain_on(NPUTOP).unwrap();

        self.check_hardware_version()?;
        Ok(())
    }

    pub fn rknpu_action_ioctl(&self, action: &mut RknpuAction) -> RkNpuResult<()> {
        match RknpuActionFlag::from(action.flags) {
            RknpuActionFlag::GetHwVersion => {
                action.value = self.core_regs().version.get();
            }
            RknpuActionFlag::ActReset => {
                debug!("[RKNPU] Performing hardware reset");
                // self.soft_reset()?;
            }
            _ => {
                error!("[RKNPU] Unsupported action flag: 0x{:x}", action.flags);
                return Err(RkNpuError::InvalidInput);
            }
        }
        Ok(())
    }

    pub fn rknpu_submit_ioctl(
        &self,
        submit: &mut RknpuSubmit,
        dma_to_kernel: fn(PhysAddr) -> VirtAddr,
    ) -> RkNpuResult<()> {
        debug!(
            "[RKNPU] SUBMIT: task_obj_addr=0x{:x}, task_number={}, flags=0x{:x}, timeout={}, \
             core_mask=0x{:x}",
            submit.task_obj_addr,
            submit.task_number,
            submit.flags,
            submit.timeout,
            self.config.core_mask
        );

        // 验证输入参数
        if submit.task_number == 0 {
            info!("[RKNPU] Invalid task_number: 0");
            return Err(RkNpuError::InvalidInput);
        }

        if submit.task_obj_addr == 0 {
            info!("[RKNPU] Invalid task_obj_addr: 0");
            return Err(RkNpuError::InvalidTaskAddress);
        }

        let task_base =
            dma_to_kernel(pa!(submit.task_obj_addr as usize)).as_mut_ptr() as *const RknpuTask;

        debug!(
            "[RKNPU] Checking interrupt status before submission: 0x{:x}",
            self.core_regs().int_status.get()
        );
        debug!(
            "[RKNPU] Checking raw interrupt status: 0x{:x}",
            self.core_regs().int_raw_status.get()
        );

        // 提交任务到硬件
        self.job_commit_pc(task_base, submit)?;

        // 等待任务完成
        let timeout = if submit.timeout > 0 {
            submit.timeout
        } else {
            5000 // 默认5秒超时
        };

        // todo: get mem pool base addr
        self.wait_job_done(timeout, task_base as usize - 0x1000usize)?;

        debug!("[RKNPU] Task submission completed successfully");
        Ok(())
    }

    pub fn rknpu_mem_sync_ioctl(&self, _mem_sync: &RknpuMemSync) -> RkNpuResult<()> {
        // Handle RKNPU_MEM_SYNC ioctl
        Ok(())
    }

    fn check_hardware_version(&self) -> RkNpuResult<()> {
        let version = self.core_regs().version.get();
        if version == RK3588_NPU_VERSION {
            Ok(())
        } else {
            Err(RkNpuError::UnsupportedVersion)
        }
    }

    /// PC 模式硬件任务提交
    fn job_commit_pc(
        &self,
        task_base: *const RknpuTask,
        submit: &mut RknpuSubmit,
    ) -> RkNpuResult<()> {
        if task_base.is_null() {
            return Err(RkNpuError::InvalidTaskAddress);
        }

        debug!(
            "[RKNPU] Committing PC job: task_base={:x}, task_start={}, task_number={}, \
             flags=0x{:x}",
            task_base as usize, submit.task_start, submit.task_number, submit.flags
        );

        unsafe {
            let task_end = submit.task_start + submit.task_number - 1;
            let first_task = task_base.add(submit.task_start as usize);
            let last_task = task_base.add(task_end as usize);

            // todo: get task mem size
            dcache_flush_range(task_base as usize, 1024);
            let reg_addr_kva = core::ptr::read_unaligned(addr_of!((*first_task).regcmd_addr))
                + 0xffff_0000_0000_0000;

            dcache_flush_range(reg_addr_kva as usize, 8 * 1024 * 1024);

            debug!(
                "[RKNPU] First task addr 0x{:x}, int_mask {}, regcmd_addr 0x{:x}",
                first_task as usize,
                core::ptr::read_unaligned(addr_of!((*first_task).int_mask)),
                core::ptr::read_unaligned(addr_of!((*first_task).regcmd_addr))
            );

            let tasks = &mut *(first_task as *mut RknpuTask);
            debug!("{:#?}", tasks);

            // 读取第一个任务的配置（使用 read_unaligned 因为是 packed struct）
            let first_regcmd_addr = core::ptr::read_unaligned(addr_of!((*first_task).regcmd_addr));
            let first_regcfg_amount =
                core::ptr::read_unaligned(addr_of!((*first_task).regcfg_amount));
            let first_int_clear = core::ptr::read_unaligned(addr_of!((*first_task).int_clear));

            // 读取最后一个任务的中断掩码
            let last_int_mask = core::ptr::read_unaligned(addr_of!((*last_task).int_mask));

            let pc_data_amount_scale = self.config.pc_data_amount_scale;
            let task_pp_en = if submit.flags & RKNPU_JOB_PINGPONG != 0 {
                1
            } else {
                0
            };
            let pc_task_number_bits = self.config.pc_task_number_bits;

            debug!(
                "[RKNPU] Committing PC job: task_start={}, task_number={}",
                submit.task_start, submit.task_number
            );
            debug!(
                "[RKNPU] First task regcmd_addr=0x{:x}, regcfg_amount={}",
                first_regcmd_addr, first_regcfg_amount
            );

            // 1. 切换到 slave 模式
            self.core_regs().pc_data_addr.set(0x1);

            // 2. 写 regcmd 地址（只使用低32位）
            self.core_regs().pc_data_addr.set(first_regcmd_addr as u32);

            // 3. 计算并写数据量
            let data_amount =
                (first_regcfg_amount + RKNPU_PC_DATA_EXTRA_AMOUNT + pc_data_amount_scale - 1)
                    / pc_data_amount_scale
                    - 1;
            debug!("[RKNPU] Data amount: {}", data_amount);
            self.core_regs().pc_data_amount.set(data_amount);

            // 4. 写中断掩码
            self.core_regs().int_mask.set(last_int_mask);

            // 5. 清除中断
            self.core_regs().int_clear.set(first_int_clear);

            // 6. 写任务控制
            let pc_task_control = ((0x6 | task_pp_en) << pc_task_number_bits) | submit.task_number;
            debug!("[RKNPU] PC task control: 0x{:x}", pc_task_control);
            self.core_regs().pc_task_control.set(pc_task_control);

            // 7. 提交任务
            self.core_regs().pc_op_en.set(0x1);
            self.core_regs().pc_op_en.set(0x0);

            debug!("[RKNPU] Task submitted to hardware");
        }

        Ok(())
    }

    /// 等待任务完成
    fn wait_job_done(&self, timeout_ms: u32, pool_start: usize) -> RkNpuResult<()> {
        debug!(
            "[RKNPU] Waiting for job completion (timeout: {}ms)",
            timeout_ms
        );

        // 简单的轮询实现，每次检查间隔约10微秒
        let max_iterations = (timeout_ms as usize) * 100; // 10us * 100 = 1ms

        for i in 0..max_iterations {
            let int_status = self.core_regs().int_status.get();

            // 检查中断状态（任何非零值表示有中断）
            if int_status == 0x100 || int_status == 0x200 {
                debug!(
                    "[RKNPU] Job completed after {} iterations, int_status=0x{:x}",
                    i, int_status
                );

                debug!("dcache {:#x}", pool_start);
                unsafe {
                    dcache_invalidate_range(pool_start, 8 * 1024 * 1024);
                }

                // 清除中断
                self.core_regs().int_clear.set(int_status);

                return Ok(());
            }

            // 简单延迟（实际延迟取决于系统）
            for _ in 0..100 {
                core::hint::spin_loop();
            }
        }

        info!("[RKNPU] Job timeout after {}ms, status=0x{:x}", timeout_ms, self.core_regs().int_status.get());
        Err(RkNpuError::TaskTimeout)
    }

    pub fn handle_irq(&self, _core: NpuCore) -> RkNpuResult<u32> {
        let int_status = self.core_regs().int_status.get();
        if int_status != 0 {
            // 清除中断
            self.core_regs().int_clear.set(int_status);
            Ok(int_status)
        } else {
            Err(RkNpuError::NoInterrupt)
        }
    }

    /// 微秒级延迟
    fn delay_us(&self, us: u32) {
        // 简单的忙等待实现
        for _ in 0..(us * 100) {
            core::hint::spin_loop();
        }
    }

    /// 清除中断状态
    fn clear_interrupts(&self) -> RkNpuResult<()> {
        use crate::configs::INT_CLEAR_VALUE;
        self.core_regs().int_clear.set(INT_CLEAR_VALUE);
        info!("[RKNPU] Interrupts cleared");
        Ok(())
    }

    /// 禁用所有使能位
    fn disable_enables(&self) -> RkNpuResult<()> {
        // 禁用 PC 操作
        self.core_regs().pc_op_en.set(0);
        // 清除使能掩码
        self.core_regs().enable_mask.set(0);
        info!("[RKNPU] All enables disabled");
        Ok(())
    }

    /// 执行 AXI 总线复位
    ///
    /// AXI 复位会重置 NPU 的 AXI 总线接口
    fn reset_axi(&self) -> RkNpuResult<()> {
        use crate::configs::cru_softrst::*;

        info!("[RKNPU] Performing AXI reset");

        // 只复位 NPU0 核心（当前只使用单核）
        let reset_bit = NPU0_AXI_SRST;

        // RK 芯片的写保护机制：高 16 位为写使能掩码
        // 步骤 1: 置位 - 触发复位
        let set_value = (1 << (reset_bit + WRITE_MASK_SHIFT)) | (1 << reset_bit);
        self.cru_regs().softrst_con_npu.set(set_value);

        // 步骤 2: 等待复位生效（至少 10us）
        self.delay_us(10);

        // 步骤 3: 清零 - 释放复位
        let clear_value = (1 << (reset_bit + WRITE_MASK_SHIFT)) | (0 << reset_bit);
        self.cru_regs().softrst_con_npu.set(clear_value);

        // 步骤 4: 等待稳定
        self.delay_us(5);

        info!("[RKNPU] AXI reset completed");
        Ok(())
    }

    /// 执行 AHB 总线复位
    ///
    /// AHB 复位会重置 NPU 的 AHB 总线接口
    fn reset_ahb(&self) -> RkNpuResult<()> {
        use crate::configs::cru_softrst::*;

        info!("[RKNPU] Performing AHB reset");

        // 只复位 NPU0 核心（当前只使用单核）
        let reset_bit = NPU0_AHB_SRST;

        // RK 芯片的写保护机制：高 16 位为写使能掩码
        // 步骤 1: 置位 - 触发复位
        let set_value = (1 << (reset_bit + WRITE_MASK_SHIFT)) | (1 << reset_bit);
        self.cru_regs().softrst_con_npu.set(set_value);

        // 步骤 2: 等待复位生效（至少 10us）
        self.delay_us(10);

        // 步骤 3: 清零 - 释放复位
        let clear_value = (1 << (reset_bit + WRITE_MASK_SHIFT)) | (0 << reset_bit);
        self.cru_regs().softrst_con_npu.set(clear_value);

        // 步骤 4: 等待稳定
        self.delay_us(5);

        info!("[RKNPU] AHB reset completed");
        Ok(())
    }

    /// 执行软复位
    ///
    /// 软复位会重置 NPU 的状态，包括：
    /// 1. 清除中断状态
    /// 2. 禁用所有使能位
    /// 3. 执行 AXI 总线复位
    /// 4. 执行 AHB 总线复位
    ///
    /// 基于 C 驱动中的 rknpu_soft_reset() 函数实现
    pub fn soft_reset(&self) -> RkNpuResult<()> {
        info!("[RKNPU] Starting soft reset");

        // 1. 清除中断状态
        self.clear_interrupts()?;

        // 2. 禁用所有使能位
        // self.disable_enables()?;

        // 3. 执行 AXI 复位
        self.reset_axi()?;

        // 4. 执行 AHB 复位
        self.reset_ahb()?;

        // 5. 等待复位完成
        self.delay_us(10);

        // Convert pm_base (usize) to NonNull<u8> expected by RockchipPM::new
        let base_ptr = NonNull::new(self.pm_base as *mut u8)
            .ok_or(RkNpuError::InvalidInput)?;
        let mut pm = RockchipPM::new(base_ptr, rockchip_pm::RkBoard::Rk3588);
        pm.power_domain_off(NPU1).unwrap();
        pm.power_domain_off(NPU2).unwrap();
        pm.power_domain_off(NPU).unwrap();
        pm.power_domain_off(NPUTOP).unwrap();

        self.delay_us(1000); // 等待 1ms

        pm.power_domain_on(NPUTOP).unwrap();
        pm.power_domain_on(NPU).unwrap();
        pm.power_domain_on(NPU1).unwrap();
        pm.power_domain_on(NPU2).unwrap();

        info!("[RKNPU] Soft reset completed successfully");
        Ok(())
    }
}
