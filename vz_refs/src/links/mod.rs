//! Code links: the items and files a book's prose names, resolved through the code index, and their cards.
//!
//! `` [`TapeWriter::resume_on`] `` names a public item, `` [`resume.rs`] `` or `` [`crashes.rs:resume`] `` a file
//! or a marked section of one.
//!
//! - An item name matches an item whose path ends with it; a name that starts with a crate's name matches only in
//!   that crate. A name that is an item's whole path matches that item alone, not the items whose path merely ends
//!   with it, so the path an error suggests always resolves.
//! - A trailing `()` asks for a function and `!` for a macro. Two items with one path, a type and a function or two
//!   `#[cfg]` arms, are told apart as rustdoc does, by a kind before the name (`` [`fn@level`] ``) and by the
//!   arm's predicate (`` [`cfg(unix)@platform`] ``). A re-exported item is one item under all its paths.
//! - A file name matches a file git tracks whose path ends with it; a section is one marked
//!   `--8<-- [start:NAME]` ... `--8<-- [end:NAME]`.
//!
//! A page the book copied from another repository (the sources file, [`vz_core::sources`]) resolves its links
//! against that repository's workspace first, then against the book's own; its cards link that repository at the
//! commit the sources file gives.
//!
//! A name that matches nothing, or several things, is a [`LinkError`], which fails the build.

mod card;
mod docs;
mod docs_rs;

pub use card::{CardSizes, FileCardError};
pub use docs::bounded;

use crate::read::FileError;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use vz_core::code_index::{Entry, FORMAT, FileEntry, Index, Kind, WorkspaceIndex};
use vz_core::sources::{Sources, SourcesError};

/// Bytes of a code index read at most
// spirit-level: allow(exact-units-quantity) a byte count passed to take(), next to std's u64 sizes
pub const INDEX_BYTES_MAX: u64 = 256 << 20;

/// The kinds rustdoc writes before a name, and the kinds of the index each stands for, sorted by name
const KINDS: [(&str, &[Kind]); 15] = [
    ("const", &[Kind::Const]),
    ("constant", &[Kind::Const]),
    ("enum", &[Kind::Enum]),
    ("field", &[Kind::Field]),
    ("fn", &[Kind::Fn]),
    ("macro", &[Kind::Macro]),
    ("method", &[Kind::Fn]),
    ("static", &[Kind::Static]),
    ("struct", &[Kind::Struct]),
    ("trait", &[Kind::Trait]),
    ("tyalias", &[Kind::Type]),
    (
        "type",
        &[
            Kind::Struct,
            Kind::Enum,
            Kind::Union,
            Kind::Trait,
            Kind::Type,
        ],
    ),
    ("union", &[Kind::Union]),
    ("value", &[Kind::Fn, Kind::Const, Kind::Static]),
    ("variant", &[Kind::Variant]),
];

/// The place an item is defined at: its file and lines, which its re-exports share
type Site = (String, [usize; 2]);

fn site(e: &Entry) -> Site {
    (e.file.clone(), e.lines)
}

/// The first segment of a path: the crate's name
fn crate_of(path: &str) -> &str {
    path.split("::").next().unwrap_or(path)
}

/// The last segment of a path: the item's own name
fn own_name(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

/// Why the code index cannot be read
#[derive(Debug)]
pub enum IndexError {
    /// There is no file at the path: `vz prep` did not run
    Missing {
        /// The index, as the settings name it
        file: String,
    },
    /// The file cannot be read
    Read {
        /// The index, as the settings name it
        file: String,
        /// Why
        source: std::io::Error,
    },
    /// The file is no JSON, or no code index
    Json {
        /// The index, as the settings name it
        file: String,
        /// Why
        source: serde_json::Error,
    },
    /// The index has another layout than [`FORMAT`]
    Format {
        /// The index, as the settings name it
        file: String,
    },
    /// The sources file does not read
    Sources(FileError<SourcesError>),
    /// A source of the sources file is not in the index at its commit: the index was written before the file
    Stale {
        /// The index, as the settings name it
        file: String,
        /// The source's name
        source: String,
    },
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { file } => {
                write!(f, "{file} is missing: run `vz prep` before the build")
            }
            Self::Read { file, source } => write!(f, "{file}: {source}"),
            Self::Json { file, source } => write!(f, "{file}: {source}"),
            Self::Format { file } => write!(
                f,
                "{file} is not a code index of format {FORMAT}: rebuild it with `vz prep`"
            ),
            Self::Sources(e) => write!(f, "{e}"),
            Self::Stale { file, source } => write!(
                f,
                "{file} does not hold source \"{source}\" at the sources file's commit: rebuild it with `vz prep`"
            ),
        }
    }
}

