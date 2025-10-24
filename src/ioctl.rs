use log::info;
use rk3588_rs::{
    DrmVersion, RknpuAction, RknpuMemSync,
    RknpuSubmit,
};

use crate::{
    RknpuDev,
    types::{RkNpuError, RkNpuIoctl, RkNpuResult},
};

pub fn rknpu_ioctl(rknpu: &RknpuDev, rknpu_cmd: Option<RkNpuIoctl>, arg: usize) -> RkNpuResult<()> {
    info!("rknpu ioctl => cmd: {:?}, arg: {:#x}", rknpu_cmd, arg);
    match rknpu_cmd {
        Some(RkNpuIoctl::DrmIoctlVersion) => {
            let drm_ver = unsafe { &mut *(arg as *mut DrmVersion) };
            drm_ver.version_major = 1;
            drm_ver.version_minor = 0;
            drm_ver.version_patchlevel = 0;

            if !drm_ver.name.is_null() && drm_ver.name_len > 0 {
                let name = b"rknpu\0";
                let copy_len = core::cmp::min(name.len(), drm_ver.name_len);
                unsafe {
                    core::ptr::copy_nonoverlapping(name.as_ptr(), drm_ver.name, copy_len);
                }
                drm_ver.name_len = copy_len;
            }

            if !drm_ver.date.is_null() && drm_ver.date_len > 0 {
                let date = b"20251023\0";
                let copy_len = core::cmp::min(date.len(), drm_ver.date_len);
                unsafe {
                    core::ptr::copy_nonoverlapping(date.as_ptr(), drm_ver.date, copy_len);
                }
                drm_ver.date_len = copy_len;
            }

            if !drm_ver.desc.is_null() && drm_ver.desc_len > 0 {
                let desc = b"Rockchip NPU Simulated\0";
                let copy_len = core::cmp::min(desc.len(), drm_ver.desc_len);
                unsafe {
                    core::ptr::copy_nonoverlapping(desc.as_ptr(), drm_ver.desc, copy_len);
                }
                drm_ver.desc_len = copy_len;
            }
            Ok(())
        }
        Some(RkNpuIoctl::RknpuAction) => {
            let action = unsafe { &mut *(arg as *mut RknpuAction) };
            rknpu.rknpu_action_ioctl(action)
        }
        Some(RkNpuIoctl::RknpuSubmit) => {
            let submit = unsafe { &mut *(arg as *mut RknpuSubmit) };
            rknpu.rknpu_submit_ioctl(submit)
        }
        Some(RkNpuIoctl::RknpuMemSync) => {
            let mem_sync = unsafe { &mut *(arg as *mut RknpuMemSync) };
            rknpu.rknpu_mem_sync_ioctl(mem_sync)
        }
        _ => Err(RkNpuError::InvalidInput),
    }
}
