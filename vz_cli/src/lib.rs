//! The `vz` command of the book kit, as a library.
//!
//! [`main`] is the whole command: the `vz` binary calls it, and so does the Python console script `vz` through
//! `vhrd_zensical._vz.main`, so both are the same build at the same version. Each command group registers itself
//! in its own module (the registry); [`command`], help and dispatch are built from those registrations.

mod assets;
mod book;
mod build;
mod check;
mod prep;
mod refs;
mod registry;
mod serve;
mod stamp;
mod written;

use clap::{Args, FromArgMatches};
use std::error::Error;
use std::ffi::OsString;
use vc_ui::{ColorFlag, Exit};

/// The tool's name, in front of its error line
const TOOL: &str = "vz";

/// The root command `vz` with every registered command under it, for parsing and for shell completion
pub fn command() -> clap::Command {
    // `about` after the flag: augmenting sets the flag struct's own doc as the command's about
    ColorFlag::augment_args(registry::root())
        .about("The Zensical book kit: code index, card and code assets, build stamp, frame check")
        .long_about(None)
}

/// Runs `vz` with `args` (the program name first, as `std::env::args_os` gives) and returns the exit code.
///
/// Help, `--version` and usage errors are clap's output; a command's error ends as one line on stderr,
/// `vz: <error>: <cause>` (`vc_ui::finish`).
pub fn main<I, T>(args: I) -> Exit
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    vc_ui::finish(TOOL, run(args), |_| None)
}

/// Parses `args` and runs the chosen command
fn run<I, T>(args: I) -> Result<Exit, Box<dyn Error>>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = match command().try_get_matches_from(args) {
        Ok(m) => m,
        Err(e) => {
            // A closed stdout (`vz --help | head -0`) is not an error of the command.
            e.print().ok();
            return Ok(if e.use_stderr() {
                Exit::BAD_ARGS
            } else {
                Exit::SUCCESS
            });
        }
    };
    ColorFlag::from_arg_matches(&matches)?.apply();
    registry::run(&matches)
}

#[cfg(test)]
mod tests {
    use vc_ui::Exit;

    #[test]
    fn version_and_help_succeed() {
        assert_eq!(super::main(["vz", "--version"]), Exit::SUCCESS);
        assert_eq!(super::main(["vz", "--help"]), Exit::SUCCESS);
    }

    #[test]
    fn an_unknown_command_is_a_usage_error() {
        assert_eq!(super::main(["vz", "no-such-command"]), Exit::BAD_ARGS);
    }

    #[test]
    fn the_command_is_consistent() {
        super::command().debug_assert();
    }
}