impl std::error::Error for IndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::Sources(e) => Some(e),
            Self::Missing { .. } | Self::Format { .. } | Self::Stale { .. } => None,
        }
    }
}

/// Where a link was looked up: the book's own workspace, or a source of the sources file by name
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    /// The book's own workspace
    Book,
    /// A source of the sources file
    Source(String),
}

impl std::fmt::Display for Place {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Book => write!(f, "the book's"),
            Self::Source(name) => write!(f, "{name}'s"),
        }
    }
}

/// The places as a reader lists them: `ww_stdlib's or the book's`
fn either(places: &[Place]) -> String {
    places
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" or ")
}

/// A code link that names nothing, or several things
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkError {
    /// A kind before the name that rustdoc does not know
    NoKind {
        /// The link as written
        written: String,
        /// The kind
        kind: String,
    },
    /// No item has the name
    NoItem {
        /// The link as written
        written: String,
        /// Where it was looked up, in order
        places: Vec<Place>,
    },
    /// Several items have the name
    SeveralItems {
        /// The link as written
        written: String,
        /// Where they are
        place: Place,
        /// How a link names each alone
        forms: Vec<String>,
    },
    /// No file has the name
    NoFile {
        /// The file name as written
        name: String,
        /// Where it was looked up, in order
        places: Vec<Place>,
        /// Whether the index lists no file at all in those places
        none_listed: bool,
    },
    /// Several files have the name
    SeveralFiles {
        /// The file name as written
        name: String,
        /// Where they are
        place: Place,
        /// Their paths
        paths: Vec<String>,
    },
    /// The file's section markers do not pair up
    Markers {
        /// The link as written
        written: String,
        /// What is wrong, as the index says it
        problem: String,
    },
    /// The file marks no such section
    NoSection {
        /// The file name as written
        name: String,
        /// The section as written
        section: String,
        /// The file's path
        path: String,
        /// The sections it marks
        known: Vec<String>,
    },
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoKind { written, kind } => {
                let kinds: Vec<&str> = KINDS.iter().map(|(k, _)| *k).collect();
                write!(
                    f,
                    "{written}: `{kind}@` is no kind; the kinds are {}",
                    kinds.join(", ")
                )
            }
            Self::NoItem { written, places } => write!(
                f,
                "{written}: no public item of {} crates has this name",
                either(places)
            ),
            Self::SeveralItems {
                written,
                place,
                forms,
            } => {
                let forms: Vec<String> = forms.iter().map(|x| format!("[`{x}`]")).collect();
                write!(
                    f,
                    "{written}: several items of {place} crates have this name; write one of {}",
                    forms.join(", ")
                )
            }
            Self::NoFile {
                name,
                places,
                none_listed,
            } => {
                let hint = if *none_listed {
                    " (the index lists no files: vz refs ran outside a git checkout)"
                } else {
                    ""
                };
                write!(
                    f,
                    "[`{name}`]: no file git tracks in {} repository has this name{hint}",
                    either(places)
                )
            }
            Self::SeveralFiles { name, place, paths } => write!(
                f,
                "[`{name}`]: several files of {place} repository have this name ({}); name more of the path",
                paths.join(", ")
            ),
            Self::Markers { written, problem } => write!(f, "[`{written}`]: {problem}"),
            Self::NoSection {
                name,
                section,
                path,
                known,
            } => {
                let known = if known.is_empty() {
                    "none".to_owned()
                } else {
                    known.join(", ")
                };
                write!(
                    f,
                    "[`{name}:{section}`]: {path} marks no section `{section}` (it marks: {known})"
                )
            }
        }
    }
}

impl std::error::Error for LinkError {}

/// An item link as written: `` [`fn@cfg(unix)@a::b()`] ``
#[derive(Clone, Copy, Debug, Default)]
pub struct ItemLink<'a> {
    /// The kind before the name, without its `@`
    pub kind: Option<&'a str>,
    /// The arm's predicate before the name, `cfg(...)`, without its `@`
    pub cfg: Option<&'a str>,
    /// The name
    pub name: &'a str,
    /// `()`, `!` or nothing
    pub suffix: &'a str,
}

