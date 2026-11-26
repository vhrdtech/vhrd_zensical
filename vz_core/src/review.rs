//! The review file: how much of each page has been proofread, which the review extension shows.
//!
//! A book writes it before its build from `glossator book export`, which counts only valid, trusted stamps; the
//! file is already verified and holds no names, keys or signatures.
//!
//! ```json
//! {"format": 1, "pages": {"tracks.md": {"lines": 120, "read": 98, "stale": 3, "reviewed": "2026-11-26",
//!   "sections": [{"id": null, "line": 1, "state": "read"},
//!                {"id": "items-and-tracks", "line": 12, "state": "partial"}]}}}
//! ```
//!
//! - A page is keyed by its path relative to the docs folder, as Zensical names it.
//! - `lines` are the page's lines of text, `read` those proofread and unchanged since, `stale` those proofread but
//!   changed since; `reviewed` the day of the last proofreading.
//! - A section's `id` is the id the site gives its heading; `null` is the part under the page title, before the
//!   first heading below it. A section runs to the next heading of any level below the title. Sections without text
//!   are left out.
//!
//! A file of another `format` is skipped ([`Review::parse`] gives `None`), so a newer export never fails a build.
//! Readers ignore keys they do not know; `tests/fixtures/review/ok.json` is the layout's snapshot.

use crate::FILE_BYTES_MAX;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The version of the file's layout this reader knows; a file of another is skipped
pub const FORMAT: u32 = 1;

/// How far a section has been proofread
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// Every line proofread and unchanged since
    Read,
    /// Some lines proofread and unchanged since
    Partial,
    /// Proofread, then changed
    Stale,
    /// Not proofread
    Unread,
}

/// One section of a page
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    /// Its heading's id on the site; `None`: the part under the page title
    pub id: Option<String>,
    /// The line of the page's file it starts at, from 1
    pub line: u32,
    /// How far it has been proofread
    pub state: State,
}

/// One page's proofreading
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    /// Its lines of text
    pub lines: u32,
    /// Lines proofread and unchanged since
    pub read: u32,
    /// Lines proofread, then changed
    pub stale: u32,
    /// The day of its last proofreading, `YYYY-MM-DD`; `None` when it has none
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed: Option<String>,
    /// Its sections with text, in page order
    #[serde(default)]
    pub sections: Vec<Section>,
}

#[derive(Deserialize)]
struct Probe {
    format: u32,
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    pages: BTreeMap<String, Page>,
}

/// Why a text is no review file
#[derive(Debug)]
pub enum ReviewError {
    /// The text is longer than [`FILE_BYTES_MAX`]
    TooLarge,
    /// JSON syntax, a missing key, a value of the wrong type, a state other than read, partial, stale, unread
    Parse(serde_json::Error),
    /// A page key that is no relative path under the docs folder
    Page {
        /// The page as written
        page: String,
    },
    /// `read` or `stale` above `lines`
    Counts {
        /// The page
        page: String,
    },
    /// A `reviewed` that is no `YYYY-MM-DD` day
    Date {
        /// The page
        page: String,
        /// The day as written
        date: String,
    },
    /// A section id that is empty or holds white space
    Section {
        /// The page
        page: String,
        /// The id as written
        id: String,
    },
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(f, "a review file is {FILE_BYTES_MAX} bytes at most"),
            Self::Parse(e) => write!(f, "{e}"),
            Self::Page { page } => write!(
                f,
                "page \"{page}\" is no relative path under the docs folder, such as \"guide/intro.md\""
            ),
            Self::Counts { page } => write!(
                f,
                "page \"{page}\": `read` and `stale` are each at most `lines`"
            ),
            Self::Date { page, date } => write!(
                f,
                "page \"{page}\": reviewed \"{date}\" is no day such as \"2026-11-26\""
            ),
            Self::Section { page, id } => write!(
                f,
                "page \"{page}\": section id \"{id}\" is empty or holds white space"
            ),
        }
    }
}

impl std::error::Error for ReviewError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            _ => None,
        }
    }
}

/// The proofreading of a book's pages
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Review {
    /// Each page, by its path relative to the docs folder
    pub pages: BTreeMap<String, Page>,
}

