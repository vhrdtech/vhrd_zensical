//! `vz prep`: everything a book's `zensical.toml` settings ask for, in one step: the glossary check and code
//! index and the cards' files when the book turns them on (`vz refs`'s job), every asset the kit ships (`vz
//! assets`'s), and the footer build stamp when the book names a theme `custom_dir` (`vz stamp`'s). `vz build`
//! and `vz serve` run this first, always writing (never `--check`).

use crate::book::Book;
use crate::registry::Registered;
use crate::written::{Did, Written, report, write_checked};
use clap::{ArgMatches, Command, FromArgMatches};
use std::error::Error;
use std::path::Path;
use vc_ui::{Exit, Paint};

#[derive(clap::Args)]
struct Args {
    /// The project whose zensical.toml, Cargo.toml and code are read; the files are written under it
    #[arg(long, default_value = ".", value_hint = clap::ValueHint::DirPath)]
    root: std::path::PathBuf,
    /// The full commit hash code links open their lines at; default: `git rev-parse HEAD`
    #[arg(long, env = "GITHUB_SHA")]
    commit: Option<String>,
    /// The name the stamp shows, for a virtual workspace whose Cargo.toml has no [package].name
    #[arg(long)]
    name: Option<String>,
    /// Write nothing; fail when a file is stale
    #[arg(long)]
    check: bool,
}

pub(crate) fn command() -> Command {
    <Args as clap::Args>::augment_args(
        Command::new("prep")
            .about("Write everything the book's zensical.toml settings ask for")
            .display_order(40),
    )
    .after_help("Examples:\n  vz prep\n  vz prep --check")
}

pub(crate) fn run(matches: &ArgMatches) -> Result<Exit, Box<dyn Error>> {
    let args = Args::from_arg_matches(matches)?;
    let written = prep(
        &args.root,
        args.commit.as_deref(),
        args.name.as_deref(),
        args.check,
    )?;
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

/// Writes (or, with `check`, compares) every file `root`'s `zensical.toml` settings ask for: the glossary check
/// and code index and the cards' files when it turns them on, every asset the kit ships, and the footer build
/// stamp when it names a theme `custom_dir`. `commit` and `name` follow `vz refs` and `vz stamp`.
///
/// # Errors
///
/// A setting has the wrong type, the glossary does not read, code links are on but the book sets no `repo_url`,
/// the workspace or the index cannot be built, or a file cannot be written.
pub(crate) fn prep(
    root: &Path,
    commit: Option<&str>,
    name: Option<&str>,
    check: bool,
) -> Result<Vec<Written>, Box<dyn Error>> {
    let book = Book::read(root)?;
    let settings = book.settings()?;
    let mut out = Vec::new();
    if let Some(g) = &settings.glossary {
        out.push(crate::refs::check_glossary(root, g)?);
    }
    if let Some(c) = &settings.code_refs {
        out.push(crate::refs::write_code_index(
            root, &book, c, commit, check,
        )?);
    }
    for asset in vz_refs::assets::ALL {
        out.push(write_checked(
            &book.docs(asset.path),
            asset.text,
            asset.what.to_owned(),
            check,
        )?);
    }
    if book.custom_dir.is_some() {
        out.extend(crate::stamp::write_stamp(root, &book, name, check)?);
    }
    Ok(out)
}

inventory::submit! { Registered { command, run } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_every_file_the_settings_name() {
        let dir = std::env::temp_dir().join(format!("vz_cli_prep_test_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[workspace]\n\n[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("src/lib.rs"), "/// Hello.\npub fn hello() {}\n").unwrap();
        std::fs::write(
            dir.join("docs/glossary.toml"),
            "[[term]]\nname = \"piece\"\nshort = \"One file.\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("zensical.toml"),
            "[project]\nrepo_url = \"https://github.com/o/demo\"\n[project.theme]\ncustom_dir = \"overrides\"\n[project.markdown_extensions]\n\"vhrd_zensical.glossary\" = {}\n\"vhrd_zensical.code_refs\" = {}\n",
        )
        .unwrap();
        let written = prep(&dir, Some("0123456789abcdef"), None, false).unwrap();
        let paths: Vec<String> = written
            .iter()
            .map(|w| w.path.strip_prefix(&dir).unwrap().display().to_string())
            .collect();
        assert_eq!(
            paths,
            [
                "docs/glossary.toml",
                "target/vz/code_index.json",
                "docs/stylesheets/vz-refs.css",
                "docs/javascripts/vz-refs.js",
                "docs/stylesheets/vz-code.css",
                "overrides/partials/vz-stamp.html",
                "docs/javascripts/vz-stamp-age.js",
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn writes_only_assets_for_a_minimal_book() {
        let dir =
            std::env::temp_dir().join(format!("vz_cli_prep_test_minimal_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("zensical.toml"), "[project]\nsite_name = \"x\"\n").unwrap();
        let written = prep(&dir, None, None, false).unwrap();
        let paths: Vec<String> = written
            .iter()
            .map(|w| w.path.strip_prefix(&dir).unwrap().display().to_string())
            .collect();
        assert_eq!(
            paths,
            [
                "docs/stylesheets/vz-refs.css",
                "docs/javascripts/vz-refs.js",
                "docs/stylesheets/vz-code.css",
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
