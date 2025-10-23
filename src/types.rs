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
pub enum RkNpuError {
    DomainNotFound,
    Timeout,
    UnsupportedVersion,
    HardwareError,
}

pub type RkNpuResult<T> = Result<T, RkNpuError>;
