//! The book's frame: the reading ramp, the shelves outside it and the waived fences of `book_checks.toml`, and the
//! check of a book's nav and pages against them.
//!
//! ```toml
//! outside = ["Reference", "Articles"]  # top-level nav sections after the ramp; their pages may have any level
//!
//! [[part]]                             # the reading ramp, in nav order
//! title = "Basic"                      # a top-level nav section
//! level = "basic"                      # the level of every page in it: basic, advanced or nomicon
//!
//! [bare_fences]                        # pages still holding bare `rust` fences, under the docs folder
//! "guide/old.md" = 3                   # how many it may hold; the count may only go down
//! ```
//!
//! [`check`] reads the nav in click order (next, next) and fails:
//!
//! - a ramp that ever steps down a level, or puts a page in a part of another level;
//! - a top-level nav section that is neither a ramp part nor a shelf outside it, and a ramp part after a shelf;
//! - a page without a valid `level:` in its front matter, a page the nav lists twice, a page that is not there;
//! - a bare `rust` fence beyond the page's waived count: outside Nomicon every `rust` fence is an include of a
//!   marked section of a compiled source (`--8<-- "path:section"`); a Nomicon page may also show a sketch, marked
//!   `title="sketch: ..."`. A waived count higher than the page's fences fails too, so the count only goes down;
//! - a file under the docs folder's [`GENERATED_DIR`] that no page includes: stale generated output.
//!
//! Types, parsers and the check over texts the caller reads; no I/O.

use crate::FILE_BYTES_MAX;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The frame file, at the project root
pub const FILE: &str = "book_checks.toml";

/// Where golden tests write what a reader sees (transcripts, bytes, chart data), under the docs folder
pub const GENERATED_DIR: &str = "generated";

/// Entries one nav holds at most, sections counted
pub const NAV_ENTRIES_MAX: usize = 1024;

/// Sections a nav nests at most
pub const NAV_DEPTH_MAX: usize = 8;

/// How deep a page goes, from its front matter's `level:`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// What a reader needs to use the thing the book documents
    Basic,
    /// Every option and when to pick it
    Advanced,
    /// The internals
    Nomicon,
}

impl Level {
    /// The level a front matter's `level:` value names
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "basic" => Some(Self::Basic),
            "advanced" => Some(Self::Advanced),
            "nomicon" => Some(Self::Nomicon),
            _ => None,
        }
    }

    /// Its name with the article: `a basic`, `an advanced`
    fn with_article(self) -> &'static str {
        match self {
            Self::Basic => "a basic",
            Self::Advanced => "an advanced",
            Self::Nomicon => "a nomicon",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Basic => "basic",
            Self::Advanced => "advanced",
            Self::Nomicon => "nomicon",
        })
    }
}

/// One part of the reading ramp: a top-level nav section whose pages all have one level
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    /// The section's title in the nav
    pub title: String,
    /// The level of every page in it
    pub level: Level,
}

/// What [`FILE`] holds
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checks {
    /// The parts of the ramp, in reading order
    pub part: Vec<Part>,
    /// Titles of the top-level sections after the ramp (Reference, Articles), whose pages may have any level
    pub outside: Vec<String>,
    /// Pages still holding bare `rust` fences, under the docs folder, with how many each may hold
    #[serde(default)]
    pub bare_fences: BTreeMap<String, usize>,
}

/// Why a text is no frame file
#[derive(Debug)]
pub enum ChecksError {
    /// The text is longer than [`FILE_BYTES_MAX`]
    TooLarge,
    /// TOML syntax, a missing or unknown key, a value of the wrong type, a level other than the three
    Parse(toml::de::Error),
    /// A title named twice, by two parts, two shelves, or a part and a shelf
    TitleTwice {
        /// The title
        title: String,
    },
}

impl fmt::Display for ChecksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => write!(f, "a frame file is {FILE_BYTES_MAX} bytes at most"),
            Self::Parse(e) => write!(f, "{e}"),
            Self::TitleTwice { title } => write!(
                f,
                "section \"{title}\" is named twice among the parts and the shelves outside them"
            ),
        }
    }
}