impl Review {
    /// Parses a review file; `None` when it is of a format other than [`FORMAT`]
    ///
    /// # Errors
    ///
    /// The text is too large or no review file, or a page's key, counts, day or section ids are malformed.
    pub fn parse(text: &str) -> Result<Option<Self>, ReviewError> {
        if text.len() > FILE_BYTES_MAX {
            return Err(ReviewError::TooLarge);
        }
        let probe: Probe = serde_json::from_str(text).map_err(ReviewError::Parse)?;
        if probe.format != FORMAT {
            return Ok(None);
        }
        let file: File = serde_json::from_str(text).map_err(ReviewError::Parse)?;
        for (page, p) in &file.pages {
            check(page, p)?;
        }
        Ok(Some(Self { pages: file.pages }))
    }

    /// A page's proofreading; `page` as Zensical names it, relative to the docs folder (`\` and a leading `./` are
    /// read as `/` and nothing)
    pub fn page(&self, page: &str) -> Option<&Page> {
        let page = page.replace('\\', "/");
        let page = page.strip_prefix("./").unwrap_or(&page);
        self.pages.get(page)
    }
}

/// A page's own fields checked
fn check(page: &str, p: &Page) -> Result<(), ReviewError> {
    if !crate::sources::is_page(page) {
        return Err(ReviewError::Page {
            page: page.to_owned(),
        });
    }
    if p.read > p.lines || p.stale > p.lines {
        return Err(ReviewError::Counts {
            page: page.to_owned(),
        });
    }
    if let Some(date) = p.reviewed.as_deref().filter(|d| !is_day(d)) {
        return Err(ReviewError::Date {
            page: page.to_owned(),
            date: date.to_owned(),
        });
    }
    if let Some(id) = p
        .sections
        .iter()
        .filter_map(|s| s.id.as_deref())
        .find(|id| id.is_empty() || id.chars().any(char::is_whitespace))
    {
        return Err(ReviewError::Section {
            page: page.to_owned(),
            id: id.to_owned(),
        });
    }
    Ok(())
}

/// Whether `date` is `YYYY-MM-DD` with a month of 1 to 12 and a day of 1 to 31
fn is_day(date: &str) -> bool {
    let parts: Vec<&str> = date.split('-').collect();
    let [y, m, d] = parts.as_slice() else {
        return false;
    };
    let number = |s: &str, len: usize, range: std::ops::RangeInclusive<u32>| {
        s.len() == len
            && s.chars().all(|c| c.is_ascii_digit())
            && s.parse::<u32>().is_ok_and(|n| range.contains(&n))
    };
    number(y, 4, 0..=9999) && number(m, 2, 1..=12) && number(d, 2, 1..=31)
}

/// The parts of a day `YYYY-MM-DD`: year, month from 1, day of the month; `None` when it is none
pub fn day(date: &str) -> Option<(u32, u32, u32)> {
    if !is_day(date) {
        return None;
    }
    let mut parts = date.split('-').map(|s| s.parse::<u32>().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn another_format_is_skipped() {
        assert_eq!(
            Review::parse(r#"{"format": 2, "pages": "anything"}"#).unwrap(),
            None
        );
    }

    #[test]
    fn a_file_without_format_is_refused() {
        assert!(matches!(
            Review::parse(r#"{"pages": {}}"#),
            Err(ReviewError::Parse(_))
        ));
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let r = Review::parse(
            r#"{"format": 1, "by": "x", "pages": {"a.md": {"lines": 2, "read": 1, "stale": 0, "new": [], "sections": []}}}"#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r.page("./a.md").unwrap().read, 1);
        assert_eq!(r.page("a.md").unwrap().reviewed, None);
    }

    #[test]
    fn days() {
        assert_eq!(day("2026-11-26"), Some((2026, 11, 26)));
        for bad in [
            "2026-13-01",
            "2026-11-00",
            "26-11-2026",
            "2026-1-01",
            "2026-11-26x",
            "",
            "2026-+1-01",
        ] {
            assert_eq!(day(bad), None, "{bad}");
        }
    }
}
