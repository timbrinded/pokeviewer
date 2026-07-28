#![cfg_attr(target_arch = "xtensa", no_main)]
#![cfg_attr(target_arch = "xtensa", no_std)]
#![doc = "Safe invalid-RTC terminal-policy diagnostic firmware."]

#[cfg(target_arch = "xtensa")]
use esp_backtrace as _;

#[cfg(target_arch = "xtensa")]
esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(target_arch = "xtensa")]
#[esp_hal::main]
fn main() -> ! {
    pokeviewer_firmware::run_failure_diagnostic(pokeviewer_firmware::FailureKind::InvalidRtc)
}

#[cfg(not(target_arch = "xtensa"))]
fn main() {
    eprintln!("build this diagnostic for the ESP32-S3 target");
}
