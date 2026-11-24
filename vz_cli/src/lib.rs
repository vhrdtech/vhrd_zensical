//! The `vz` command of the book kit, as a library.
//!
//! [`run`] is the whole command: the `vz` binary calls it, and so does the Python console script `vz` through
//! `vhrd_zensical._vz.main`, so both are the same build at the same version. Each command group registers itself
//! in its own module ([`registry`]); the root command, help and dispatch are built from those registrations.

mod registry;

use std::ffi::OsString;

/// Exit status of a run that went wrong in the command line (clap's own code for usage errors)
const USAGE: u8 = 2;

/// Runs `vz` with `args` (the program name first, as `std::env::args_os` gives) and returns the exit status.
///
/// Help, `--version` and usage errors are printed by clap; a command's error is printed to stderr as `vz: <error>`.
pub fn run<I, T>(args: I) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = match registry::root().try_get_matches_from(args) {
        Ok(m) => m,
        Err(e) => return print_clap(&e),
    };
    match registry::run(&matches) {
        Ok(code) => code,
        Err(e) => {
            #[expect(
                clippy::print_stderr,
                reason = "the command's one place that reports an error, for the binary and the console script"
            )]
            {
                eprintln!("vz: {e}");
            }
            1
        }
    }
}

/// Prints clap's help, version or usage error and gives its exit status
fn print_clap(e: &clap::Error) -> u8 {
    // A closed stdout (`vz --help | head -0`) is not an error of the command.
    e.print().ok();
    if e.use_stderr() { USAGE } else { 0 }
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_and_help_succeed() {
        assert_eq!(super::run(["vz", "--version"]), 0);
        assert_eq!(super::run(["vz", "--help"]), 0);
    }

    #[test]
    fn an_unknown_command_is_a_usage_error() {
        assert_eq!(super::run(["vz", "no-such-command"]), super::USAGE);
    }
}
