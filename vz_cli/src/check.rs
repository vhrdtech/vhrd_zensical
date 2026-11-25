//! `vz check`: the book's frame against its `book_checks.toml` ([`vz_core::frame`]): the nav in level order, a
//! level on every page, no page twice, no bare `rust` fence beyond the waived counts, no stale generated output.
//! Pages are read from the docs folder, so a book that copies pages from other repositories assembles them first;
//! a page its sources file lists is a copy.

use crate::book::{self, Book};
use crate::registry::Registered;
use crate::written::{Did, Written, report};
use clap::{ArgMatches, Command, FromArgMatches};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use vc_ui::{Exit, Paint};
use vz_core::frame::{self, Checks, ChecksError, Nav, NavError, Page, Problem};
use vz_core::sources::{Sources, SourcesError};
use vz_refs::read::FileError;

/// Files under the docs folder's generated folder at most
const GENERATED_FILES_MAX: usize = 16_384;

#[derive(clap::Args)]
struct Args {
    /// The project whose zensical.toml, frame file and pages are read
    #[arg(long, default_value = ".", value_hint = clap::ValueHint::DirPath)]
    root: PathBuf,
}

pub(crate) fn command() -> Command {
    <Args as clap::Args>::augment_args(
        Command::new("check")
            .about("Check the book's frame: nav level order, levels, bare rust fences, generated files")
            .display_order(45),
    )
    .after_help("Examples:\n  vz check\n  vz check --root ../book")
}

/// Why `vz check` cannot run, or that it found problems
#[derive(Debug)]
enum CheckError {
    /// `zensical.toml` does not read
    Book(book::ReadError),
    /// The frame file does not read
    Checks(FileError<ChecksError>),
    /// The book names no nav
    NoNav,
    /// The nav cannot be read in click order
    Nav(NavError),
    /// The kit's extension settings do not read
    Settings(vz_core::ext::SettingsError),
    /// The sources file does not read
    Sources(FileError<SourcesError>),
    /// The generated folder cannot be listed
    Generated {
        /// The folder
        dir: PathBuf,
        /// Why
        source: std::io::Error,
    },
    /// More than [`GENERATED_FILES_MAX`] generated files
    TooManyGenerated,
    /// The frame has problems, printed above
    Problems {
        /// How many
        count: usize,
    },
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Book(e) => write!(f, "{e}"),
            Self::Checks(e) => write!(f, "{e}"),
            Self::NoNav => write!(f, "{} names no nav: nothing to check", book::FILE),
            Self::Nav(e) => write!(f, "{}: {e}", book::FILE),
            Self::Settings(e) => write!(f, "{}: {e}", book::FILE),
            Self::Sources(e) => write!(f, "{e}"),
            Self::Generated { dir, .. } => write!(f, "listing {}", dir.display()),
            Self::TooManyGenerated => write!(
                f,
                "more than {GENERATED_FILES_MAX} files under the generated folder"
            ),
            Self::Problems { count } => write!(f, "{count} problems in the book's frame"),
        }
    }
}

impl Error for CheckError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Book(e) => Some(e),
            Self::Checks(e) => Some(e),
            Self::Nav(e) => Some(e),
            Self::Settings(e) => Some(e),
            Self::Sources(e) => Some(e),
            Self::Generated { source, .. } => Some(source),
            Self::NoNav | Self::TooManyGenerated | Self::Problems { .. } => None,
        }
    }
}

pub(crate) fn run(matches: &ArgMatches) -> Result<Exit, Box<dyn Error>> {
    let args = Args::from_arg_matches(matches)?;
    let (checked, problems) = check(&args.root)?;
    if !problems.is_empty() {
        print_problems(Paint::stdout(), &problems);
        return Err(Box::new(CheckError::Problems {
            count: problems.len(),
        }));
    }
    report(Paint::stdout(), &checked);
    Ok(Exit::SUCCESS)
}

/// Checks the frame of the book at `root`: what was checked, for the report, and every problem found
fn check(root: &Path) -> Result<(Written, Vec<Problem>), CheckError> {
    let book = Book::read(root).map_err(CheckError::Book)?;
    let path = root.join(frame::FILE);
    let checks =
        vz_refs::read::format(&path, frame::FILE, Checks::parse).map_err(CheckError::Checks)?;
    if book.nav.is_empty() {
        return Err(CheckError::NoNav);
    }
    let nav = Nav::from_toml(&book.nav).map_err(CheckError::Nav)?;
    let sources = sources(root, &book)?;
    let generated = generated(&book)?;
    let read = |page: &str| read_page(&book, sources.as_ref(), page);
    let problems = frame::check(&checks, &nav, read, &generated);
    let checked = Written {
        did: Did::Checked,
        path,
        what: format!(
            "{} pages in {} parts and {} shelves",
            nav.pages.len(),
            checks.part.len(),
            checks.outside.len()
        ),
    };
    Ok((checked, problems))
}

/// The sources file the code link extension names, when it does
fn sources(root: &Path, book: &Book) -> Result<Option<Sources>, CheckError> {
    let settings = book.settings().map_err(CheckError::Settings)?;
    let Some(file) = settings.code_refs.and_then(|c| c.sources) else {
        return Ok(None);
    };
    vz_refs::sources::read(&root.join(&file), &file)
        .map(Some)
        .map_err(CheckError::Sources)
}

