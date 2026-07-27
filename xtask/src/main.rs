#![forbid(unsafe_code)]
#![doc = "Cargo-native repository automation."]

use std::env;
use std::process::{Command, ExitCode};

mod content;
mod golden;
mod qualification;
mod render;

const ESP_TOOLCHAIN: &str = "esp-1.95.0.0";
const ESP_TOOLCHAIN_ARG: &str = "+esp-1.95.0.0";
const ESP_TARGET: &str = "xtensa-esp32s3-none-elf";
const RELEASE_FIRMWARE: &str = "pokeviewer-firmware";
const SLEEP_DIAGNOSTIC: &str = "pokeviewer-sleep-diagnostic";
const USB_PROVISIONING: &str = "pokeviewer-usb-provisioning";

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
        Some("usb-provisioning-build") => run_cargo(&firmware_args("build", USB_PROVISIONING)),
        Some("usb-provisioning-flash") => run_cargo(&firmware_args("run", USB_PROVISIONING)),
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
        Some("render-setup-screen") => {
            let output_file = arguments.next();
            if arguments.next().is_some() {
                return fail("render-setup-screen accepts at most one output file");
            }
            task_result(render::setup_screen_command(output_file.as_deref()))
        }
        Some("render-recovery-screens") => {
            let output_dir = arguments.next();
            if arguments.next().is_some() {
                return fail("render-recovery-screens accepts at most one output directory");
            }
            task_result(render::recovery_screens_command(output_dir.as_deref()))
        }
        Some("qualification-schedule") => {
            let Some(start) = arguments.next() else {
                return fail("qualification-schedule requires START_DATE");
            };
            let output = arguments.next();
            if arguments.next().is_some() {
                return fail("qualification-schedule accepts START_DATE and optional OUTPUT");
            }
            task_result(qualification::schedule_command(&start, output.as_deref()))
        }
        Some("golden-update") => {
            if arguments.next().is_some() {
                return fail("golden-update accepts no arguments");
            }
            task_result(golden::update_command())
        }
        Some("golden-check") => {
            let diff_dir = arguments.next();
            if arguments.next().is_some() {
                return fail("golden-check accepts at most one diff directory");
            }
            task_result(golden::check_command(diff_dir.as_deref()))
        }
        Some("golden-demo-failure") => {
            let output_dir = arguments.next();
            if arguments.next().is_some() {
                return fail("golden-demo-failure accepts at most one output directory");
            }
            task_result(golden::demo_failure_command(output_dir.as_deref()))
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
    usb-provisioning-build
                      Build the bounded wired RTC provisioning firmware
    usb-provisioning-flash
                      Build, flash, and monitor wired provisioning firmware
    content-fetch [CACHE_DIR]
                      Explicitly fetch IDs 1-151 into a new review cache
    content-build [CACHE_DIR] [PACK_FILE] [MANIFEST_FILE]
                      Build a deterministic pack without network access
    render-samples [OUTPUT_DIR]
                      Render representative panel-native PBM and PNG evidence
    render-contact-sheet [OUTPUT_FILE]
                      Render all 151 cards into one actual-pixel PNG
    render-setup-screen [OUTPUT_FILE]
                      Render the adult invalid-RTC recovery screen
    render-recovery-screens [OUTPUT_DIR]
                      Render every classified adult recovery screen
    qualification-schedule START_DATE [OUTPUT]
                      Write seven expected daily transitions and frame hashes
    golden-update     Explicitly regenerate reviewed raw and PNG goldens
    golden-check [DIFF_DIR]
                      Compare exact frames and emit failure artifacts
    golden-demo-failure [OUTPUT_DIR]
                      Capture a deterministic one-pixel failure example
    help              Print this help"
    );
}

#[cfg(test)]
mod tests {
    use super::{ESP_TARGET, RELEASE_FIRMWARE, SLEEP_DIAGNOSTIC, USB_PROVISIONING, firmware_args};

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

    #[test]
    fn usb_provisioning_selects_its_own_binary() {
        let args = firmware_args("build", USB_PROVISIONING);

        assert_eq!(args[1], "build");
        assert!(args.contains(&USB_PROVISIONING));
    }
}
