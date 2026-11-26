//! What a page shows of its proofreading: a badge under its title and a mark beside each section's blocks.
//!
//! The review file ([`vz_core::review`]) is written by the book before its build; a book built without it (a CI
//! build without glossator) shows nothing. The badge reads `Proofread` when every line is proofread,
//! `Proofread 82%` below that, with `· changed since` when proofread lines changed since; its tooltip names the
//! last day, `Last proofread 26 Nov 2026`. A page with no line proofread has no badge.
//!
//! The marks are hidden until the reader turns them on ([`crate::assets::REVIEW_SCRIPT`]): `read`, `stale` and
//! `partial` on each block of a section in that state, none on an unread one.

use crate::read::{FileError, FileErrorKind};
use std::path::Path;
use vz_core::review::{Page, Review, ReviewError, State};

/// The review file at `path`, named `file` in errors; `None` when it is missing or of a format this kit does not
/// know
///
/// # Errors
///
/// The file cannot be read, or is no review file.
pub fn read(path: &Path, file: &str) -> Result<Option<Review>, FileError<ReviewError>> {
    match crate::read::format(path, file, Review::parse) {
        Err(FileError {
            kind: FileErrorKind::Read(e),
            ..
        }) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        r => r,
    }
}

/// The badge under a page's title
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Badge {
    /// What it shows: `Proofread 82% · changed since`
    pub text: String,
    /// Its tooltip: `Last proofread 26 Nov 2026`; empty when the file names no day
    pub title: String,
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The badge of `page`; `None` when no line of it is proofread
pub fn badge(page: &Page) -> Option<Badge> {
    if page.read == 0 {
        return None;
    }
    let mut text = if page.read >= page.lines {
        "Proofread".to_owned()
    } else {
        // floor, so a page short of every line never reads 100%; at least 1, since some line is read
        let percent = (u64::from(page.read) * 100 / u64::from(page.lines)).max(1);
        format!("Proofread {percent}%")
    };
    if page.stale > 0 {
        text.push_str(" · changed since");
    }
    let title = page
        .reviewed
        .as_deref()
        .and_then(vz_core::review::day)
        .and_then(|(y, m, d)| {
            let month = MONTHS.get(usize::try_from(m.checked_sub(1)?).ok()?)?;
            Some(format!("Last proofread {d} {month} {y}"))
        })
        .unwrap_or_default();
    Some(Badge { text, title })
}

/// One top-level block of a rendered page, as the extension walks them in order
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block<'a> {
    /// The page title, its first `h1`
    Title,
    /// A heading below the title, with the id the site gives it
    Heading(Option<&'a str>),
    /// Any other block
    Other,
}

/// The mark of each block of `page`, in order: `read`, `stale`, `partial`, or `None`
///
/// A block belongs to the section of the last heading before it; blocks before the first heading below the title
/// belong to the part under the title (id `null`). The title itself has no mark; nor has a section the file does
/// not list, or a heading without an id and its blocks.
pub fn marks(page: &Page, blocks: &[Block<'_>]) -> Vec<Option<&'static str>> {
    let state = |id: Option<&str>| {
        page.sections
            .iter()
            .find(|s| s.id.as_deref() == id)
            .and_then(|s| mark(s.state))
    };
    let mut current = state(None);
    blocks
        .iter()
        .map(|b| match b {
            Block::Title => None,
            Block::Heading(id) => {
                current = id.and_then(|id| state(Some(id)));
                current
            }
            Block::Other => current,
        })
        .collect()
}

fn mark(state: State) -> Option<&'static str> {
    match state {
        State::Read => Some("read"),
        State::Stale => Some("stale"),
        State::Partial => Some("partial"),
        State::Unread => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vz_core::review::Section;

    fn page(lines: u32, read: u32, stale: u32) -> Page {
        Page {
            lines,
            read,
            stale,
            reviewed: Some("2026-11-26".to_owned()),
            sections: Vec::new(),
        }
    }

    fn text(p: &Page) -> Option<String> {
        badge(p).map(|b| b.text)
    }

    #[test]
    fn the_badge_text() {
        assert_eq!(text(&page(120, 120, 0)).as_deref(), Some("Proofread"));
        assert_eq!(text(&page(100, 82, 0)).as_deref(), Some("Proofread 82%"));
        assert_eq!(text(&page(1000, 999, 0)).as_deref(), Some("Proofread 99%"));
        assert_eq!(text(&page(1000, 1, 0)).as_deref(), Some("Proofread 1%"));
        assert_eq!(
            text(&page(120, 98, 3)).as_deref(),
            Some("Proofread 81% · changed since")
        );
        assert_eq!(
            text(&page(10, 10, 2)).as_deref(),
            Some("Proofread · changed since")
        );
        assert_eq!(text(&page(10, 0, 4)), None);
        assert_eq!(text(&page(0, 0, 0)), None);
    }

    #[test]
    fn the_badge_title_names_the_day() {
        assert_eq!(
            badge(&page(2, 1, 0)).unwrap().title,
            "Last proofread 26 Nov 2026"
        );
        let mut p = page(2, 1, 0);
        p.reviewed = Some("2026-01-05".to_owned());
        assert_eq!(badge(&p).unwrap().title, "Last proofread 5 Jan 2026");
        p.reviewed = None;
        assert_eq!(badge(&p).unwrap().title, "");
    }

    #[test]
    fn each_block_takes_its_section_mark() {
        let mut p = page(50, 30, 1);
        for (id, state) in [
            (None, State::Read),
            (Some("a"), State::Partial),
            (Some("b"), State::Stale),
            (Some("c"), State::Unread),
        ] {
            p.sections.push(Section {
                id: id.map(str::to_owned),
                line: 1,
                state,
            });
        }
        let blocks = [
            Block::Other,
            Block::Title,
            Block::Other,
            Block::Heading(Some("a")),
            Block::Other,
            Block::Heading(Some("b")),
            Block::Heading(Some("c")),
            Block::Other,
            Block::Heading(None),
            Block::Other,
            Block::Heading(Some("gone")),
            Block::Other,
        ];
        assert_eq!(
            marks(&p, &blocks),
            [
                Some("read"),
                None,
                Some("read"),
                Some("partial"),
                Some("partial"),
                Some("stale"),
                None,
                None,
                None,
                None,
                None,
                None,
            ]
        );
    }

    #[test]
    fn a_missing_file_is_no_review() {
        let dir = std::env::temp_dir().join(format!("vz_refs_review_{}", std::process::id()));
        assert_eq!(read(&dir.join("review.json"), "review.json").unwrap(), None);
    }
}
