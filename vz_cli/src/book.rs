//! Reading a book's `zensical.toml`: the handful of settings the kit's commands need, beyond the Markdown
//! extensions' own settings ([`vz_core::ext`]).

use std::fmt;
use std::path::{Path, PathBuf};
use toml::Value;

/// The book's configuration file, at the project root
pub(crate) const FILE: &str = "zensical.toml";

/// The docs folder when the book names none
const DEFAULT_DOCS_DIR: &str = "docs";

/// The built site's folder when the book names none
const DEFAULT_SITE_DIR: &str = "site";

/// Why `zensical.toml` cannot be read
#[derive(Debug)]
pub(crate) enum ReadError {
    /// It could not be read
    Read(std::io::Error),
    /// It is not valid TOML
    Parse(toml::de::Error),
    /// It has no `[project]` table
    NoProject,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(_) => write!(f, "reading {FILE}"),
            Self::Parse(_) => write!(f, "parsing {FILE}"),
            Self::NoProject => write!(f, "{FILE} has no [project] table"),
        }
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(e) => Some(e),
            Self::Parse(e) => Some(e),
            Self::NoProject => None,
        }
    }
}

/// A Zensical book, as far as `vz`'s commands need it
#[derive(Debug)]
pub(crate) struct Book {
    /// The project root, where `zensical.toml` is read from and every relative path starts
    pub(crate) root: PathBuf,
    /// `docs_dir`, relative to the root
    pub(crate) docs_dir: String,
    /// `site_dir`, relative to the root: left out of the code index's text files
    pub(crate) site_dir: String,
    /// `theme.custom_dir`, relative to the root; a book has no place for the stamp partial without one
    pub(crate) custom_dir: Option<String>,
    /// `repo_url`: the repository code links and the stamp read commits against
    pub(crate) repo_url: Option<String>,
    /// `nav`: the pages in click order, as written; empty when the book names none
    pub(crate) nav: Vec<Value>,
    /// `markdown_extensions`, read through [`vz_core::ext`]
    markdown_extensions: toml::Table,
}

impl Book {
    /// Reads `zensical.toml` at `root`
    ///
    /// # Errors
    ///
    /// It cannot be read, does not parse, or has no `[project]` table.
    pub(crate) fn read(root: &Path) -> Result<Self, ReadError> {
        let text = std::fs::read_to_string(root.join(FILE)).map_err(ReadError::Read)?;
        let doc: toml::Table = text.parse().map_err(ReadError::Parse)?;
        let project = doc
            .get("project")
            .and_then(Value::as_table)
            .ok_or(ReadError::NoProject)?;
        let string = |key: &str| project.get(key).and_then(Value::as_str).map(str::to_owned);
        let custom_dir = project
            .get("theme")
            .and_then(Value::as_table)
            .and_then(|t| t.get("custom_dir"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let markdown_extensions = project
            .get("markdown_extensions")
            .and_then(Value::as_table)
            .cloned()
            .unwrap_or_default();
        Ok(Self {
            root: root.to_owned(),
            docs_dir: string("docs_dir").unwrap_or_else(|| DEFAULT_DOCS_DIR.to_owned()),
            site_dir: string("site_dir").unwrap_or_else(|| DEFAULT_SITE_DIR.to_owned()),
            custom_dir,
            repo_url: string("repo_url"),
            nav: project
                .get("nav")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            markdown_extensions,
        })
    }

    /// The kit's extensions the book turns on, with their settings
    ///
    /// # Errors
    ///
    /// A setting has the wrong type.
    pub(crate) fn settings(&self) -> Result<vz_core::ext::Settings, vz_core::ext::SettingsError> {
        vz_core::ext::Settings::from_toml(&self.markdown_extensions)
    }

    /// A path relative to the docs folder, as a path relative to the root
    pub(crate) fn docs(&self, rel: &str) -> PathBuf {
        self.root.join(&self.docs_dir).join(rel)
    }

    /// A path relative to the theme's `custom_dir`, as a path relative to the root; `None` when the book names
    /// no `custom_dir`
    pub(crate) fn custom(&self, rel: &str) -> Option<PathBuf> {
        self.custom_dir
            .as_ref()
            .map(|c| self.root.join(c).join(rel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(text: &str) -> Book {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("vz_cli_book_test_{}_{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE), text).unwrap();
        let b = Book::read(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        b
    }

    #[test]
    fn defaults_when_the_book_names_none() {
        let b = book("[project]\nsite_name = \"x\"\n");
        assert_eq!(b.docs_dir, "docs");
        assert_eq!(b.site_dir, "site");
        assert_eq!(b.custom_dir, None);
        assert_eq!(b.repo_url, None);
        assert_eq!(b.docs("a.css"), b.root.join("docs/a.css"));
    }

    #[test]
    fn every_setting_as_written() {
        let b = book(
            "[project]\nsite_name = \"x\"\ndocs_dir = \"pages\"\nsite_dir = \"out\"\nrepo_url = \"https://github.com/o/r\"\n[project.theme]\ncustom_dir = \"overrides\"\n",
        );
        assert_eq!(b.docs_dir, "pages");
        assert_eq!(b.site_dir, "out");
        assert_eq!(b.custom_dir.as_deref(), Some("overrides"));
        assert_eq!(b.repo_url.as_deref(), Some("https://github.com/o/r"));
        assert_eq!(
            b.custom("partials/vz-stamp.html").unwrap(),
            b.root.join("overrides/partials/vz-stamp.html")
        );
    }

    #[test]
    fn no_custom_dir_gives_no_path() {
        let b = book("[project]\nsite_name = \"x\"\n");
        assert_eq!(b.custom("partials/vz-stamp.html"), None);
    }

    #[test]
    fn extensions_reach_the_settings() {
        let b = book(
            "[project]\n[project.markdown_extensions]\n\"vhrd_zensical.glossary\" = { page = \"/terms/\" }\n",
        );
        let s = b.settings().unwrap();
        assert_eq!(s.glossary.unwrap().page, "/terms/");
    }

    #[test]
    fn no_project_table_is_an_error() {
        let dir =
            std::env::temp_dir().join(format!("vz_cli_book_test_noproj_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE), "site_name = \"x\"\n").unwrap();
        let e = Book::read(&dir).unwrap_err();
        assert!(matches!(e, ReadError::NoProject), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
