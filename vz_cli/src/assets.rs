//! `vz assets`: writes the cards' stylesheet and script, the code colours and the review marks' stylesheet and
//! script into the book's docs folder, at their default paths. `vz refs` writes the first two itself, for a book
//! that turns on the glossary or the code links; this command writes every asset the kit ships, unconditionally.

use crate::book::Book;
use crate::registry::Registered;
use crate::written::{Did, report, write_checked};
use clap::{ArgMatches, Command, FromArgMatches};
use std::error::Error;
use std::path::PathBuf;
use vc_ui::{Exit, Paint};

#[derive(clap::Args)]
struct Args {
    /// The project whose zensical.toml names the docs folder; the files are written under it
    #[arg(long, default_value = ".", value_hint = clap::ValueHint::DirPath)]
    root: PathBuf,
    /// Write nothing; fail when a file is stale
    #[arg(long)]
    check: bool,
}

pub(crate) fn command() -> Command {
    <Args as clap::Args>::augment_args(
        Command::new("assets")
            .about(
                "Write the cards' and review marks' stylesheets and scripts and the code colours",
            )
            .display_order(30),
    )
    .after_help("Examples:\n  vz assets\n  vz assets --check")
}

pub(crate) fn run(matches: &ArgMatches) -> Result<Exit, Box<dyn Error>> {
    let args = Args::from_arg_matches(matches)?;
    let book = Book::read(&args.root)?;
    let paint = Paint::stdout();
    let mut stale = false;
    for asset in vz_refs::assets::ALL {
        let w = write_checked(
            &book.docs(asset.path),
            asset.text,
            asset.what.to_owned(),
            args.check,
        )?;
        stale |= w.did == Did::Stale;
        report(paint, &w);
    }
    Ok(if args.check && stale {
        Exit::FAILURE
    } else {
        Exit::SUCCESS
    })
}

inventory::submit! { Registered { command, run } }
