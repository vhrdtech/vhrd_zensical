//! `vz stamp`: writes the book's footer build stamp (crate version, git commit, build time) and the script that
//! shows the build's age from the reader's own clock.

use crate::book::Book;
use crate::registry::Registered;
use crate::written::{Did, Written, report, write_checked};
use clap::{ArgMatches, Command, FromArgMatches};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use vc_ui::{Exit, Paint};

#[derive(clap::Args)]
struct Args {
    /// The project whose Cargo.toml, git state and zensical.toml are read; the files are written under it
    #[arg(long, default_value = ".", value_hint = clap::ValueHint::DirPath)]
    root: PathBuf,
    /// The name the stamp shows, for a virtual workspace whose Cargo.toml has no [package].name; its version is
    /// the workspace member's with that name, else [workspace.package].version
    #[arg(long)]
    name: Option<String>,
    /// Write nothing; fail when a file is stale
    #[arg(long)]
    check: bool,
}

pub(crate) fn command() -> Command {
    <Args as clap::Args>::augment_args(
        Command::new("stamp")
            .about("Write the footer build stamp and its age script")
            .display_order(20),
    )
    .after_help(
        "Examples:\n  vz stamp\n  vz stamp --check\n  vz stamp --name vhrd_zensical --root ../vhrd_zensical",
    )
}

/// Why `vz stamp` cannot run
#[derive(Debug)]
enum StampError {
    /// The package's name and version cannot be read
    Package(vz_stamp::Error),
    /// The book sets no `theme.custom_dir`: the stamp partial has nowhere to go
    NoCustomDir,
}

impl fmt::Display for StampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Package(e) => write!(f, "{e}"),
            Self::NoCustomDir => write!(
                f,
                "the book sets no theme.custom_dir: the stamp partial has nowhere to go"
            ),
        }
    }
}

impl Error for StampError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Package(e) => Some(e),
            Self::NoCustomDir => None,
        }
    }
}

pub(crate) fn run(matches: &ArgMatches) -> Result<Exit, Box<dyn Error>> {
    let args = Args::from_arg_matches(matches)?;
    let written = write(&args.root, args.name.as_deref(), args.check)?;
    let paint = Paint::stdout();
    let stale = written.iter().any(|w| w.did == Did::Stale);
    for w in &written {
        report(paint, w);
    }
    Ok(if args.check && stale {
        Exit::FAILURE
    } else {
        Exit::SUCCESS
    })
}

/// Writes (or, with `check`, compares) the stamp partial and the age script; an error when the book names no
/// theme `custom_dir`
fn write(root: &Path, name: Option<&str>, check: bool) -> Result<[Written; 2], Box<dyn Error>> {
    let book = Book::read(root)?;
    if book.custom_dir.is_none() {
        return Err(Box::new(StampError::NoCustomDir));
    }
    write_stamp(root, &book, name, check)
}

/// Writes (or, with `check`, compares) the stamp partial and the age script, given a book that names a theme
/// `custom_dir`
///
/// # Panics
///
/// `book.custom_dir` is `None`.
pub(crate) fn write_stamp(
    root: &Path,
    book: &Book,
    name: Option<&str>,
    check: bool,
) -> Result<[Written; 2], Box<dyn Error>> {
    let custom = book
        .custom(vz_core::stamp::DEFAULT_STAMP_PARTIAL)
        .expect("caller checked custom_dir");
    let (name, version) = vz_stamp::package(root, name).map_err(StampError::Package)?;
    let git = vc_build::collect(root);
    let s = vz_stamp::Stamp::now(name, version, git.sha);
    Ok([
        write_checked(
            &custom,
            &s.html(),
            format!("{} {}", s.name, s.version),
            check,
        )?,
        write_checked(
            &book.docs(vz_core::stamp::DEFAULT_AGE_SCRIPT),
            vz_stamp::AGE_SCRIPT,
            "the age script".to_owned(),
            check,
        )?,
    ])
}

inventory::submit! { Registered { command, run } }

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("vz_cli_stamp_test_{}_{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"fx\"\nversion = \"1.2.3\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("zensical.toml"),
            "[project]\n[project.theme]\ncustom_dir = \"overrides\"\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn writes_the_partial_and_the_script() {
        let dir = project("both");
        let written = write(&dir, None, false).unwrap();
        assert_eq!(
            written[0].path,
            dir.join("overrides/partials/vz-stamp.html")
        );
        assert_eq!(written[0].what, "fx 1.2.3");
        assert_eq!(
            written[1].path,
            dir.join("docs/javascripts/vz-stamp-age.js")
        );
        let html = std::fs::read_to_string(&written[0].path).unwrap();
        assert!(html.contains("fx 1.2.3"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_custom_dir_is_an_error() {
        let dir =
            std::env::temp_dir().join(format!("vz_cli_stamp_test_nocustom_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"fx\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("zensical.toml"), "[project]\n").unwrap();
        let e = write(&dir, None, false).unwrap_err();
        assert!(e.to_string().contains("custom_dir"), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
