//! `vz refs`: checks the glossary, writes the code index and the cards' stylesheet and script that the book's
//! Markdown extensions read. Run before every build (`vz prep` does); never belongs in git.

use crate::book::Book;
use crate::registry::Registered;
use crate::written::{Did, Written, report, write_checked};
use clap::{ArgMatches, Command, FromArgMatches};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use vc_ui::{Exit, Paint};
use vz_core::ext::{CODE_REFS_EXTENSION, CodeRefsSettings, GLOSSARY_EXTENSION, GlossarySettings};

#[derive(clap::Args)]
struct Args {
    /// The project whose zensical.toml and code are read; the files are written under it
    #[arg(long, default_value = ".", value_hint = clap::ValueHint::DirPath)]
    root: PathBuf,
    /// The full commit hash code links open their lines at; default: `git rev-parse HEAD`
    #[arg(long, env = "GITHUB_SHA")]
    commit: Option<String>,
    /// Write nothing; fail when a file is stale
    #[arg(long)]
    check: bool,
}

pub(crate) fn command() -> Command {
    <Args as clap::Args>::augment_args(
        Command::new("refs")
            .about("Check the glossary and write the code index and the cards' files")
            .display_order(10),
    )
    .after_help("Examples:\n  vz refs\n  vz refs --check")
}

/// Why `vz refs` cannot run
#[derive(Debug)]
enum RefsError {
    /// The book turns on neither the glossary nor the code link extension
    NotOn,
    /// The glossary does not read
    Glossary {
        file: String,
        source: vz_core::glossary::GlossaryError,
    },
    /// Code links are on but the book sets no `repo_url`
    NoRepository,
    /// The commit cannot be found
    Commit(String),
    /// The workspace cannot be loaded
    Workspace(vz_refs::workspace::LoadError),
    /// The index cannot be built
    Index(vz_refs::code_index::IndexError),
    /// The index cannot be written as JSON
    Json(serde_json::Error),
}

impl fmt::Display for RefsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotOn => write!(
                f,
                "the book turns on neither {GLOSSARY_EXTENSION} nor {CODE_REFS_EXTENSION} in markdown_extensions"
            ),
            Self::Glossary { file, source } => write!(f, "{file}: {source}"),
            Self::NoRepository => write!(
                f,
                "{CODE_REFS_EXTENSION} is on but the book sets no repo_url: code links open their lines there"
            ),
            Self::Commit(e) => write!(
                f,
                "`git rev-parse HEAD` names no commit for the code links ({e}): pass --commit"
            ),
            Self::Workspace(e) => write!(f, "{e}"),
            Self::Index(e) => write!(f, "the code index: {e}"),
            Self::Json(e) => write!(f, "the code index as JSON: {e}"),
        }
    }
}

impl Error for RefsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Workspace(e) => Some(e),
            Self::Index(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::NotOn | Self::Glossary { .. } | Self::NoRepository | Self::Commit(_) => None,
        }
    }
}

