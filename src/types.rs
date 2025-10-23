use rk3588_rs::{
    DrmVersion, RknpuAction, RknpuMemCreate, RknpuMemDestroy, RknpuMemMap, RknpuSubmit, RknpuTask, DRM_COMMAND_BASE, DRM_IOCTL_BASE, RKNPU_ACTION, RKNPU_MEM_CREATE, RKNPU_MEM_DESTROY, RKNPU_MEM_MAP, RKNPU_SUBMIT
};

const IOC_READ: u32 = 2;
const IOC_WRITE: u32 = 1;

const fn _iowr(ty: u8, nr: u32, size: usize) -> u32 {
    ((IOC_READ | IOC_WRITE) << 30) | ((size as u32) << 16) | ((ty as u32) << 8) | nr
}

const DRM_IOCTL_RKNPU_ACTION: u32 = _iowr(DRM_IOCTL_BASE, DRM_COMMAND_BASE + RKNPU_ACTION, 8);
const DRM_IOCTL_RKNPU_SUBMIT: u32 = _iowr(
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE + RKNPU_SUBMIT,
    core::mem::size_of::<RknpuSubmit>(),
);
const DRM_IOCTL_RKNPU_MEM_CREATE: u32 =
    _iowr(DRM_IOCTL_BASE, DRM_COMMAND_BASE + RKNPU_MEM_CREATE, core::mem::size_of::<RknpuMemCreate>());
const DRM_IOCTL_RKNPU_MEM_MAP: u32 = _iowr(DRM_IOCTL_BASE, DRM_COMMAND_BASE + RKNPU_MEM_MAP, core::mem::size_of::<RknpuMemMap>());
const DRM_IOCTL_RKNPU_MEM_DESTROY: u32 =
    _iowr(DRM_IOCTL_BASE, DRM_COMMAND_BASE + RKNPU_MEM_DESTROY, core::mem::size_of::<RknpuMemDestroy>());
const DRM_IOCTL_VERSION: u32 = _iowr(DRM_IOCTL_BASE, 0x00, core::mem::size_of::<DrmVersion>());
const DRM_IOCTL_RKNPU_MEM_SYNC: u32 =
    _iowr(DRM_IOCTL_BASE, DRM_COMMAND_BASE + 0x05, core::mem::size_of::<RknpuMemDestroy>());


/// NPU 核心标识
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpuCore {
    Npu0 = 0,
    Npu1 = 1,
    Npu2 = 2,
}

impl NpuCore {
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Npu0),
            1 => Some(Self::Npu1),
            2 => Some(Self::Npu2),
            _ => None,
        }
    }

    pub const fn index(&self) -> usize {
        *self as usize
    }

    pub const fn mask_bit(&self) -> u32 {
        1 << (*self as u32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RkBoard {
    Rk3588,
    Rk3568,
    Rv1106,
    Rk3562,
    Rk3583,
}

impl RkBoard {
    pub const fn num_cores(&self) -> usize {
        match self {
            Self::Rk3588 => 3,
            Self::Rk3583 => 2,
            Self::Rk3568 | Self::Rv1106 | Self::Rk3562 => 1,
        }
    }

    pub const fn core_mask(&self) -> u32 {
        match self {
            Self::Rk3588 => 0x7,                               // 0b111 - 3 cores
            Self::Rk3583 => 0x3,                               // 0b011 - 2 cores
            Self::Rk3568 | Self::Rv1106 | Self::Rk3562 => 0x1, // 0b001 - 1 core
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RkNpuIoctl {
    DrmIoctlVersion,
    RknpuAction,
    RknpuMemCreate,
    RknpuMemSync,
    RknpuMemDestroy,
    RknpuMemMap,
    RknpuSubmit,
}

impl RkNpuIoctl {
    pub const fn from_cmd(cmd: u32) -> Option<Self> {
        match cmd {
            DRM_IOCTL_VERSION => Some(Self::DrmIoctlVersion),
            DRM_IOCTL_RKNPU_ACTION => Some(Self::RknpuAction),
            DRM_IOCTL_RKNPU_MEM_CREATE => Some(Self::RknpuMemCreate),
            DRM_IOCTL_RKNPU_MEM_SYNC => Some(Self::RknpuMemSync),
            DRM_IOCTL_RKNPU_MEM_DESTROY => Some(Self::RknpuMemDestroy),
            DRM_IOCTL_RKNPU_MEM_MAP => Some(Self::RknpuMemMap),
            DRM_IOCTL_RKNPU_SUBMIT => Some(Self::RknpuSubmit),
            _ => None,
        }
    }
}

pub enum RknpuActionFlag {
    GetHwVersion = 0,
    GetDrvVersion = 1,
    GetFreq = 2,
    SetFreq = 3,
    GetVolt = 4,
    SetVolt = 5,
    ActReset = 6,
    GetBwPriority = 7,
    SetBwPriority = 8,
    GetBwExpect = 9,
    SetBwExpect = 10,
    GetBwTw = 11,
    SetBwTw = 12,
    ActClrTotalRwAmount = 13,
    GetDtWrAmount = 14,
    GetDtRdAmount = 15,
    GetWtRdAmount = 16,
    GetTotalRwAmount = 17,
    GetIommuEn = 18,
    SetProcNice = 19,
    PowerOn = 20,
    PowerOff = 21,
    GetTotalSramSize = 22,
    GetFreeSramSize = 23,
}

impl From<u32> for RknpuActionFlag {
    fn from(value: u32) -> Self {
        match value {
            0 => RknpuActionFlag::GetHwVersion,
            1 => RknpuActionFlag::GetDrvVersion,
            2 => RknpuActionFlag::GetFreq,
            3 => RknpuActionFlag::SetFreq,
            4 => RknpuActionFlag::GetVolt,
            5 => RknpuActionFlag::SetVolt,
            6 => RknpuActionFlag::ActReset,
            7 => RknpuActionFlag::GetBwPriority,
            8 => RknpuActionFlag::SetBwPriority,
            9 => RknpuActionFlag::GetBwExpect,
            10 => RknpuActionFlag::SetBwExpect,
            11 => RknpuActionFlag::GetBwTw,
            12 => RknpuActionFlag::SetBwTw,
            13 => RknpuActionFlag::ActClrTotalRwAmount,
            14 => RknpuActionFlag::GetDtWrAmount,
            15 => RknpuActionFlag::GetDtRdAmount,
            16 => RknpuActionFlag::GetWtRdAmount,
            17 => RknpuActionFlag::GetTotalRwAmount,
            18 => RknpuActionFlag::GetIommuEn,
            19 => RknpuActionFlag::SetProcNice,
            20 => RknpuActionFlag::PowerOn,
            21 => RknpuActionFlag::PowerOff,
            22 => RknpuActionFlag::GetTotalSramSize,
            23 => RknpuActionFlag::GetFreeSramSize,
            _ => panic!("Invalid RknpuActionEnum value: {}", value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RkNpuError {
    DomainNotFound,
    Timeout,
    UnsupportedVersion,
    InvalidInput,
    HardwareError,
}

pub type RkNpuResult<T> = Result<T, RkNpuError>;
