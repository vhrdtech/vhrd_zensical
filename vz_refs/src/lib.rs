//! The code index a book's code links read, and everything a book's references show.
//!
//! - [`workspace`]: the cargo workspace the book documents: its library targets, their source files, git's tracked
//!   files and commit.
//! - [`code_index`]: every public item of the book's libraries and every text file git tracks, as the code link
//!   extension reads them ([`vz_core::code_index::Index`]).
//! - [`links`]: code links resolved through the index, and their cards.
//! - [`sources`]: the sources file, the repositories a book copies pages from, and their workspaces indexed.
//! - [`glossary`]: glossary terms' links and cards, and the glossary page's entries.
//! - [`abbreviations`]: the abbreviations file as `abbr` definitions on every page.
//! - [`review`]: a page's proofreading badge and its sections' review marks, from the review file.
//! - [`cards`]: what every card shares: its id, frame and parts, and the [`cards::Render`] the caller brings.
//! - [`page`]: where on its page a failing reference is written.
//! - [`assets`]: the cards' stylesheet and script, the code colours and the review marks' files, which a book registers as `extra_css` and
//!   `extra_javascript`.
//! - [`read`]: a book's files read whole, bounded.
//!
//! The Python package's Markdown extensions call these through `vhrd_zensical._vz`; they keep only what
//! Python-Markdown and Pygments need of them.

pub mod abbreviations;
pub mod assets;
pub mod cards;
pub mod code_index;
pub mod glossary;
pub mod links;
pub mod md_blocks;
pub mod page;
pub mod read;
pub mod review;
pub mod sources;
pub mod workspace;
