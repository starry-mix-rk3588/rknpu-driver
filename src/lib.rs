pub mod configs;
pub mod registers;
mod rknpu_dev;
pub mod types;
mod ioctl;

pub use rknpu_dev::*;
pub use ioctl::rknpu_ioctl;