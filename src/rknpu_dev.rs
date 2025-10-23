use core::ptr::NonNull;

use log::info;
use rk3588_rs::{
    RknpuAction, RknpuMemCreate, RknpuMemDestroy, RknpuMemMap, RknpuMemSync, RknpuSubmit,
};
use rockchip_pm::{PD, RockchipPM};
use tock_registers::interfaces::Readable;

use crate::{
    configs::{RK3588_NPU_VERSION, RknpuConfig},
    registers::RknpuRegisters,
    types::{RkBoard, RkNpuError, RkNpuResult, RknpuActionFlag},
};

pub struct RknpuDev {
    config: RknpuConfig,
    core_base: usize,
    pub is_initialized: bool,
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
            is_initialized: false,
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

        // IRQ Register Initialization

        self.is_initialized = true;
        Ok(())
    }

    pub fn rknpu_action_ioctl(&self, action: &mut RknpuAction) -> RkNpuResult<()> {
        match RknpuActionFlag::from(action.flags) {
            RknpuActionFlag::GetHwVersion => {
                action.value = self.core_regs().version.get();
            }
            _ => {
                return Err(RkNpuError::InvalidInput);
            }
        }
        Ok(())
    }

    pub fn rknpu_submit_ioctl(&self, submit: &RknpuSubmit) -> RkNpuResult<()> {
        info!(
            "[RKNPU] SUBMIT: task_obj_addr=0x{:x}, task_number={}, flags=0x{:x}, timeout={}, core_mask=0x{:x}",
            submit.task_obj_addr, submit.task_number, submit.flags, submit.timeout, self.config.core_mask
        );
        Ok(())
    }

    pub fn rknpu_mem_create_ioctl(&self, _mem_create: &RknpuMemCreate) -> RkNpuResult<()> {
        // Handle RKNPU_MEM_CREATE ioctl
        Ok(())
    }

    pub fn rknpu_mem_map_ioctl(&self, _mem_map: &RknpuMemMap) -> RkNpuResult<()> {
        // Handle RKNPU_MEM_MAP ioctl
        Ok(())
    }

    pub fn rknpu_mem_destroy_ioctl(&self, _mem_destroy: &RknpuMemDestroy) -> RkNpuResult<()> {
        // Handle RKNPU_MEM_DESTROY ioctl
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
}
