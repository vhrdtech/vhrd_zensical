//! The sources file: the other repositories a book copies pages from, so their code links resolve.
//!
//! A book can assemble pages from other repositories' checkouts into its docs folder. A code link on such a page
//! names an item of that repository, which the book's own workspace does not have. The book's assembler writes this
//! file at every build (never kept in git); the code link extension's `sources` setting names it. `vz refs` indexes
//! each source's workspace beside the book's own, and the extension resolves a link on a copied page against its
//! source first, then against the book's own crates.
//!
//! ```toml
//! format = 1
//!
//! [[source]]
//! name = "ww_stdlib"                                    # unique; errors and the index name the source by it
//! root = "/home/u/.cache/ww_book/ww_stdlib"             # the checkout: its git root and cargo workspace root
//! repository = "https://github.com/vhrdtech/ww_stdlib"  # cards link its files at `commit`
//! commit = "0123456789abcdef0123456789abcdef01234567"   # the full hash checked out at `root`
//! crates = ["ww_logic_analyzer"]                        # library names; empty or left out: every library
//! on_crates_io = ["ww_logic_analyzer"]                  # package names published on crates.io: docs.rs links
//! pages = ["std_library/ww_logic_analyzer.md"]          # the copies, relative to the docs folder
//! ```
//!
//! `root` is absolute or relative to the project root. A page is a relative path with `/` separators and no `.` or
//! `..` segment, as Zensical names a page; it belongs to one source at most. Readers ignore keys they do not know;
//! `tests/fixtures/sources/ok.toml` is the layout's snapshot.

use crate::FILE_BYTES_MAX;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The version of the file's layout; a file of another is refused
pub const FORMAT: u32 = 1;

/// Sources a file lists at most; each is a workspace `vz refs` loads and indexes
pub const SOURCES_MAX: usize = 64;

/// One repository whose pages the book copies
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// Its name, unique in the file
    pub name: String,
    /// Its checkout, absolute or relative to the project root: its git root and cargo workspace root
    pub root: String,
    /// The repository's address (`https://github.com/vhrdtech/ww_stdlib`)
    pub repository: String,
    /// The full hash of the commit checked out at [`Source::root`]
    pub commit: String,
    /// The library crates indexed, by library name; empty: every library of the workspace
    #[serde(default)]
    pub crates: Vec<String>,
    /// The packages published on crates.io, by package name: their cards link to docs.rs
    #[serde(default)]
    pub on_crates_io: Vec<String>,
    /// The pages copied from it, relative to the docs folder
    pub pages: Vec<String>,
}

#[derive(Deserialize)]
struct File {
    format: u32,
    #[serde(default)]
    source: Vec<Source>,
}

/// Why a text is no sources file
#[derive(Debug)]
pub enum SourcesError {
    /// The text is longer than [`FILE_BYTES_MAX`]
    TooLarge,
    /// TOML syntax, a missing key, a value of the wrong type
    Parse(toml::de::Error),
    /// A `format` other than [`FORMAT`]
    Format {
        /// The format as written
        found: u32,
    },
    /// More than [`SOURCES_MAX`] sources
    TooMany,
    /// A name that is empty or holds other than letters, digits and `_.-`
    Name {
        /// The name as written
        name: String,
    },
    /// Two sources of one name
    TwoSources {
        /// The name
        name: String,
    },
    /// A `root` that is empty
    Root {
        /// The source
        source: String,
    },
    /// A `repository` that is no `https://` or `http://` address
    Repository {
        /// The source
        source: String,
        /// The address as written
        repository: String,
    },
    /// A `commit` that is no full hash: 40 or 64 hex digits
    Commit {
        /// The source
        source: String,
        /// The commit as written
        commit: String,
    },
    /// A source without pages
    NoPages {
        /// The source
        source: String,
    },
    /// A page that is no relative path to a file of the docs folder
    Page {
        /// The source
        source: String,
        /// The page as written
        page: String,
    },
    /// A page listed under two sources
    PageTwice {
        /// The page
        page: String,
        /// The source listed first
        first: String,
        /// The other source
        second: String,
    },
}

