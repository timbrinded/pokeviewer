#![cfg_attr(target_arch = "xtensa", no_main)]
#![cfg_attr(target_arch = "xtensa", no_std)]
#![doc = "Timer-only deep-sleep isolation firmware."]

#[cfg(target_arch = "xtensa")]
use esp_backtrace as _;

#[cfg(target_arch = "xtensa")]
esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(target_arch = "xtensa")]
#[esp_hal::main]
fn main() -> ! {
    pokeviewer_firmware::run_timer_sleep_diagnostic()
}

#[cfg(not(target_arch = "xtensa"))]
fn main() {
    eprintln!("use `cargo xtask timer-sleep-diagnostic-build` for the ESP32-S3 diagnostic");
}