impl std::error::Error for ChecksError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            Self::TooLarge | Self::TitleTwice { .. } => None,
        }
    }
}

impl Checks {
    /// Parses a frame file
    ///
    /// # Errors
    ///
    /// The text is too large or no frame file, or a section title is named twice.
    pub fn parse(text: &str) -> Result<Self, ChecksError> {
        if text.len() > FILE_BYTES_MAX {
            return Err(ChecksError::TooLarge);
        }
        let checks: Self = toml::from_str(text).map_err(ChecksError::Parse)?;
        let mut titles = BTreeSet::new();
        let all = checks.part.iter().map(|p| &p.title).chain(&checks.outside);
        for title in all {
            if !titles.insert(title.as_str()) {
                return Err(ChecksError::TitleTwice {
                    title: title.clone(),
                });
            }
        }
        Ok(checks)
    }
}

/// One page of the nav, in click order
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavPage {
    /// Its path under the docs folder
    pub page: String,
    /// The top-level section it is in
    pub part: String,
}

/// A book's nav: `zensical.toml`'s `project.nav`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Nav {
    /// The titles of its top-level sections, in order
    pub sections: Vec<String>,
    /// Its pages in click order; links to other sites are left out
    pub pages: Vec<NavPage>,
}

/// Why a nav cannot be read in click order
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavError {
    /// A top-level entry that is no section `{ "Title" = [...] }`
    NotASection {
        /// Its place among the top-level entries, from 1
        index: usize,
    },
    /// More than [`NAV_ENTRIES_MAX`] entries
    TooMany,
    /// Sections nested deeper than [`NAV_DEPTH_MAX`]
    TooDeep {
        /// The top-level section
        part: String,
    },
    /// An entry that is neither a page, a list nor a section of one title
    Entry {
        /// The top-level section
        part: String,
    },
}

impl fmt::Display for NavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotASection { index } => write!(
                f,
                "top-level nav entry {index} is no section of its own, `{{ \"Title\" = [...] }}`"
            ),
            Self::TooMany => write!(f, "the nav has more than {NAV_ENTRIES_MAX} entries"),
            Self::TooDeep { part } => write!(
                f,
                "the nav nests deeper than {NAV_DEPTH_MAX} sections in `{part}`"
            ),
            Self::Entry { part } => write!(
                f,
                "a nav entry in `{part}` is neither a page nor a section of one title"
            ),
        }
    }
}

impl std::error::Error for NavError {}

impl Nav {
    /// Reads a nav from `zensical.toml`'s `project.nav` array, in click order
    ///
    /// # Errors
    ///
    /// A top-level entry is no section, the nav is too large or too deep, or an entry is malformed.
    pub fn from_toml(nav: &[toml::Value]) -> Result<Self, NavError> {
        let mut out = Self::default();
        let mut entries = 0;
        for (i, entry) in nav.iter().enumerate() {
            let (title, value) =
                single_entry(entry).ok_or(NavError::NotASection { index: i + 1 })?;
            out.sections.push(title.to_owned());
            walk(value, title, 1, &mut out.pages, &mut entries)?;
        }
        Ok(out)
    }
}

/// The title and value of a nav entry written `{ "Title" = value }`
fn single_entry(entry: &toml::Value) -> Option<(&str, &toml::Value)> {
    let table = entry.as_table()?;
    let mut items = table.iter();
    let (title, value) = items.next()?;
    items.next().is_none().then_some((title.as_str(), value))
}

