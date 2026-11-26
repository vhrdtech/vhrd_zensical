//! What a page shows of its proofreading: a badge under its title and a mark beside each section's blocks.
//!
//! The review file ([`vz_core::review`]) is written by the book before its build; a book built without it (a CI
//! build without glossator) shows nothing. The badge reads `Read` when every line is read,
//! `Read 82%` below that, with `· changed since` when proofread lines changed since; its tooltip names the
//! last day, `Last read and signed 26 Nov 2026`. A page with no line proofread has no badge.
//!
//! The marks are hidden until the reader turns them on ([`crate::assets::REVIEW_SCRIPT`]): `read`, `stale` and
//! `partial` on each block of a section in that state, none on an unread one.

use crate::read::{FileError, FileErrorKind};
use std::path::Path;
use vz_core::review::{LineState, Page, Review, ReviewError, Section, State};

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
    /// What it shows: `Read 82% · changed since`
    pub text: String,
    /// Its tooltip: `Last read and signed 26 Nov 2026`; empty when the file names no day
    pub title: String,
    /// Lines proofread, in percent: 100 only when every line is
    // spirit-level: allow(exact-units-quantity) a whole percent shown on the badge, rounded down by design
    pub percent: u64,
    /// Whether proofread lines changed since
    pub stale: bool,
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The badge of `page`; `None` when no line of it is proofread
pub fn badge(page: &Page) -> Option<Badge> {
    if page.read == 0 {
        return None;
    }
    let percent = if page.read >= page.lines {
        100
    } else {
        // floor, so a page short of every line never reads 100%; at least 1, since some line is read
        (u64::from(page.read) * 100 / u64::from(page.lines)).clamp(1, 99)
    };
    let mut text = if percent == 100 {
        "Read".to_owned()
    } else {
        format!("Read {percent}%")
    };
    if page.stale > 0 {
        text.push_str(" · changed since");
    }
    let title = day(page.reviewed.as_deref())
        .map(|d| format!("Last read and signed {d}"))
        .unwrap_or_default();
    Some(Badge {
        text,
        title,
        percent,
        stale: page.stale > 0,
    })
}

/// `26 Nov 2026` for `2026-11-26`
fn day(reviewed: Option<&str>) -> Option<String> {
    let (y, m, d) = vz_core::review::day(reviewed?)?;
    let month = MONTHS.get(usize::try_from(m.checked_sub(1)?).ok()?)?;
    Some(format!("{d} {month} {y}"))
}

