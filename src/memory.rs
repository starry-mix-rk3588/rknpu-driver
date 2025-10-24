use memory_addr::VirtAddr;

use crate::types::RkNpuResult;

pub trait NpuAllocator {
    fn create_handle(&self, size: usize) -> RkNpuResult<(u32, u64, u64)>;
    fn destroy_handle(&self, handle: u32) -> bool;
    /// offset, size
    fn get_handle(&self, handle: u32) -> RkNpuResult<(u64, usize)>;
    fn user_to_kernel_addr(&self, user_addr: usize) -> RkNpuResult<VirtAddr>;
}