fn walk(
    value: &toml::Value,
    part: &str,
    depth: usize,
    pages: &mut Vec<NavPage>,
    entries: &mut usize,
) -> Result<(), NavError> {
    *entries += 1;
    if *entries > NAV_ENTRIES_MAX {
        return Err(NavError::TooMany);
    }
    if depth > NAV_DEPTH_MAX {
        return Err(NavError::TooDeep {
            part: part.to_owned(),
        });
    }
    match value {
        toml::Value::String(page) => {
            if !page.contains("://") {
                pages.push(NavPage {
                    page: page.clone(),
                    part: part.to_owned(),
                });
            }
        }
        toml::Value::Array(items) => {
            for item in items {
                walk(item, part, depth + 1, pages, entries)?;
            }
        }
        toml::Value::Table(_) => {
            let (_, inner) = single_entry(value).ok_or_else(|| NavError::Entry {
                part: part.to_owned(),
            })?;
            walk(inner, part, depth + 1, pages, entries)?;
        }
        _ => {
            return Err(NavError::Entry {
                part: part.to_owned(),
            });
        }
    }
    Ok(())
}

/// The `level:` of a page's front matter (the block between the first two `---` lines): `None` without one, the
/// value as written when it names no level
pub fn front_matter_level(text: &str) -> Option<Result<Level, String>> {
    let mut lines = text.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    for line in lines {
        if line.trim_end() == "---" {
            return None;
        }
        if let Some(value) = line.strip_prefix("level:") {
            let value = value.trim();
            return Some(Level::parse(value).ok_or_else(|| value.to_owned()));
        }
    }
    None
}

/// A `rust` fence of a page that is not an include
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BareFence {
    /// Its opening line, counted from 1
    pub line: usize,
    /// Whether it is marked as a sketch (`title="sketch..."`), which a Nomicon page may show
    pub sketch: bool,
}

/// The fence open while [`bare_rust_fences`] reads a page
struct OpenFence {
    /// Its character, `` ` `` or `~`
    ch: char,
    /// How many of them open it
    len: usize,
    /// Whether its language is `rust` or `rs`
    rust: bool,
    /// Where it opens and whether it is a sketch
    fence: BareFence,
    /// Whether every body line so far is an include; `None`: no body line yet
    includes: Option<bool>,
}

impl OpenFence {
    /// The fence `trimmed`, line `line` of its page, opens; `None` when it opens none
    fn opening(trimmed: &str, line: usize) -> Option<Self> {
        let ch = trimmed.chars().next().filter(|c| *c == '`' || *c == '~')?;
        let len = trimmed.chars().take_while(|c| *c == ch).count();
        let info = trimmed[len..].trim();
        if len < 3 || (ch == '`' && info.contains('`')) {
            return None;
        }
        let lang = info
            .split(|c: char| c.is_whitespace() || c == '}')
            .next()
            .unwrap_or_default()
            .trim_start_matches(['{', '.']);
        Some(Self {
            ch,
            len,
            rust: lang == "rust" || lang == "rs",
            fence: BareFence {
                line,
                sketch: is_sketch(info),
            },
            includes: None,
        })
    }

    /// Whether it is a `rust` fence whose body is not only includes
    fn is_bare(&self) -> bool {
        self.rust && self.includes != Some(true)
    }
}

/// The `rust` fences of a page (`rs` too) whose body is not only `--8<--` include lines. A fence inside another
/// fence (a ````` ````text ````` showing Markdown) is the outer fence's text, not a fence of its own.
pub fn bare_rust_fences(text: &str) -> Vec<BareFence> {
    let mut bare = Vec::new();
    let mut open: Option<OpenFence> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(o) = &mut open {
            if trimmed.len() >= o.len && trimmed.chars().all(|c| c == o.ch) {
                if o.is_bare() {
                    bare.push(o.fence);
                }
                open = None;
            } else if !trimmed.is_empty() {
                let include = trimmed.starts_with("--8<--");
                o.includes = Some(o.includes.unwrap_or(true) && include);
            }
            continue;
        }
        open = OpenFence::opening(trimmed, index + 1);
    }
    if let Some(o) = open
        && o.is_bare()
    {
        bare.push(o.fence);
    }
    bare
}

/// Whether a fence's info string carries `title="sketch..."`
fn is_sketch(info: &str) -> bool {
    info.split_once("title=")
        .is_some_and(|(_, rest)| rest.trim_start_matches(['"', '\'']).starts_with("sketch"))
}

