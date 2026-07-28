#![cfg_attr(target_arch = "xtensa", no_main)]
#![cfg_attr(target_arch = "xtensa", no_std)]
#![doc = "RTC and e-paper hardware bring-up diagnostic firmware."]

#[cfg(target_arch = "xtensa")]
use esp_backtrace as _;

#[cfg(target_arch = "xtensa")]
esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(target_arch = "xtensa")]
#[esp_hal::main]
fn main() -> ! {
    match pokeviewer_firmware::run_hardware_diagnostics() {
        Ok(report) => esp_println::println!(
            "hardware diagnostics complete; RTC={:04}-{:02}-{:02} {:02}:{:02}:{:02}; alarm_was_pending={}; panel rail off",
            report.rtc_datetime.year,
            report.rtc_datetime.month,
            report.rtc_datetime.day,
            report.rtc_datetime.hour,
            report.rtc_datetime.minute,
            report.rtc_datetime.second,
            report.alarm_was_pending,
        ),
        Err(error) => esp_println::println!("hardware diagnostics failed: {error}"),
    }

    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(target_arch = "xtensa"))]
fn main() {
    eprintln!("use `cargo xtask firmware-diagnostic-build` for the ESP32-S3 diagnostic");
}