impl std::fmt::Display for SourcesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(f, "a sources file is {FILE_BYTES_MAX} bytes at most"),
            Self::Parse(e) => write!(f, "{e}"),
            Self::Format { found } => write!(
                f,
                "format {found} is not the sources file's format {FORMAT}: the book's assembler and the kit disagree"
            ),
            Self::TooMany => write!(f, "more than {SOURCES_MAX} sources"),
            Self::Name { name } => write!(
                f,
                "source \"{name}\": a name is letters, digits and `_.-`, not empty"
            ),
            Self::TwoSources { name } => write!(f, "two sources are named \"{name}\""),
            Self::Root { source } => write!(f, "source \"{source}\": `root` is empty"),
            Self::Repository { source, repository } => write!(
                f,
                "source \"{source}\": repository \"{repository}\" is no https:// address"
            ),
            Self::Commit { source, commit } => write!(
                f,
                "source \"{source}\": commit \"{commit}\" is no full hash (40 or 64 hex digits)"
            ),
            Self::NoPages { source } => write!(f, "source \"{source}\" lists no pages"),
            Self::Page { source, page } => write!(
                f,
                "source \"{source}\": page \"{page}\" is no relative path under the docs folder, such as \"guide/intro.md\""
            ),
            Self::PageTwice {
                page,
                first,
                second,
            } => write!(
                f,
                "page \"{page}\" is listed under two sources, \"{first}\" and \"{second}\""
            ),
        }
    }
}

impl std::error::Error for SourcesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            _ => None,
        }
    }
}

/// The sources of a book
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sources {
    /// The sources, in file order
    pub sources: Vec<Source>,
    /// Each page and the index of its source
    by_page: BTreeMap<String, usize>,
}

impl Sources {
    /// Parses a sources file
    ///
    /// # Errors
    ///
    /// The text is too large, no sources file or of another format, lists too many sources, or a source's name,
    /// root, repository, commit or pages are malformed; a name or a page is listed twice.
    pub fn parse(text: &str) -> Result<Self, SourcesError> {
        if text.len() > FILE_BYTES_MAX {
            return Err(SourcesError::TooLarge);
        }
        let file: File = toml::from_str(text).map_err(SourcesError::Parse)?;
        if file.format != FORMAT {
            return Err(SourcesError::Format { found: file.format });
        }
        if file.source.len() > SOURCES_MAX {
            return Err(SourcesError::TooMany);
        }
        let mut by_page: BTreeMap<String, usize> = BTreeMap::new();
        for (i, s) in file.source.iter().enumerate() {
            check(s)?;
            if file.source[..i].iter().any(|o| o.name == s.name) {
                return Err(SourcesError::TwoSources {
                    name: s.name.clone(),
                });
            }
            for page in &s.pages {
                if let Some(&first) = by_page.get(page) {
                    return Err(SourcesError::PageTwice {
                        page: page.clone(),
                        first: file.source[first].name.clone(),
                        second: s.name.clone(),
                    });
                }
                by_page.insert(page.clone(), i);
            }
        }
        Ok(Self {
            sources: file.source,
            by_page,
        })
    }

    /// The source a page was copied from; `page` as Zensical names it, relative to the docs folder (`\` and a
    /// leading `./` are read as `/` and nothing)
    pub fn source_of(&self, page: &str) -> Option<&Source> {
        let page = page.replace('\\', "/");
        let page = page.strip_prefix("./").unwrap_or(&page);
        self.by_page.get(page).and_then(|&i| self.sources.get(i))
    }
}

