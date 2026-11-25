//! The sources file read, and each source's workspace indexed beside the book's own.
//!
//! The sources file ([`vz_core::sources`]) names the repositories a book copies pages from. `vz refs` indexes each
//! source's workspace at its checkout, and the code link extension reads which page came from where; both read the
//! file through [`read`].

use crate::code_index::{Forge, IndexError, Libraries};
use crate::read::FileError;
use crate::workspace::{LoadError, Workspace};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use vz_core::code_index::WorkspaceIndex;
use vz_core::sources::{Source, Sources, SourcesError};

/// The sources file at `path`, named `file` in errors
///
/// # Errors
///
/// The file cannot be read or is no sources file.
pub fn read(path: &Path, file: &str) -> Result<Sources, FileError<SourcesError>> {
    crate::read::format(path, file, Sources::parse)
}

/// Why a source cannot be indexed
#[derive(Debug)]
pub struct SourceError {
    /// The source's name
    pub source: String,
    /// Why
    pub kind: SourceErrorKind,
}

/// Why a source cannot be indexed
#[derive(Debug)]
pub enum SourceErrorKind {
    /// A page it lists is not in the docs folder: the file is older than the pages
    NoPage {
        /// The page
        page: String,
    },
    /// Its workspace cannot be loaded
    Load(LoadError),
    /// Its root is not its cargo workspace's root, so its files' paths would not be the repository's
    NotRoot {
        /// The root, as the sources file gives it
        root: String,
        /// The workspace root cargo found
        workspace: PathBuf,
    },
    /// Its index cannot be built
    Index(IndexError),
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = &self.source;
        match &self.kind {
            SourceErrorKind::NoPage { page } => write!(
                f,
                "source \"{name}\": page {page} is not in the docs folder: run the book's assembler again"
            ),
            SourceErrorKind::Load(e) => write!(f, "source \"{name}\": {e}"),
            SourceErrorKind::NotRoot { root, workspace } => write!(
                f,
                "source \"{name}\": root {root} is not its cargo workspace's root {}",
                workspace.display()
            ),
            SourceErrorKind::Index(e) => write!(f, "source \"{name}\": {e}"),
        }
    }
}

impl std::error::Error for SourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            SourceErrorKind::Load(e) => Some(e),
            SourceErrorKind::Index(e) => Some(e),
            SourceErrorKind::NotRoot { .. } | SourceErrorKind::NoPage { .. } => None,
        }
    }
}

/// The index of every source's workspace, by source name; `project` is the project root its `root`s are relative
/// to, `docs` the docs folder its pages are in
///
/// # Errors
///
/// A page is not in the docs folder, a source's workspace cannot be loaded, its root is not the workspace's root,
/// or its index cannot be built.
pub fn index(
    project: &Path,
    docs: &Path,
    sources: &Sources,
) -> Result<BTreeMap<String, WorkspaceIndex>, SourceError> {
    sources
        .sources
        .iter()
        .map(|s| Ok((s.name.clone(), index_one(project, docs, s)?)))
        .collect()
}

fn index_one(project: &Path, docs: &Path, s: &Source) -> Result<WorkspaceIndex, SourceError> {
    let err = |kind| SourceError {
        source: s.name.clone(),
        kind,
    };
    if let Some(page) = s.pages.iter().find(|p| !docs.join(p).is_file()) {
        return Err(err(SourceErrorKind::NoPage { page: page.clone() }));
    }
    let root = project.join(&s.root);
    let workspace = Workspace::load(&root).map_err(|e| err(SourceErrorKind::Load(e)))?;
    let same = match (root.canonicalize(), workspace.root.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if !same {
        return Err(err(SourceErrorKind::NotRoot {
            root: s.root.clone(),
            workspace: workspace.root,
        }));
    }
    let libraries = Libraries {
        crates: &s.crates,
        on_crates_io: &s.on_crates_io,
    };
    let forge = Forge {
        repository: &s.repository,
        commit: &s.commit,
    };
    crate::code_index::index(&workspace, libraries, &[], forge)
        .map_err(|e| err(SourceErrorKind::Index(e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/projects/book")
    }

    fn sources() -> Sources {
        read(&book().join("sources.toml"), "sources.toml").expect("the fixture reads")
    }

    #[test]
    fn indexes_each_source_at_its_commit() {
        let book = book();
        let index = index(&book, &book.join("docs"), &sources()).unwrap();
        let la = &index["la"];
        assert_eq!(la.commit, "0123456789abcdef0123456789abcdef01234567");
        let raw = la.items.iter().find(|e| e.path == "la::RawChunk").unwrap();
        assert_eq!(
            raw.url,
            "https://github.com/o/la/blob/0123456789abcdef0123456789abcdef01234567/src/lib.rs#L4-L7"
        );
        assert!(la.crates["la"].published);
    }

    #[test]
    fn a_page_missing_from_the_docs_folder_fails() {
        let book = book();
        let e = index(&book, &book.join("src"), &sources()).unwrap_err();
        assert_eq!(
            e.to_string(),
            "source \"la\": page analyzer/la.md is not in the docs folder: run the book's assembler again"
        );
    }

    #[test]
    fn a_root_below_the_workspace_root_fails() {
        let book = book();
        let mut s = sources();
        s.sources[0].root = "../la/src".to_owned();
        let e = index(&book, &book.join("docs"), &s).unwrap_err();
        assert!(matches!(e.kind, SourceErrorKind::NotRoot { .. }), "{e}");
    }
}
