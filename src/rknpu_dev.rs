use core::ptr::{NonNull, addr_of};

use log::info;
use memory_addr::{PhysAddr, VirtAddr, pa};
use rk3588_rs::{
    RKNPU_JOB_PINGPONG, RKNPU_PC_DATA_EXTRA_AMOUNT, RknpuAction, RknpuMemSync, RknpuSubmit,
    RknpuTask,
};
use rockchip_pm::{PD, RockchipPM};
use tock_registers::interfaces::{Readable, Writeable};

use crate::{
    configs::{RK3588_NPU_VERSION, RknpuConfig},
    registers::RknpuRegisters,
    types::{NpuCore, RkBoard, RkNpuError, RkNpuResult, RknpuActionFlag},
};

pub struct RknpuDev {
    config: RknpuConfig,
    core_base: usize,
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
    pub fn new(base: usize, board: RkBoard) -> Self {
        RknpuDev {
            config: RknpuConfig::from_board(board),
            core_base: base,
        }
    }

    const fn core_regs(&self) -> &RknpuRegisters {
        unsafe { &*(self.core_base as *const _) }
    }

    pub fn initialize(&mut self, pmu_base: NonNull<u8>) -> RkNpuResult<()> {
        let mut pm = RockchipPM::new(pmu_base, rockchip_pm::RkBoard::Rk3588);
        pm.power_domain_on(NPU1).unwrap();
        pm.power_domain_off(NPU2).unwrap();
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
                info!("[RKNPU] Hardware reset todo");
            }
            _ => {
                info!("[RKNPU] Unsupported action flag: 0x{:x}", action.flags);
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
        info!(
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

        info!(
            "[RKNPU] Checking interrupt status before submission: 0x{:x}",
            self.core_regs().int_status.get()
        );
        info!(
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

        self.wait_job_done(timeout)?;

        info!("[RKNPU] Task submission completed successfully");
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

        info!(
            "[RKNPU] Committing PC job: task_base={:x}, task_start={}, task_number={}, flags=0x{:x}",
            task_base as usize, submit.task_start, submit.task_number, submit.flags
        );

        unsafe {
            let task_end = submit.task_start + submit.task_number - 1;
            let first_task = task_base.add(submit.task_start as usize);
            let last_task = task_base.add(task_end as usize);
            info!(
                "[RKNPU] First task addr 0x{:x}, int_mask {}, regcmd_addr 0x{:x}",
                first_task as usize,
                core::ptr::read_unaligned(addr_of!((*first_task).int_mask)),
                core::ptr::read_unaligned(addr_of!((*first_task).regcmd_addr))
            );

            let tasks = &mut *(first_task as *mut RknpuTask);
            info!("{:#?}", tasks);


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

            info!(
                "[RKNPU] Committing PC job: task_start={}, task_number={}",
                submit.task_start, submit.task_number
            );
            info!(
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
            info!("[RKNPU] Data amount: {}", data_amount);
            self.core_regs().pc_data_amount.set(data_amount);

            // 4. 写中断掩码
            self.core_regs().int_mask.set(last_int_mask);

            // 5. 清除中断
            self.core_regs().int_clear.set(first_int_clear);

            // 6. 写任务控制
            let pc_task_control = ((0x6 | task_pp_en) << pc_task_number_bits) | submit.task_number;
            info!("[RKNPU] PC task control: 0x{:x}", pc_task_control);
            self.core_regs().pc_task_control.set(pc_task_control);

            // 7. 提交任务
            self.core_regs().pc_op_en.set(0x1);
            self.core_regs().pc_op_en.set(0x0);

            info!("[RKNPU] Task submitted to hardware");
        }

        Ok(())
    }

    /// 等待任务完成
    fn wait_job_done(&self, timeout_ms: u32) -> RkNpuResult<()> {
        info!(
            "[RKNPU] Waiting for job completion (timeout: {}ms)",
            timeout_ms
        );

        // 简单的轮询实现，每次检查间隔约10微秒
        let max_iterations = (timeout_ms as usize) * 100; // 10us * 100 = 1ms

        for i in 0..max_iterations {
            let int_status = self.core_regs().int_status.get();

            // 检查中断状态（任何非零值表示有中断）
            if int_status != 0 {
                info!(
                    "[RKNPU] Job completed after {} iterations, int_status=0x{:x}",
                    i, int_status
                );

                // 清除中断
                self.core_regs().int_clear.set(int_status);

                return Ok(());
            }

            // 简单延迟（实际延迟取决于系统）
            for _ in 0..100 {
                core::hint::spin_loop();
            }
        }

        info!("[RKNPU] Job timeout after {}ms", timeout_ms);
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
}
