extern crate tracing as log;

pub mod encode;
pub mod heuristic;
pub mod process;
pub mod read_image;
pub mod util;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessConfig {
    pub max_width: u32,
    pub max_height: u32,
    pub max_pixels: u32,
}

pub use image::{self, ImageFormat};
