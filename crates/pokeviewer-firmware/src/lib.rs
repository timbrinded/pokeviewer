#![no_std]
#![forbid(unsafe_code)]
#![doc = "Firmware boundary for the supported Waveshare V2 board."]

pub use pokeviewer_core::{
    DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAMEBUFFER_BYTES, InvalidDateTime, LocalDateTime, Weekday,
};

mod application;
#[cfg(target_arch = "xtensa")]
mod board;
#[cfg(any(target_arch = "xtensa", test))]
mod bounded_busy;
#[cfg(any(target_arch = "xtensa", test))]
mod es8311;
mod failure;
#[cfg(target_arch = "xtensa")]
mod panel;
#[cfg(any(target_arch = "xtensa", test))]
mod pcf85063;
mod protocol;
mod rtc;
#[cfg(target_arch = "xtensa")]
mod runtime;
#[cfg(target_arch = "xtensa")]
mod sleep;
#[cfg(test)]
mod test_i2c;
#[cfg(target_arch = "xtensa")]
mod usb_protocol;

pub use application::{
    ApplicationError, RenderedFrame, RetainedCard, Screen, WakePlan, plan_wake, render_rtc_frame,
};
#[cfg(target_arch = "xtensa")]
pub use board::{
    HardwareDiagnosticReport, run_hardware_diagnostics, run_sleep_diagnostic, run_usb_provisioning,
};
pub use failure::{FailureKind, FailurePolicy, RecoveryAction, render_failure_screen};
#[cfg(target_arch = "xtensa")]
pub use pcf85063::{Pcf85063Rtc, Pcf85063RtcError};
pub use protocol::handle_protocol_request;
pub use rtc::{FakeRtc, FakeRtcError, Rtc};
#[cfg(target_arch = "xtensa")]
pub use runtime::run_pokeviewer;
#[cfg(target_arch = "xtensa")]
pub use usb_protocol::{UsbProtocolError, UsbProtocolTransport};

/// Exact hardware target supported by release firmware.
pub const BOARD_TARGET: &str = "Waveshare ESP32-S3-ePaper-1.54-EN V2";
