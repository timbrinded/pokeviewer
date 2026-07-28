#![forbid(unsafe_code)]
#![doc = "Linux USB provisioning and diagnostics command-line entry point."]

use std::{env, process::ExitCode};

fn main() -> ExitCode {
    match pokeviewerctl::run(env::args().skip(1)) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("pokeviewerctl: {error}");
            ExitCode::from(2)
        }
    }
}
