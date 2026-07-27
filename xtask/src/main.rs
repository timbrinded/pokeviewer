#![forbid(unsafe_code)]
#![doc = "Cargo-native repository automation."]

use std::env;
use std::process::{Command, ExitCode};

const ESP_TOOLCHAIN: &str = "esp-1.95.0.0";
const ESP_TOOLCHAIN_ARG: &str = "+esp-1.95.0.0";
const ESP_TARGET: &str = "xtensa-esp32s3-none-elf";

fn main() -> ExitCode {
    match env::args().nth(1).as_deref() {
        None | Some("help" | "--help" | "-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("firmware-build") => run_cargo(&firmware_args("build")),
        Some("firmware-flash") => run_cargo(&firmware_args("run")),
        Some(command) => {
            eprintln!("unknown xtask command: {command}");
            ExitCode::from(2)
        }
    }
}

fn firmware_args(action: &'static str) -> [&'static str; 9] {
    [
        ESP_TOOLCHAIN_ARG,
        action,
        "--package",
        "pokeviewer-firmware",
        "--bin",
        "pokeviewer-firmware",
        "--target",
        ESP_TARGET,
        "--locked",
    ]
}

fn run_cargo(args: &[&str]) -> ExitCode {
    let status = Command::new("cargo").args(args).arg("--release").status();

    match status {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("failed to run cargo with toolchain {ESP_TOOLCHAIN}: {error}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "\
Pokeviewer repository tasks

USAGE:
    cargo xtask <COMMAND>

COMMANDS:
    firmware-build    Build release firmware
    firmware-flash    Build, flash, and monitor release firmware
    help              Print this help"
    );
}

#[cfg(test)]
mod tests {
    use super::{ESP_TARGET, firmware_args};

    #[test]
    fn firmware_commands_select_the_embedded_target() {
        let args = firmware_args("build");

        assert_eq!(args[1], "build");
        assert!(args.contains(&ESP_TARGET));
        assert!(args.contains(&"--locked"));
    }
}