/// A source's own fields checked
fn check(s: &Source) -> Result<(), SourcesError> {
    let source = || s.name.clone();
    if s.name.is_empty()
        || !s
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    {
        return Err(SourcesError::Name {
            name: s.name.clone(),
        });
    }
    if s.root.trim().is_empty() {
        return Err(SourcesError::Root { source: source() });
    }
    let host = s
        .repository
        .strip_prefix("https://")
        .or_else(|| s.repository.strip_prefix("http://"));
    if host.is_none_or(|h| h.is_empty() || h.chars().any(char::is_whitespace)) {
        return Err(SourcesError::Repository {
            source: source(),
            repository: s.repository.clone(),
        });
    }
    if !matches!(s.commit.len(), 40 | 64) || !s.commit.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(SourcesError::Commit {
            source: source(),
            commit: s.commit.clone(),
        });
    }
    if s.pages.is_empty() {
        return Err(SourcesError::NoPages { source: source() });
    }
    if let Some(page) = s.pages.iter().find(|p| !is_page(p)) {
        return Err(SourcesError::Page {
            source: source(),
            page: page.clone(),
        });
    }
    Ok(())
}

/// Whether `page` is a relative path to a file: `/` separators, no empty, `.` or `..` segment
fn is_page(page: &str) -> bool {
    !page.contains('\\')
        && page
            .split('/')
            .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

    fn file(source: &str) -> String {
        format!(
            "format = 1\n[[source]]\nname = \"a\"\nroot = \"../a\"\nrepository = \"https://github.com/o/a\"\ncommit = \"{COMMIT}\"\npages = [\"x/a.md\"]\n{source}"
        )
    }

    #[test]
    fn a_page_finds_its_source() {
        let s = Sources::parse(&file(
            "[[source]]\nname = \"b\"\nroot = \"/b\"\nrepository = \"https://github.com/o/b\"\ncommit = \"{COMMIT}\"\npages = [\"x/b.md\"]\n"
                .replace("{COMMIT}", COMMIT)
                .as_str(),
        ))
        .unwrap();
        assert_eq!(s.source_of("x/a.md").unwrap().name, "a");
        assert_eq!(s.source_of("./x/b.md").unwrap().name, "b");
        assert_eq!(s.source_of("x\\b.md").unwrap().name, "b");
        assert!(s.source_of("x/c.md").is_none());
        assert_eq!(s.sources[0].crates.len(), 0);
    }

    #[test]
    fn an_empty_file_lists_no_sources() {
        assert_eq!(Sources::parse("format = 1\n").unwrap(), Sources::default());
    }

    #[test]
    fn unknown_keys_are_ignored() {
        assert!(Sources::parse(&file("later = 1\n")).is_ok());
    }

    #[test]
    fn malformed_sources_are_refused() {
        let base = file("");
        for (text, want) in [
            ("format = 2\n".to_owned(), "format 2"),
            (base.replace("\"a\"\n", "\"a b\"\n"), "a name is"),
            (base.replace("../a", " "), "`root` is empty"),
            (
                base.replace("https://github.com/o/a", "github.com/o/a"),
                "no https://",
            ),
            (base.replace(COMMIT, "0123abc"), "no full hash"),
            (base.replace("[\"x/a.md\"]", "[]"), "lists no pages"),
            (base.replace("x/a.md", "../a.md"), "no relative path"),
            (base.replace("x/a.md", "/a.md"), "no relative path"),
            (
                format!("{base}{}", base.replace("format = 1\n", "")),
                "two sources",
            ),
            (
                format!(
                    "{base}{}",
                    base.replace("format = 1\n", "")
                        .replace("\"a\"\n", "\"b\"\n")
                ),
                "listed under two sources, \"a\" and \"b\"",
            ),
        ] {
            let e = Sources::parse(&text).unwrap_err().to_string();
            assert!(e.contains(want), "{text}\n{e}");
        }
    }

    #[test]
    fn too_many_sources_are_refused() {
        let one = file("");
        let one = one.replace("format = 1\n", "");
        let mut text = "format = 1\n".to_owned();
        for i in 0..=SOURCES_MAX {
            text += &one
                .replace("\"a\"\n", &format!("\"s{i}\"\n"))
                .replace("x/a.md", &format!("x/{i}.md"));
        }
        assert!(matches!(Sources::parse(&text), Err(SourcesError::TooMany)));
    }
}
