//! The code index: the JSON file of public items and text files that a book's code links name.
//!
//! It holds the book's own workspace and, by name, each source of the sources file ([`crate::sources`]): the other
//! repositories the book copies pages from, each at its own commit.
//!
//! `vz refs` writes it (default path [`crate::ext::DEFAULT_CODE_INDEX`]) at every book build and never keeps it in
//! git, so it always matches the code. The code link extension reads it and refuses an index whose `format` is
//! not [`FORMAT`]. Readers ignore fields they do not know; `tests/fixtures/code_index.json` is the layout's
//! snapshot.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The version of the index's layout; the code link extension refuses an index of another
pub const FORMAT: u32 = 3;

/// What an item is
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A function or method
    Fn,
    /// A struct
    Struct,
    /// A union
    Union,
    /// A named field of a struct or a union
    Field,
    /// An enum
    Enum,
    /// A variant of an enum
    Variant,
    /// A trait
    Trait,
    /// A type alias, associated types of a trait included
    Type,
    /// A constant, associated ones included
    Const,
    /// A static
    Static,
    /// An exported `macro_rules!` macro
    Macro,
}

impl Kind {
    /// Its name as the index writes it and a card shows it: `fn`, `struct`, ...
    pub fn name(self) -> &'static str {
        match self {
            Self::Fn => "fn",
            Self::Struct => "struct",
            Self::Union => "union",
            Self::Field => "field",
            Self::Enum => "enum",
            Self::Variant => "variant",
            Self::Trait => "trait",
            Self::Type => "type",
            Self::Const => "const",
            Self::Static => "static",
            Self::Macro => "macro",
        }
    }
}

/// One public item
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Its path from its crate's name through the modules it is defined in
    pub path: String,
    /// What it is
    pub kind: Kind,
    /// Its `#[cfg(...)]` predicate as written, several joined in `all(...)`; two arms of one item differ in it
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cfg: Option<String>,
    /// The file it is defined in, relative to the project root
    pub file: String,
    /// Its first and last line in the file, from 1: from its visibility to its end, docs and attributes left out
    pub lines: [usize; 2],
    /// Its signature, or its whole source when short, dedented
    pub source: String,
    /// Its docs, as written: Markdown with rustdoc's intra-doc links
    pub doc: String,
    /// Its lines on the forge at the index's commit
    pub url: String,
    /// For a re-export, the path the item is defined at
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reexports: Option<String>,
}

/// A library crate of the index, as docs.rs addresses it
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateEntry {
    /// The package name, which docs.rs puts first in its addresses (`ww-tape`; the library's own name is `ww_tape`)
    pub package: String,
    /// Whether the package is published on crates.io, as the code link extension's `on_crates_io` setting declares
    pub published: bool,
}

/// One text file of the project, which file references name
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    /// Its path, relative to the project root
    pub path: String,
    /// How many lines it has
    pub lines: usize,
    /// Its marked sections: the first and last line between the markers, from 1
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sections: BTreeMap<String, [usize; 2]>,
    /// Markers that do not pair up, as `file:line: what is wrong`; its sections are then left out
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

/// The index file
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    /// The version of its layout, [`FORMAT`]
    pub format: u32,
    /// The book's own workspace
    pub book: WorkspaceIndex,
    /// The workspaces of the sources file, by source name
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sources: BTreeMap<String, WorkspaceIndex>,
}

/// One workspace's libraries and text files, at one commit of its repository
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceIndex {
    /// The commit the addresses point at
    pub commit: String,
    /// The address a file's path follows to open it at the commit, `#L1-L9` after it for lines
    pub blob: String,
    /// The indexed libraries by library name, the first segment of their items' paths
    #[serde(default)]
    pub crates: BTreeMap<String, CrateEntry>,
    /// The public items, sorted by path
    pub items: Vec<Entry>,
    /// The project's text files, which file references name, sorted by path
    pub files: Vec<FileEntry>,
}
