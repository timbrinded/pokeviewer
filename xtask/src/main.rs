#![forbid(unsafe_code)]
#![doc = "Cargo-native repository automation."]

use std::env;
use std::process::{Command, ExitCode};

mod content;
mod render;

const ESP_TOOLCHAIN: &str = "esp-1.95.0.0";
const ESP_TOOLCHAIN_ARG: &str = "+esp-1.95.0.0";
const ESP_TARGET: &str = "xtensa-esp32s3-none-elf";
const RELEASE_FIRMWARE: &str = "pokeviewer-firmware";
const SLEEP_DIAGNOSTIC: &str = "pokeviewer-sleep-diagnostic";

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    match arguments.next().as_deref() {
        None | Some("help" | "--help" | "-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("firmware-build") => run_cargo(&firmware_args("build", RELEASE_FIRMWARE)),
        Some("firmware-flash") => run_cargo(&firmware_args("run", RELEASE_FIRMWARE)),
        Some("sleep-diagnostic-build") => run_cargo(&firmware_args("build", SLEEP_DIAGNOSTIC)),
        Some("sleep-diagnostic-flash") => run_cargo(&firmware_args("run", SLEEP_DIAGNOSTIC)),
        Some("content-fetch") => {
            let cache_dir = arguments.next();
            if arguments.next().is_some() {
                return fail("content-fetch accepts at most one cache directory");
            }
            task_result(content::fetch_command(cache_dir.as_deref()))
        }
        Some("content-build") => {
            let arguments: Vec<_> = arguments.collect();
            task_result(content::build_command(&arguments))
        }
        Some("render-samples") => {
            let output_dir = arguments.next();
            if arguments.next().is_some() {
                return fail("render-samples accepts at most one output directory");
            }
            task_result(render::samples_command(output_dir.as_deref()))
        }
        Some("render-contact-sheet") => {
            let output_file = arguments.next();
            if arguments.next().is_some() {
                return fail("render-contact-sheet accepts at most one output file");
            }
            task_result(render::contact_sheet_command(output_file.as_deref()))
        }
        Some(command) => {
            eprintln!("unknown xtask command: {command}");
            ExitCode::from(2)
        }
    }
}

fn task_result(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(&error),
    }
}

fn fail(error: &str) -> ExitCode {
    eprintln!("{error}");
    ExitCode::FAILURE
}

fn firmware_args(action: &'static str, binary: &'static str) -> [&'static str; 9] {
    [
        ESP_TOOLCHAIN_ARG,
        action,
        "--package",
        "pokeviewer-firmware",
        "--bin",
        binary,
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
    sleep-diagnostic-build
                      Build RTC wake/deep-sleep diagnostic firmware
    sleep-diagnostic-flash
                      Build, flash, and monitor sleep diagnostic firmware
    content-fetch [CACHE_DIR]
                      Explicitly fetch IDs 1-151 into a new review cache
    content-build [CACHE_DIR] [PACK_FILE] [MANIFEST_FILE]
                      Build a deterministic pack without network access
    render-samples [OUTPUT_DIR]
                      Render representative panel-native PBM and PNG evidence
    render-contact-sheet [OUTPUT_FILE]
                      Render all 151 cards into one actual-pixel PNG
    help              Print this help"
    );
}

#[cfg(test)]
mod tests {
    use super::{ESP_TARGET, RELEASE_FIRMWARE, SLEEP_DIAGNOSTIC, firmware_args};

    #[test]
    fn firmware_commands_select_the_embedded_target() {
        let args = firmware_args("build", RELEASE_FIRMWARE);

        assert_eq!(args[1], "build");
        assert!(args.contains(&ESP_TARGET));
        assert!(args.contains(&"--locked"));
    }

    #[test]
    fn sleep_diagnostic_selects_its_own_binary() {
        let args = firmware_args("run", SLEEP_DIAGNOSTIC);

        assert_eq!(args[1], "run");
        assert!(args.contains(&SLEEP_DIAGNOSTIC));
    }
}
