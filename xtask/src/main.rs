#![forbid(unsafe_code)]
#![doc = "Cargo-native repository automation."]

use std::env;

fn main() {
    match env::args().nth(1).as_deref() {
        None | Some("help" | "--help" | "-h") => print_help(),
        Some(command) => {
            eprintln!("unknown xtask command: {command}");
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!(
        "\
Pokeviewer repository tasks

USAGE:
    cargo run -p xtask -- <COMMAND>

COMMANDS:
    help    Print this help"
    );
}