impl ItemLink<'_> {
    /// The link as written, brackets and backticks included
    fn written(&self) -> String {
        let kind = self.kind.map(|k| format!("{k}@")).unwrap_or_default();
        let cfg = self.cfg.map(|c| format!("{c}@")).unwrap_or_default();
        format!("[`{kind}{cfg}{}{}`]", self.name, self.suffix)
    }
}

/// The item a link names
#[derive(Clone, Debug)]
pub struct ItemRef {
    /// What its card is told apart by on a page
    pub key: String,
    /// Its lines on the forge
    pub href: String,
    /// What the link shows: the name as written
    pub text: String,
    /// The part of the index it is in
    part: usize,
    entry: Entry,
}

/// The file or section a link names
#[derive(Clone, Debug)]
pub struct FileRef {
    /// What its card is told apart by on a page
    pub key: String,
    /// Its lines on the forge
    pub href: String,
    /// What the link shows: the name as written
    pub text: String,
    /// The part of the index it is in
    part: usize,
    path: String,
    section: Option<String>,
    first: usize,
    last: usize,
}

/// One workspace of the index, as code links read it
#[derive(Clone, Debug)]
struct Part {
    place: Place,
    /// Where its files are, relative to the project root: empty for the book's own
    root: PathBuf,
    index: WorkspaceIndex,
    /// The indexed crates' names
    crates: BTreeSet<String>,
    /// Each definition's item by the path a card shows: the shortest that keeps the item's own name
    sites: BTreeMap<Site, usize>,
}

/// The code index as code links read it: the book's own workspace and the sources its copied pages come from
#[derive(Clone, Debug)]
pub struct CodeLinks {
    /// The book's own first, then the sources in the sources file's order
    parts: Vec<Part>,
    sources: Sources,
}

