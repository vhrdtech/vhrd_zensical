//! `vz`: the book kit's command. Everything is in the library, which the Python console script runs too.

use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(vz_cli::run(std::env::args_os()))
}