pub(crate) fn run(matches: &ArgMatches) -> Result<Exit, Box<dyn Error>> {
    let args = Args::from_arg_matches(matches)?;
    let written = write(&args.root, args.commit.as_deref(), args.check)?;
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

/// Checks the glossary and writes the code index and the cards' files the book turns on
fn write(root: &Path, commit: Option<&str>, check: bool) -> Result<Vec<Written>, Box<dyn Error>> {
    let book = Book::read(root)?;
    let settings = book.settings()?;
    if settings.glossary.is_none() && settings.code_refs.is_none() {
        return Err(Box::new(RefsError::NotOn));
    }
    let mut out = Vec::new();
    if let Some(g) = &settings.glossary {
        out.push(check_glossary(root, g)?);
    }
    if let Some(c) = &settings.code_refs {
        out.push(write_code_index(root, &book, c, commit, check)?);
    }
    if settings.glossary.is_some() || settings.code_refs.is_some() {
        out.extend(write_cards(&book, check)?);
    }
    Ok(out)
}

/// Reads and parses the glossary at `g.file`, which the build would refuse if it did not read
pub(crate) fn check_glossary(root: &Path, g: &GlossarySettings) -> Result<Written, Box<dyn Error>> {
    let text = std::fs::read_to_string(root.join(&g.file))?;
    let glossary =
        vz_core::glossary::Glossary::parse(&text).map_err(|source| RefsError::Glossary {
            file: g.file.clone(),
            source,
        })?;
    Ok(Written {
        did: Did::Checked,
        path: root.join(&g.file),
        what: format!("{} terms", glossary.terms.len()),
    })
}

/// Builds the workspace's code index at `commit` (`None`: `git rev-parse HEAD`) and writes it to `c.index`
pub(crate) fn write_code_index(
    root: &Path,
    book: &Book,
    c: &CodeRefsSettings,
    commit: Option<&str>,
    check: bool,
) -> Result<Written, Box<dyn Error>> {
    let repository = book.repo_url.as_deref().ok_or(RefsError::NoRepository)?;
    let workspace = vz_refs::workspace::Workspace::load(root).map_err(RefsError::Workspace)?;
    let commit = match commit {
        Some(c) if !c.is_empty() => c.to_owned(),
        _ => workspace
            .head()
            .map_err(|e| RefsError::Commit(e.to_string()))?,
    };
    let forge = vz_refs::code_index::Forge {
        repository,
        commit: &commit,
    };
    let index = vz_refs::code_index::index(&workspace, c, &[book.site_dir.as_str()], forge)
        .map_err(RefsError::Index)?;
    let mut json = serde_json::to_string_pretty(&index).map_err(RefsError::Json)?;
    json.push('\n');
    write_checked(
        &root.join(&c.index),
        &json,
        format!("{} items at {}", index.items.len(), short(&commit)),
        check,
    )
    .map_err(Into::into)
}

/// Writes the cards' stylesheet and script into the book's docs folder
pub(crate) fn write_cards(book: &Book, check: bool) -> Result<[Written; 2], Box<dyn Error>> {
    Ok([
        write_checked(
            &book.docs(vz_refs::assets::CARD_CSS.path),
            vz_refs::assets::CARD_CSS.text,
            vz_refs::assets::CARD_CSS.what.to_owned(),
            check,
        )?,
        write_checked(
            &book.docs(vz_refs::assets::CARD_SCRIPT.path),
            vz_refs::assets::CARD_SCRIPT.text,
            vz_refs::assets::CARD_SCRIPT.what.to_owned(),
            check,
        )?,
    ])
}

/// A commit's first 12 characters
fn short(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}

inventory::submit! { Registered { command, run } }

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str, zensical: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("vz_cli_refs_test_{}_{name}", std::process::id()));
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
        std::fs::write(dir.join("zensical.toml"), zensical).unwrap();
        dir
    }

    #[test]
    fn writes_the_index_and_the_cards() {
        let dir = project(
            "both",
            "[project]\nrepo_url = \"https://github.com/o/demo\"\n[project.markdown_extensions]\n\"vhrd_zensical.glossary\" = {}\n\"vhrd_zensical.code_refs\" = {}\n",
        );
        let written = write(&dir, Some("0123456789abcdef"), false).unwrap();
        let paths: Vec<String> = written
            .iter()
            .map(|w| w.path.display().to_string())
            .collect();
        assert_eq!(
            paths,
            [
                dir.join("docs/glossary.toml").display().to_string(),
                dir.join("target/vz/code_index.json").display().to_string(),
                dir.join("docs/stylesheets/vz-refs.css")
                    .display()
                    .to_string(),
                dir.join("docs/javascripts/vz-refs.js")
                    .display()
                    .to_string(),
            ]
        );
        assert_eq!(written[1].what, "1 items at 0123456789ab");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn code_links_need_a_repository() {
        let dir = project(
            "norepo",
            "[project]\n[project.markdown_extensions]\n\"vhrd_zensical.code_refs\" = {}\n",
        );
        let e = write(&dir, Some("abc"), false).unwrap_err();
        assert!(e.to_string().contains("repo_url"), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_book_without_the_extensions_is_an_error() {
        let dir = project("off", "[project]\nsite_name = \"x\"\n");
        let e = write(&dir, None, false).unwrap_err();
        assert!(e.to_string().contains("turns on neither"), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