/// A section's mark on its blocks
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mark {
    /// `read`, `stale` or `partial`
    pub state: &'static str,
    /// Its tooltip: `Read and signed 26 Nov 2026`, `Partly read, last 26 Nov 2026`,
    /// `Changed since read on 26 Nov 2026`; without the day when the file names none
    pub title: String,
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
pub fn marks(page: &Page, blocks: &[Block<'_>]) -> Vec<Option<Mark>> {
    let state = |id: Option<&str>| {
        page.sections
            .iter()
            .find(|s| s.id.as_deref() == id)
            .and_then(mark)
    };
    let mut current = state(None);
    blocks
        .iter()
        .map(|b| match b {
            Block::Title => None,
            Block::Heading(id) => {
                current = id.and_then(|id| state(Some(id)));
                current.clone()
            }
            Block::Other => current.clone(),
        })
        .collect()
}

/// The mark of each top-level block of a page, from its read and stale lines
///
/// `source` is the page's Markdown, `count` the blocks it rendered to. `None` when the page has no runs (a file from before them) or when its source
/// does not split into `count` blocks (an included file, a block it does not know): the caller marks by section.
pub fn block_marks(
    page: &Page,
    source: &str,
    count: usize,
    included: impl Fn(&str) -> Option<usize>,
) -> Option<Vec<Option<Mark>>> {
    if page.runs.is_empty() && page.read + page.stale > 0 {
        return None;
    }
    let spans = crate::md_blocks::blocks_with(source, included);
    (spans.len() == count).then(|| spans.iter().map(|&(a, b)| block_mark(page, a, b)).collect())
}

/// The mark of the lines `from` to `to`: read when one read run holds them all, stale when a stale run touches them,
/// partial when a read run does
fn block_mark(page: &Page, from: usize, to: usize) -> Option<Mark> {
    let touching: Vec<_> = page
        .runs
        .iter()
        .filter(|r| r.from as usize <= to && r.to as usize >= from)
        .collect();
    let day = day(touching.iter().filter_map(|r| r.reviewed.as_deref()).max());
    let all = |s| {
        touching
            .iter()
            .any(|r| r.state == s && r.from as usize <= from && r.to as usize >= to)
    };
    let any = |s| touching.iter().any(|r| r.state == s);
    let state = if any(LineState::Stale) {
        State::Stale
    } else if all(LineState::Read) {
        State::Read
    } else if any(LineState::Read) {
        State::Partial
    } else {
        return None;
    };
    mark(&Section {
        id: None,
        line: 1,
        state,
        reviewed: None,
    })
    .map(|m| with_day(m, day))
}

fn with_day(m: Mark, day: Option<String>) -> Mark {
    let Some(d) = day else { return m };
    let title = match m.state {
        "read" => format!("Read and signed {d}"),
        "partial" => format!("Partly read, last {d}"),
        _ => format!("Changed since read on {d}"),
    };
    Mark { title, ..m }
}

fn mark(section: &Section) -> Option<Mark> {
    let day = day(section.reviewed.as_deref());
    let (state, title) = match (section.state, day) {
        (State::Read, Some(d)) => ("read", format!("Read {d}")),
        (State::Read, None) => ("read", "Read".to_owned()),
        (State::Partial, Some(d)) => ("partial", format!("Partly read, last {d}")),
        (State::Partial, None) => ("partial", "Partly read".to_owned()),
        (State::Stale, Some(d)) => ("stale", format!("Changed since read on {d}")),
        (State::Stale, None) => ("stale", "Changed since read".to_owned()),
        (State::Unread, _) => return None,
    };
    Some(Mark { state, title })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(lines: u32, read: u32, stale: u32) -> Page {
        Page {
            lines,
            read,
            stale,
            reviewed: Some("2026-11-26".to_owned()),
            sections: Vec::new(),
            runs: Vec::new(),
        }
    }

    fn text(p: &Page) -> Option<String> {
        badge(p).map(|b| b.text)
    }

    #[test]
    fn the_badge_text() {
        assert_eq!(text(&page(120, 120, 0)).as_deref(), Some("Read"));
        assert_eq!(text(&page(100, 82, 0)).as_deref(), Some("Read 82%"));
        assert_eq!(text(&page(1000, 999, 0)).as_deref(), Some("Read 99%"));
        assert_eq!(text(&page(1000, 1, 0)).as_deref(), Some("Read 1%"));
        assert_eq!(
            text(&page(120, 98, 3)).as_deref(),
            Some("Read 81% · changed since")
        );
        assert_eq!(
            text(&page(10, 10, 2)).as_deref(),
            Some("Read · changed since")
        );
        assert_eq!(text(&page(10, 0, 4)), None);
        assert_eq!(text(&page(0, 0, 0)), None);
    }

    #[test]
    fn the_badge_title_names_the_day() {
        assert_eq!(
            badge(&page(2, 1, 0)).unwrap().title,
            "Last read and signed 26 Nov 2026"
        );
        let mut p = page(2, 1, 0);
        p.reviewed = Some("2026-01-05".to_owned());
        assert_eq!(badge(&p).unwrap().title, "Last read and signed 5 Jan 2026");
        p.reviewed = None;
        assert_eq!(badge(&p).unwrap().title, "");
    }

    #[test]
    fn a_block_is_read_only_when_one_read_run_holds_it() {
        use vz_core::review::Run;
        let mut p = page(20, 8, 1);
        let run = |from, to, state, day: Option<&str>| Run {
            from,
            to,
            state,
            reviewed: day.map(str::to_owned),
        };
        p.runs = vec![
            run(1, 5, LineState::Read, Some("2026-11-26")),
            run(7, 7, LineState::Stale, None),
            run(9, 10, LineState::Read, Some("2026-03-02")),
        ];
        let state = |a, b| block_mark(&p, a, b).map(|m| m.state);
        assert_eq!(state(2, 4), Some("read"));
        assert_eq!(state(4, 6), Some("partial"));
        assert_eq!(state(6, 9), Some("stale"));
        assert_eq!(state(11, 12), None);
        assert_eq!(
            block_mark(&p, 3, 10).map(|m| m.title).as_deref(),
            Some("Changed since read on 26 Nov 2026")
        );
        let source = "# A\n\nb\n";
        assert!(block_marks(&p, source, 2, |_| None).is_some());
        assert!(
            block_marks(&p, source, 3, |_| None).is_none(),
            "a count that differs"
        );
        p.runs.clear();
        assert!(
            block_marks(&p, source, 2, |_| None).is_none(),
            "a file from before runs"
        );
    }

    #[test]
    fn a_mark_names_its_state_and_day() {
        let section = |state, reviewed: Option<&str>| Section {
            id: None,
            line: 1,
            state,
            reviewed: reviewed.map(str::to_owned),
        };
        let title = |s: &Section| mark(s).map(|m| m.title);
        assert_eq!(
            title(&section(State::Read, Some("2026-11-26"))).as_deref(),
            Some("Read 26 Nov 2026")
        );
        assert_eq!(
            title(&section(State::Partial, Some("2026-03-02"))).as_deref(),
            Some("Partly read, last 2 Mar 2026")
        );
        assert_eq!(
            title(&section(State::Stale, Some("2026-11-26"))).as_deref(),
            Some("Changed since read on 26 Nov 2026")
        );
        assert_eq!(
            title(&section(State::Partial, None)).as_deref(),
            Some("Partly read")
        );
        assert_eq!(title(&section(State::Unread, Some("2026-11-26"))), None);
    }

    #[test]
    fn the_badge_percent_is_100_only_when_every_line_is_read() {
        assert_eq!(badge(&page(10, 10, 0)).unwrap().percent, 100);
        assert_eq!(badge(&page(1000, 999, 0)).unwrap().percent, 99);
        assert!(badge(&page(10, 10, 2)).unwrap().stale);
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
                reviewed: (state == State::Read).then(|| "2026-11-26".to_owned()),
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
            marks(&p, &blocks)
                .into_iter()
                .map(|m| m.map(|m| m.state))
                .collect::<Vec<_>>(),
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
