#![cfg_attr(target_arch = "xtensa", no_main)]
#![cfg_attr(target_arch = "xtensa", no_std)]
#![doc = "Pokeviewer firmware entry point."]

#[cfg(target_arch = "xtensa")]
use esp_backtrace as _;

#[cfg(target_arch = "xtensa")]
esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(target_arch = "xtensa")]
#[esp_hal::main]
fn main() -> ! {
    let _board = pokeviewer_firmware::Board::initialize();
    esp_println::println!("Pokeviewer firmware bootstrap");

    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(target_arch = "xtensa"))]
fn main() {
    eprintln!("use `cargo xtask firmware-build` to build the ESP32-S3 firmware");
}