/// The files a page includes as snippets (`--8<-- "path"`, `--8<-- "path:section"`), as written
pub fn includes(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter_map(|line| {
        let rest = line.trim().strip_prefix("--8<--")?.trim();
        let quoted = rest.strip_prefix('"')?;
        let path = &quoted[..quoted.find('"')?];
        Some(path.split_once(':').map_or(path, |(path, _)| path))
    })
}

/// A page of the nav as the caller read it
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Its text
    pub text: String,
    /// Whether it is copied from another repository, which the book takes at a commit it does not pin: its fence
    /// count may drop under the book's feet, so a count lower than waived is no failure there
    pub copied: bool,
}

/// One thing wrong with a book's frame
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The nav's ramp parts are not [`Checks::part`] in order
    RampOrder {
        /// The ramp parts in nav order
        nav: Vec<String>,
        /// The ramp parts the frame file names
        checks: Vec<String>,
    },
    /// A ramp part after a shelf outside the ramp
    PartAfterShelf {
        /// The part
        part: String,
        /// The first shelf
        shelf: String,
    },
    /// A top-level section that is neither a ramp part nor a shelf
    UnknownSection {
        /// Its title
        section: String,
    },
    /// A page the nav lists twice
    Twice {
        /// The page
        page: String,
    },
    /// A page the nav lists that cannot be read
    Unreadable {
        /// The page
        page: String,
        /// Why, when it is there; `None`: no such page
        why: Option<String>,
    },
    /// A page without a valid `level:`
    Level {
        /// The page
        page: String,
        /// The value as written; `None`: no `level:` at all
        value: Option<String>,
    },
    /// A page in a ramp part of another level
    WrongPart {
        /// The page
        page: String,
        /// Its level
        level: Level,
        /// The part
        part: String,
        /// The part's level
        part_level: Level,
    },
    /// A page whose level is lower than the ramp page before it
    StepDown {
        /// The page
        page: String,
        /// Its level
        level: Level,
        /// The page before it in the ramp
        before: String,
        /// That page's level
        before_level: Level,
    },
    /// More bare `rust` fences than waived
    BareFences {
        /// The page
        page: String,
        /// Its level: a Nomicon page may show sketches
        level: Level,
        /// The opening lines of its bare fences
        lines: Vec<usize>,
        /// The count waived
        allowed: usize,
    },
    /// Fewer bare `rust` fences than waived: the count goes down
    WaivedTooMany {
        /// The page
        page: String,
        /// Its bare fences
        count: usize,
        /// The count waived
        allowed: usize,
    },
    /// A waiver of a page the nav does not list
    WaiverUnlisted {
        /// The page
        page: String,
    },
    /// A generated file no page includes
    NotIncluded {
        /// The file, as a page would include it
        file: String,
    },
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RampOrder { nav, checks } => write!(
                f,
                "the nav's ramp parts are {nav:?}, {FILE} names {checks:?} in that order"
            ),
            Self::PartAfterShelf { part, shelf } => write!(
                f,
                "ramp part `{part}` comes after a shelf outside the ramp (`{shelf}`)"
            ),
            Self::UnknownSection { section } => write!(
                f,
                "top-level section `{section}` is neither a ramp part nor a shelf outside it in {FILE}"
            ),
            Self::Twice { page } => write!(f, "{page}: listed twice in the nav"),
            Self::Unreadable { page, why: None } => {
                write!(f, "{page}: in the nav, but no such page")
            }
            Self::Unreadable {
                page,
                why: Some(why),
            } => write!(f, "{page}: in the nav, but unreadable: {why}"),
            Self::Level { page, value: None } => {
                write!(f, "{page}: no `level:` in its front matter")
            }
            Self::Level {
                page,
                value: Some(value),
            } => write!(
                f,
                "{page}: `level: {value}` is none of basic, advanced, nomicon"
            ),
            Self::WrongPart {
                page,
                level,
                part,
                part_level,
            } => write!(
                f,
                "{page}: {} page in part `{part}`, whose pages are {part_level}",
                level.with_article()
            ),
            Self::StepDown {
                page,
                level,
                before,
                before_level,
            } => write!(
                f,
                "{page}: the nav steps down from {before_level} ({before}) to {level}"
            ),
            Self::BareFences {
                page,
                level,
                lines,
                allowed,
            } => {
                let what = if *level == Level::Nomicon {
                    "neither an include nor a sketch (`title=\"sketch: ...\"`)"
                } else {
                    "not an include of a compiled source (`--8<-- \"path:section\"`)"
                };
                let lines: Vec<String> = lines.iter().map(usize::to_string).collect();
                write!(
                    f,
                    "{page}: {} `rust` fences {what}, {allowed} waived in {FILE}; lines {}",
                    lines.len(),
                    lines.join(", ")
                )
            }
            Self::WaivedTooMany {
                page,
                count,
                allowed,
            } => write!(
                f,
                "{page}: {count} bare `rust` fences, {allowed} waived: lower its count in {FILE} to {count}{}",
                if *count == 0 {
                    ", that is, drop the line"
                } else {
                    ""
                }
            ),
            Self::WaiverUnlisted { page } => write!(
                f,
                "{FILE}: `{page}` waives fences of a page the nav does not list"
            ),
            Self::NotIncluded { file } => write!(f, "{file}: no page in the nav includes it"),
        }
    }
}

