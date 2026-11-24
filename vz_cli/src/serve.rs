//! `vz serve`: `vz prep`, then `zensical serve` from the same environment, so a book's dev-server recipe is one
//! line.

use crate::registry::Registered;
use clap::{Arg, ArgMatches, Command, FromArgMatches};
use std::error::Error;
use vc_ui::{Exit, Paint};

#[derive(clap::Args)]
struct Args {
    /// The project whose zensical.toml, Cargo.toml and code are read, and where Zensical serves the site from
    #[arg(long, default_value = ".", value_hint = clap::ValueHint::DirPath)]
    root: std::path::PathBuf,
    /// The full commit hash code links open their lines at; default: `git rev-parse HEAD`
    #[arg(long, env = "GITHUB_SHA")]
    commit: Option<String>,
    /// The name the stamp shows, for a virtual workspace whose Cargo.toml has no [package].name
    #[arg(long)]
    name: Option<String>,
}

pub(crate) fn command() -> Command {
    <Args as clap::Args>::augment_args(
        Command::new("serve")
            .about("vz prep, then zensical serve")
            .display_order(60),
    )
    .arg(
        Arg::new("extra")
            .help("Extra arguments passed on to zensical serve")
            .trailing_var_arg(true)
            .num_args(0..)
            .allow_hyphen_values(true),
    )
    .after_help("Examples:\n  vz serve\n  vz serve -- --dev-addr 0.0.0.0:8000")
}

pub(crate) fn run(matches: &ArgMatches) -> Result<Exit, Box<dyn Error>> {
    let args = Args::from_arg_matches(matches)?;
    let extra: Vec<&String> = matches
        .get_many::<String>("extra")
        .into_iter()
        .flatten()
        .collect();
    for w in crate::prep::prep(
        &args.root,
        args.commit.as_deref(),
        args.name.as_deref(),
        false,
    )? {
        crate::written::report(Paint::stdout(), &w);
    }
    let status = std::process::Command::new("zensical")
        .arg("serve")
        .args(&extra)
        .current_dir(&args.root)
        .status()
        .map_err(|e| format!("cannot run zensical: {e}"))?;
    Ok(if status.success() {
        Exit::SUCCESS
    } else {
        Exit::FAILURE
    })
}

inventory::submit! { Registered { command, run } }