impl CodeLinks {
    /// The code index at `path`, named `file` in errors, with the sources file at `sources` (path and name) when
    /// the book copies pages
    ///
    /// # Errors
    ///
    /// The index is missing, cannot be read, or is no code index of [`FORMAT`]; the sources file does not read, or
    /// the index does not hold one of its sources at its commit.
    pub fn read(
        path: &Path,
        file: &str,
        sources: Option<(&Path, &str)>,
    ) -> Result<Self, IndexError> {
        let text = crate::read::text(path, INDEX_BYTES_MAX).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                IndexError::Missing {
                    file: file.to_owned(),
                }
            } else {
                IndexError::Read {
                    file: file.to_owned(),
                    source,
                }
            }
        })?;
        let json = |source| IndexError::Json {
            file: file.to_owned(),
            source,
        };
        #[derive(serde::Deserialize)]
        struct Layout {
            format: Option<serde_json::Value>,
        }
        let layout: Layout = serde_json::from_str(&text).map_err(json)?;
        if layout.format != Some(FORMAT.into()) {
            return Err(IndexError::Format {
                file: file.to_owned(),
            });
        }
        let index = serde_json::from_str(&text).map_err(json)?;
        let sources = match sources {
            Some((path, name)) => crate::sources::read(path, name).map_err(IndexError::Sources)?,
            None => Sources::default(),
        };
        Self::with_sources(index, sources).map_err(|source| IndexError::Stale {
            file: file.to_owned(),
            source,
        })
    }

    /// Code links over `index`, the book's own workspace alone
    pub fn new(index: Index) -> Self {
        Self {
            parts: vec![Part::new(Place::Book, PathBuf::new(), index.book)],
            sources: Sources::default(),
        }
    }

    /// Code links over `index` with the pages of `sources` resolved against their source first
    ///
    /// # Errors
    ///
    /// The name of a source the index does not hold at the commit the sources file gives.
    pub fn with_sources(index: Index, sources: Sources) -> Result<Self, String> {
        let Index {
            book,
            sources: mut indexed,
            ..
        } = index;
        let mut parts = vec![Part::new(Place::Book, PathBuf::new(), book)];
        for s in &sources.sources {
            let part = indexed
                .remove(&s.name)
                .filter(|p| p.commit == s.commit)
                .ok_or_else(|| s.name.clone())?;
            parts.push(Part::new(
                Place::Source(s.name.clone()),
                PathBuf::from(&s.root),
                part,
            ));
        }
        Ok(Self { parts, sources })
    }

    /// The parts a link on `page` is looked up in, in order: its source's first when it was copied, then the
    /// book's own
    fn order(&self, page: Option<&str>) -> Vec<usize> {
        let source = page.and_then(|p| self.sources.source_of(p)).and_then(|s| {
            self.parts
                .iter()
                .position(|p| p.place == Place::Source(s.name.clone()))
        });
        source.into_iter().chain([0]).collect()
    }

    /// The one item `link` on `page` names (`page` relative to the docs folder, as Zensical names it)
    ///
    /// # Errors
    ///
    /// The kind is unknown, or the link names no item in any place it is looked up in, or several in the first
    /// that has it.
    pub fn item(&self, link: &ItemLink<'_>, page: Option<&str>) -> Result<ItemRef, LinkError> {
        let written = link.written();
        let kinds = match link.kind {
            Some(kind) => Some(
                KINDS
                    .iter()
                    .find(|(k, _)| *k == kind)
                    .map(|(_, kinds)| *kinds)
                    .ok_or_else(|| LinkError::NoKind {
                        written: written.clone(),
                        kind: kind.to_owned(),
                    })?,
            ),
            None => match link.suffix {
                "()" => Some(&[Kind::Fn][..]),
                "!" => Some(&[Kind::Macro][..]),
                _ => None,
            },
        };
        let order = self.order(page);
        for &n in &order {
            let part = &self.parts[n];
            let found = part.matches(link.name, kinds, link.cfg);
            match found.as_slice() {
                [] => {}
                [e] => {
                    let at = match &part.place {
                        Place::Book => String::new(),
                        Place::Source(name) => format!("{name}:"),
                    };
                    return Ok(ItemRef {
                        key: format!("item {at}{} {}:{}", e.path, e.file, e.lines[0]),
                        href: e.url.clone(),
                        text: format!("{}{}", link.name, link.suffix),
                        part: n,
                        entry: (*e).clone(),
                    });
                }
                several => {
                    return Err(LinkError::SeveralItems {
                        written,
                        place: part.place.clone(),
                        forms: several.iter().map(|e| form(e, several)).collect(),
                    });
                }
            }
        }
        Err(LinkError::NoItem {
            written,
            places: self.places(&order),
        })
    }

    fn places(&self, order: &[usize]) -> Vec<Place> {
        order.iter().map(|&n| self.parts[n].place.clone()).collect()
    }

    /// The file `name` on `page` names, or its section `section`
    ///
    /// # Errors
    ///
    /// The link names no file in any place it is looked up in, or several in the first that has it; the file's
    /// markers do not pair up, or it marks no such section.
    pub fn file(
        &self,
        name: &str,
        section: Option<&str>,
        page: Option<&str>,
    ) -> Result<FileRef, LinkError> {
        let want: Vec<&str> = name.split('/').collect();
        let order = self.order(page);
        let mut at = None;
        for &n in &order {
            let part = &self.parts[n];
            let found: Vec<&FileEntry> = part
                .index
                .files
                .iter()
                .filter(|f| f.path.split('/').collect::<Vec<_>>().ends_with(&want))
                .collect();
            match found.as_slice() {
                [] => {}
                [f] => {
                    at = Some((n, *f));
                    break;
                }
                several => {
                    return Err(LinkError::SeveralFiles {
                        name: name.to_owned(),
                        place: part.place.clone(),
                        paths: several.iter().map(|f| f.path.clone()).collect(),
                    });
                }
            }
        }
        let Some((n, f)) = at else {
            return Err(LinkError::NoFile {
                name: name.to_owned(),
                places: self.places(&order),
                none_listed: order.iter().all(|&n| self.parts[n].index.files.is_empty()),
            });
        };
        let text = match section {
            Some(s) => format!("{name}:{s}"),
            None => name.to_owned(),
        };
        if let Some(problem) = &f.problem {
            return Err(LinkError::Markers {
                written: text,
                problem: problem.clone(),
            });
        }
        let blob = &self.parts[n].index.blob;
        let (first, last, href) = match section {
            None => (1, f.lines, format!("{blob}{}", f.path)),
            Some(s) => {
                let [first, last] = *f.sections.get(s).ok_or_else(|| LinkError::NoSection {
                    name: name.to_owned(),
                    section: s.to_owned(),
                    path: f.path.clone(),
                    known: f.sections.keys().cloned().collect(),
                })?;
                (first, last, format!("{blob}{}#L{first}-L{last}", f.path))
            }
        };
        let place = match &self.parts[n].place {
            Place::Book => String::new(),
            Place::Source(name) => format!("{name}:"),
        };
        Ok(FileRef {
            key: format!("file {place}{}:{}", f.path, section.unwrap_or("")),
            href,
            text,
            part: n,
            path: f.path.clone(),
            section: section.map(str::to_owned),
            first,
            last,
        })
    }
}