/// Every problem of a book's frame, in nav order (see the module's docs)
///
/// `nav` is checked against `checks`, each page as `read` gives it (`Ok(None)`: no such page; `Err`: why it does
/// not read), and `generated` holds the files under [`GENERATED_DIR`] as pages include them.
pub fn check(
    checks: &Checks,
    nav: &Nav,
    mut read: impl FnMut(&str) -> Result<Option<Page>, String>,
    generated: &[String],
) -> Vec<Problem> {
    let mut problems = sections(checks, nav);
    let parts: BTreeMap<&str, Level> = checks
        .part
        .iter()
        .map(|part| (part.title.as_str(), part.level))
        .collect();
    let mut seen = BTreeSet::new();
    let mut previous: Option<(Level, &str)> = None;
    let mut included = BTreeSet::new();
    for NavPage { page, part } in &nav.pages {
        if !seen.insert(page.as_str()) {
            problems.push(Problem::Twice { page: page.clone() });
            continue;
        }
        let found = match read(page) {
            Ok(Some(found)) => found,
            Ok(None) => {
                problems.push(Problem::Unreadable {
                    page: page.clone(),
                    why: None,
                });
                continue;
            }
            Err(why) => {
                problems.push(Problem::Unreadable {
                    page: page.clone(),
                    why: Some(why),
                });
                continue;
            }
        };
        included.extend(includes(&found.text).map(str::to_owned));
        let level = match front_matter_level(&found.text) {
            Some(Ok(level)) => level,
            value => {
                problems.push(Problem::Level {
                    page: page.clone(),
                    value: value.and_then(Result::err),
                });
                continue;
            }
        };
        if let Some(&part_level) = parts.get(part.as_str()) {
            if level != part_level {
                problems.push(Problem::WrongPart {
                    page: page.clone(),
                    level,
                    part: part.clone(),
                    part_level,
                });
            }
            if let Some((before_level, before)) = previous
                && level < before_level
            {
                problems.push(Problem::StepDown {
                    page: page.clone(),
                    level,
                    before: before.to_owned(),
                    before_level,
                });
            }
            previous = Some((level, page));
        }
        problems.extend(fences(checks, page, level, &found));
    }
    for page in checks.bare_fences.keys() {
        if !seen.contains(page.as_str()) {
            problems.push(Problem::WaiverUnlisted { page: page.clone() });
        }
    }
    for file in generated {
        if !included.contains(file) {
            problems.push(Problem::NotIncluded { file: file.clone() });
        }
    }
    problems
}

