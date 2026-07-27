#![no_std]
#![forbid(unsafe_code)]
#![doc = "Firmware boundary for the supported Waveshare V2 board."]

pub use pokeviewer_core::{DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAMEBUFFER_BYTES};

/// Exact hardware target supported by release firmware.
pub const BOARD_TARGET: &str = "Waveshare ESP32-S3-ePaper-1.54-EN V2";
