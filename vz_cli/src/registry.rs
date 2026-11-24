//! The commands of `vz`: each module registers its own with `inventory::submit!`. The root command and the run of
//! a command are built from these registrations, so adding a command never edits a central list.

use clap::{ArgMatches, Command};
use std::error::Error;
use vc_ui::Exit;

/// A command group, as its module registers it
pub(crate) struct Registered {
    /// Builds the clap command: its name, help, arguments and place in the help (`display_order`)
    command: fn() -> Command,
    /// Runs the command from its own matches and gives the exit code
    run: fn(&ArgMatches) -> Result<Exit, Box<dyn Error>>,
}

inventory::collect!(Registered);

/// Every registered command, built, in help order
fn commands() -> Vec<(Command, &'static Registered)> {
    let mut all: Vec<(Command, &Registered)> = inventory::iter::<Registered>
        .into_iter()
        .map(|r| ((r.command)(), r))
        .collect();
    all.sort_by_key(|(c, _)| (c.get_display_order(), c.get_name().to_owned()));
    all
}

/// The root command `vz` with every registered command under it
pub(crate) fn root() -> Command {
    Command::new("vz")
        .version(vc_build::version!())
        .after_help("Examples:\n  vz --version\n  vz --help")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommands(commands().into_iter().map(|(c, _)| c))
}

/// Runs the command `matches` chose
pub(crate) fn run(matches: &ArgMatches) -> Result<Exit, Box<dyn Error>> {
    let (name, sub) = matches
        .subcommand()
        .ok_or("no command given (`vz --help` lists them)")?;
    let (_, r) = commands()
        .into_iter()
        .find(|(c, _)| c.get_name() == name)
        .ok_or_else(|| format!("no command {name} (`vz --help` lists them)"))?;
    (r.run)(sub)
}

#[cfg(test)]
mod tests {
    #[test]
    fn registered_commands_are_the_expected_ones() {
        let names: Vec<String> = super::commands()
            .iter()
            .map(|(c, _)| c.get_name().to_owned())
            .collect();
        assert_eq!(names, [] as [&str; 0]);
    }

    #[test]
    fn version_is_the_cargo_version() {
        let version = super::root().render_version();
        assert!(
            version.starts_with(&format!("vz {} (", env!("CARGO_PKG_VERSION"))),
            "{version}"
        );
    }
}