/// The top-level sections against the ramp and the shelves
fn sections(checks: &Checks, nav: &Nav) -> Vec<Problem> {
    let mut problems = Vec::new();
    let is_part = |s: &str| checks.part.iter().any(|p| p.title == s);
    let is_shelf = |s: &str| checks.outside.iter().any(|o| o == s);
    let ramp: Vec<String> = checks.part.iter().map(|p| p.title.clone()).collect();
    let in_ramp: Vec<String> = nav
        .sections
        .iter()
        .filter(|s| is_part(s))
        .cloned()
        .collect();
    if in_ramp != ramp {
        problems.push(Problem::RampOrder {
            nav: in_ramp,
            checks: ramp,
        });
    }
    if let Some(after) = nav.sections.iter().position(|s| is_shelf(s))
        && let Some(late) = nav.sections[after..].iter().find(|s| is_part(s))
    {
        problems.push(Problem::PartAfterShelf {
            part: late.clone(),
            shelf: nav.sections[after].clone(),
        });
    }
    for section in &nav.sections {
        if !is_part(section) && !is_shelf(section) {
            problems.push(Problem::UnknownSection {
                section: section.clone(),
            });
        }
    }
    problems
}

/// A page's bare `rust` fences against its waived count
fn fences(checks: &Checks, page: &str, level: Level, found: &Page) -> Option<Problem> {
    let lines: Vec<usize> = bare_rust_fences(&found.text)
        .into_iter()
        .filter(|fence| level != Level::Nomicon || !fence.sketch)
        .map(|fence| fence.line)
        .collect();
    let allowed = checks.bare_fences.get(page).copied().unwrap_or(0);
    if lines.len() > allowed {
        Some(Problem::BareFences {
            page: page.to_owned(),
            level,
            lines,
            allowed,
        })
    } else if lines.len() < allowed && !found.copied {
        Some(Problem::WaivedTooMany {
            page: page.to_owned(),
            count: lines.len(),
            allowed,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHECKS: &str = r#"
outside = ["Reference"]
[[part]]
title = "Basic"
level = "basic"
[[part]]
title = "Advanced"
level = "advanced"
"#;

    fn page(level: &str, body: &str) -> String {
        format!("---\nlevel: {level}\n---\n\n# Page\n\n{body}")
    }

    /// A book of `nav` (a TOML array) with pages a and c basic, b advanced, then `pages` over them
    fn problems(nav: &str, checks_extra: &str, pages: &[(&str, String)]) -> Vec<String> {
        problems_of(nav, &format!("{CHECKS}\n{checks_extra}"), pages, &[], &[])
    }

    fn problems_of(
        nav: &str,
        checks: &str,
        pages: &[(&str, String)],
        copied: &[&str],
        generated: &[&str],
    ) -> Vec<String> {
        let checks = Checks::parse(checks).unwrap();
        let table: toml::Table = toml::from_str(&format!("nav = {nav}")).unwrap();
        let nav = Nav::from_toml(table["nav"].as_array().unwrap()).unwrap();
        let mut all: BTreeMap<&str, String> = [
            ("a.md", page("basic", "")),
            ("b.md", page("advanced", "")),
            ("c.md", page("basic", "")),
        ]
        .into_iter()
        .collect();
        all.extend(pages.iter().cloned());
        let generated: Vec<String> = generated.iter().map(|g| (*g).to_owned()).collect();
        let read = |p: &str| {
            Ok(all.get(p).map(|text| Page {
                text: text.clone(),
                copied: copied.contains(&p),
            }))
        };
        check(&checks, &nav, read, &generated)
            .iter()
            .map(Problem::to_string)
            .collect()
    }

    #[test]
    fn a_nav_in_level_order_passes() {
        let nav = r#"[{ "Basic" = ["a.md", { "Sub" = [{ "C" = "c.md" }] }, { "Site" = "https://example.com/" }] },
               { "Advanced" = ["b.md"] }, { "Reference" = ["x.md"] }]"#;
        assert_eq!(
            problems(nav, "", &[("x.md", page("basic", ""))]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_step_down_fails() {
        let nav = r#"[{ "Basic" = ["a.md", "b.md", "c.md"] }, { "Advanced" = [] }]"#;
        assert_eq!(
            problems(nav, "", &[]),
            [
                "b.md: an advanced page in part `Basic`, whose pages are basic",
                "c.md: the nav steps down from advanced (b.md) to basic",
            ]
        );
    }

    #[test]
    fn a_page_without_level_twice_or_missing_fails() {
        let nav = r#"[{ "Basic" = ["a.md", "plain.md", "odd.md", "a.md", "gone.md"] }, { "Advanced" = ["b.md"] }]"#;
        assert_eq!(
            problems(
                nav,
                "",
                &[
                    ("plain.md", "# Plain\n".to_owned()),
                    ("odd.md", page("expert", ""))
                ]
            ),
            [
                "plain.md: no `level:` in its front matter",
                "odd.md: `level: expert` is none of basic, advanced, nomicon",
                "a.md: listed twice in the nav",
                "gone.md: in the nav, but no such page",
            ]
        );
    }

    #[test]
    fn parts_out_of_order_or_unknown_fail() {
        let nav = r#"[{ "Advanced" = ["b.md"] }, { "Reference" = ["c.md"] }, { "Basic" = ["a.md"] }, { "Extra" = [] }]"#;
        assert_eq!(
            problems(nav, "", &[]),
            [
                r#"the nav's ramp parts are ["Advanced", "Basic"], book_checks.toml names ["Basic", "Advanced"] in that order"#,
                "ramp part `Basic` comes after a shelf outside the ramp (`Reference`)",
                "top-level section `Extra` is neither a ramp part nor a shelf outside it in book_checks.toml",
                "a.md: the nav steps down from advanced (b.md) to basic",
            ]
        );
    }

    #[test]
    fn a_bare_fence_fails_and_an_include_passes() {
        let nav = r#"[{ "Basic" = ["a.md", "c.md"] }, { "Advanced" = ["b.md"] }]"#;
        let pages = [
            (
                "a.md",
                page(
                    "basic",
                    "```rust\n--8<-- \"examples/x/src/lib.rs:part\"\n```\n",
                ),
            ),
            (
                "c.md",
                page("basic", "Text.\n\n```rust\nfn main() {}\n```\n"),
            ),
        ];
        assert_eq!(
            problems(nav, "", &pages),
            [
                "c.md: 1 `rust` fences not an include of a compiled source (`--8<-- \"path:section\"`), 0 waived in book_checks.toml; lines 9"
            ]
        );
    }

    #[test]
    fn waived_fences_may_only_go_down() {
        let nav = r#"[{ "Basic" = ["a.md", "c.md"] }, { "Advanced" = ["b.md"] }]"#;
        let pages = [
            ("a.md", page("basic", "```rust\nfn a() {}\n```\n")),
            ("c.md", page("basic", "```rust\nfn c() {}\n```\n")),
        ];
        assert_eq!(
            problems(
                nav,
                "[bare_fences]\n\"a.md\" = 1\n\"c.md\" = 2\n\"b.md\" = 1\n\"gone.md\" = 1\n",
                &pages
            ),
            [
                "c.md: 1 bare `rust` fences, 2 waived: lower its count in book_checks.toml to 1",
                "b.md: 0 bare `rust` fences, 1 waived: lower its count in book_checks.toml to 0, that is, drop the line",
                "book_checks.toml: `gone.md` waives fences of a page the nav does not list",
            ]
        );
    }

    #[test]
    fn a_copied_page_may_hold_fewer_fences_than_waived() {
        let nav = r#"[{ "Basic" = ["a.md", "c.md"] }, { "Advanced" = ["b.md"] }]"#;
        let checks = format!("{CHECKS}\n[bare_fences]\n\"c.md\" = 2\n");
        let pages = [("c.md", page("basic", "```rust\nfn c() {}\n```\n"))];
        assert_eq!(
            problems_of(nav, &checks, &pages, &["c.md"], &[]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_nomicon_page_may_show_marked_sketches_only() {
        let checks = "outside = []\n[[part]]\ntitle = \"Nomicon\"\nlevel = \"nomicon\"\n";
        let pages = [(
            "n.md",
            page(
                "nomicon",
                "```rust title=\"sketch: the loop\"\nloop {}\n```\n\n```rust\nfn bare() {}\n```\n",
            ),
        )];
        assert_eq!(
            problems_of(r#"[{ "Nomicon" = ["n.md"] }]"#, checks, &pages, &[], &[]),
            [
                "n.md: 1 `rust` fences neither an include nor a sketch (`title=\"sketch: ...\"`), 0 waived in book_checks.toml; lines 11"
            ]
        );
    }

    #[test]
    fn a_generated_file_no_page_includes_fails() {
        let nav = r#"[{ "Basic" = ["a.md", "c.md"] }, { "Advanced" = ["b.md"] }]"#;
        let pages = [(
            "a.md",
            page(
                "basic",
                "```text\n--8<-- \"docs/generated/ex/used.txt\"\n```\n",
            ),
        )];
        assert_eq!(
            problems_of(
                nav,
                CHECKS,
                &pages,
                &[],
                &["docs/generated/ex/orphan.txt", "docs/generated/ex/used.txt"]
            ),
            ["docs/generated/ex/orphan.txt: no page in the nav includes it"]
        );
    }

    #[test]
    fn an_unreadable_page_says_why() {
        let checks = Checks::parse(CHECKS).unwrap();
        let nav = Nav {
            sections: vec!["Basic".to_owned(), "Advanced".to_owned()],
            pages: vec![NavPage {
                page: "a.md".to_owned(),
                part: "Basic".to_owned(),
            }],
        };
        let got = check(&checks, &nav, |_| Err("not UTF-8".to_owned()), &[]);
        assert_eq!(
            got.iter().map(Problem::to_string).collect::<Vec<_>>(),
            ["a.md: in the nav, but unreadable: not UTF-8"]
        );
    }

    #[test]
    fn fences_inside_a_longer_fence_are_its_text() {
        let text =
            "````text\n```rust\nfn shown() {}\n```\n````\n\n  ~~~ {.rust}\n  x\n  ~~~\n\n```rs\n";
        assert_eq!(
            bare_rust_fences(text),
            [
                BareFence {
                    line: 7,
                    sketch: false
                },
                BareFence {
                    line: 11,
                    sketch: false
                }
            ]
        );
    }

    #[test]
    fn front_matter_level_is_read_from_the_block_only() {
        assert_eq!(
            front_matter_level("---\nicon: x\nlevel: advanced\n---\n"),
            Some(Ok(Level::Advanced))
        );
        assert_eq!(front_matter_level("# Title\nlevel: basic\n"), None);
        assert_eq!(front_matter_level("---\n---\nlevel: basic\n"), None);
        assert_eq!(
            front_matter_level("---\nlevel: expert\n---\n"),
            Some(Err("expert".to_owned()))
        );
    }

    #[test]
    fn a_malformed_nav_is_refused() {
        let nav = |text: &str| {
            let table: toml::Table = toml::from_str(&format!("nav = {text}")).unwrap();
            Nav::from_toml(table["nav"].as_array().unwrap())
        };
        assert_eq!(nav(r#"["a.md"]"#), Err(NavError::NotASection { index: 1 }));
        assert_eq!(
            nav(r#"[{ "A" = [1] }]"#),
            Err(NavError::Entry {
                part: "A".to_owned()
            })
        );
        assert_eq!(
            nav(r#"[{ "A" = [{ "x" = "x.md", "y" = "y.md" }] }]"#),
            Err(NavError::Entry {
                part: "A".to_owned()
            })
        );
        let deep = format!(
            "[{{ \"A\" = {}\"a.md\"{} }}]",
            "[".repeat(NAV_DEPTH_MAX),
            "]".repeat(NAV_DEPTH_MAX)
        );
        assert_eq!(
            nav(&deep),
            Err(NavError::TooDeep {
                part: "A".to_owned()
            })
        );
        let many = format!(
            "[{{ \"A\" = [{}] }}]",
            vec!["\"p.md\""; NAV_ENTRIES_MAX].join(", ")
        );
        assert_eq!(nav(&many), Err(NavError::TooMany));
    }
}