/// A page of the nav from the docs folder: `Ok(None)` when it is not there
fn read_page(book: &Book, sources: Option<&Sources>, page: &str) -> Result<Option<Page>, String> {
    match vz_refs::read::text(&book.docs(page), vz_core::FILE_BYTES_MAX as u64) {
        Ok(text) => Ok(Some(Page {
            text,
            copied: sources.is_some_and(|s| s.source_of(page).is_some()),
        })),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// The files under the docs folder's generated folder, as a page includes them: from the project root, `/`
/// separators, sorted; none when there is no such folder
fn generated(book: &Book) -> Result<Vec<String>, CheckError> {
    let docs = book.docs_dir.trim_start_matches("./").trim_end_matches('/');
    let top = format!("{docs}/{}", frame::GENERATED_DIR);
    let mut files = Vec::new();
    let mut dirs = vec![top];
    while let Some(dir) = dirs.pop() {
        let path = book.root.join(&dir);
        let entries = match std::fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => return Err(CheckError::Generated { dir: path, source }),
        };
        for entry in entries {
            let err = |source| CheckError::Generated {
                dir: path.clone(),
                source,
            };
            let entry = entry.map_err(err)?;
            let rel = format!("{dir}/{}", entry.file_name().to_string_lossy());
            if entry.file_type().map_err(err)?.is_dir() {
                dirs.push(rel);
            } else {
                files.push(rel);
            }
            if files.len() + dirs.len() > GENERATED_FILES_MAX {
                return Err(CheckError::TooManyGenerated);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Prints each problem on its own line; the one place `vz check` prints them
#[expect(
    clippy::print_stdout,
    reason = "the problems are the command's own report, printed in this one place"
)]
fn print_problems(paint: Paint, problems: &[Problem]) {
    let word = paint.err("problem");
    for p in problems {
        // spirit-level: allow(print-in-library) vz_cli carries the vz binary's whole logic (AGENTS.md section 5);
        // this is a command's own report, printed in this one place
        println!("{word} {p}");
    }
}

inventory::submit! { Registered { command, run } }

#[cfg(test)]
mod tests {
    use super::*;

    fn book(name: &str, nav: &str, docs_dir: Option<&str>) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("vz_cli_check_test_{}_{name}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let docs = docs_dir.unwrap_or("docs");
        std::fs::create_dir_all(dir.join(docs)).unwrap();
        let docs_line = docs_dir.map_or(String::new(), |d| format!("docs_dir = \"{d}\"\n"));
        std::fs::write(
            dir.join("zensical.toml"),
            format!("[project]\n{docs_line}nav = {nav}\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join(frame::FILE),
            "outside = []\n[[part]]\ntitle = \"Basic\"\nlevel = \"basic\"\n",
        )
        .unwrap();
        dir
    }

    fn page(dir: &Path, docs: &str, name: &str, body: &str) {
        let path = dir.join(docs).join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("---\nlevel: basic\n---\n\n{body}")).unwrap();
    }

    #[test]
    fn generated_files_are_named_from_the_root_under_the_docs_folder() {
        let dir = book("generated", r#"[{ "Basic" = ["a.md"] }]"#, Some("./pages/"));
        page(
            &dir,
            "pages",
            "a.md",
            "--8<-- \"pages/generated/x/used.txt\"\n",
        );
        std::fs::create_dir_all(dir.join("pages/generated/x")).unwrap();
        std::fs::write(dir.join("pages/generated/x/used.txt"), "1\n").unwrap();
        std::fs::write(dir.join("pages/generated/orphan.txt"), "2\n").unwrap();
        let (_, problems) = check(&dir).unwrap();
        assert_eq!(
            problems.iter().map(Problem::to_string).collect::<Vec<_>>(),
            ["pages/generated/orphan.txt: no page in the nav includes it"]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_page_the_sources_file_lists_is_a_copy() {
        let dir = book("copied", r#"[{ "Basic" = ["lib/a.md"] }]"#, None);
        let zensical = std::fs::read_to_string(dir.join("zensical.toml")).unwrap();
        std::fs::write(
            dir.join("zensical.toml"),
            format!("{zensical}[project.markdown_extensions]\n\"vhrd_zensical.code_refs\" = {{ sources = \"sources.toml\" }}\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join("sources.toml"),
            "format = 1\n[[source]]\nname = \"lib\"\nroot = \"../lib\"\nrepository = \"https://github.com/o/lib\"\ncommit = \"0123456789abcdef0123456789abcdef01234567\"\npages = [\"lib/a.md\"]\n",
        )
        .unwrap();
        std::fs::write(
            dir.join(frame::FILE),
            "outside = []\n[[part]]\ntitle = \"Basic\"\nlevel = \"basic\"\n[bare_fences]\n\"lib/a.md\" = 3\n",
        )
        .unwrap();
        page(&dir, "docs", "lib/a.md", "```rust\nfn a() {}\n```\n");
        let (checked, problems) = check(&dir).unwrap();
        assert_eq!(problems, []);
        assert_eq!(checked.what, "1 pages in 1 parts and 0 shelves");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_book_without_a_nav_or_frame_file_is_an_error() {
        let dir = book("nonav", "[]", None);
        assert!(matches!(check(&dir), Err(CheckError::NoNav)));
        std::fs::remove_file(dir.join(frame::FILE)).unwrap();
        let e = check(&dir).unwrap_err().to_string();
        assert!(e.starts_with("book_checks.toml: "), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