impl Part {
    fn new(place: Place, root: PathBuf, index: WorkspaceIndex) -> Self {
        let crates = index
            .items
            .iter()
            .map(|i| crate_of(&i.path).to_owned())
            .collect();
        let mut defined: BTreeMap<Site, &str> = BTreeMap::new();
        for i in index.items.iter().filter(|i| i.reexports.is_none()) {
            defined.insert(site(i), own_name(&i.path));
        }
        let mut order: Vec<usize> = (0..index.items.len()).collect();
        order.sort_by_cached_key(|&n| {
            let i = &index.items[n];
            (
                defined.get(&site(i)) != Some(&own_name(&i.path)),
                i.path.matches("::").count(),
                i.path.clone(),
            )
        });
        let mut sites = BTreeMap::new();
        for n in order {
            sites.entry(site(&index.items[n])).or_insert(n);
        }
        Self {
            place,
            root,
            index,
            crates,
            sites,
        }
    }

    /// The items `name` names, of `kinds` and in the arm `cfg` when given: one per definition, by its shortest
    /// path, sorted by path
    fn matches(&self, name: &str, kinds: Option<&[Kind]>, cfg: Option<&str>) -> Vec<&Entry> {
        let want: Vec<&str> = name.split("::").collect();
        let in_crate = want.len() > 1 && self.crates.contains(want[0]);
        let mut found: Vec<&Entry> = self
            .index
            .items
            .iter()
            .filter(|i| {
                let segs: Vec<&str> = i.path.split("::").collect();
                if in_crate {
                    segs[0] == want[0] && segs.ends_with(&want[1..])
                } else {
                    segs.ends_with(&want)
                }
            })
            .filter(|i| kinds.is_none_or(|k| k.contains(&i.kind)))
            .filter(|i| {
                cfg.is_none_or(|c| {
                    bare(&format!("cfg({})", i.cfg.as_deref().unwrap_or(""))) == bare(c)
                })
            })
            .collect();
        if found.iter().any(|i| i.path == name) {
            found.retain(|i| i.path == name);
        }
        let first: BTreeMap<Site, &Entry> = found
            .iter()
            .map(|i| {
                let s = site(i);
                let n = *self.sites.get(&s).expect("every item's site has an item");
                (s, &self.index.items[n])
            })
            .collect();
        let mut out: Vec<&Entry> = first.into_values().collect();
        out.sort_by(|a, b| (&a.path, a.lines).cmp(&(&b.path, b.lines)));
        out
    }

    /// The item an intra-doc link in `at`'s docs names: the one match, else the one in `at`'s crate
    fn near(&self, name: &str, at: &Entry) -> Option<&Entry> {
        let mut found = self.matches(name, None, None);
        if found.len() > 1 {
            let krate = crate_of(&at.path);
            found.retain(|i| crate_of(&i.path) == krate);
        }
        match found.as_slice() {
            [one] => Some(one),
            _ => None,
        }
    }
}

/// A predicate without its whitespace, to compare two as written
fn bare(cfg: &str) -> String {
    cfg.chars().filter(|c| !c.is_whitespace()).collect()
}

/// How a link names `e` alone among `among`: its path, with its kind and arm where others share it
fn form(e: &Entry, among: &[&Entry]) -> String {
    let same: Vec<&&Entry> = among.iter().filter(|i| i.path == e.path).collect();
    let kinds: BTreeSet<&str> = same.iter().map(|i| i.kind.name()).collect();
    let cfgs: BTreeSet<Option<&str>> = same.iter().map(|i| i.cfg.as_deref()).collect();
    let kind = if kinds.len() > 1 {
        format!("{}@", e.kind.name())
    } else {
        String::new()
    };
    let cfg = match &e.cfg {
        Some(c) if cfgs.len() > 1 => format!("cfg({c})@"),
        _ => String::new(),
    };
    format!("{kind}{cfg}{}", e.path)
}

#[cfg(test)]
mod tests;
