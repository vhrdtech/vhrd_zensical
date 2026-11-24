//! The book kit's formats, shared by the `vz` command, the Python extensions and the book lints of `spl`.
//!
//! - [`glossary`]: `docs/glossary.toml`, the terms prose links to.
//! - [`abbreviations`]: `docs/abbreviations.toml`, the long forms shown on hover.
//! - [`ext`]: the Markdown extensions' ids in `zensical.toml`, their settings and default paths, read through
//!   [`ext::ExtConfig`] so each caller brings its own configuration tree.
//! - [`code_index`]: the code index's JSON types and their layout version [`code_index::FORMAT`].
//! - [`stamp`]: the footer build stamp's markers and default paths.
//!
//! Types and parsers only: the caller reads the files, bounded by [`FILE_BYTES_MAX`]. No I/O, threads or async
//! runtime.

pub mod abbreviations;
pub mod code_index;
pub mod ext;
pub mod glossary;
pub mod stamp;

/// Bytes of a glossary or abbreviations file at most; a caller reads at most one byte more, and a longer text
/// is refused by the parser
// spirit-level: allow(exact-units-quantity) vz_core depends on serde and toml only; a caller passes it to take()
pub const FILE_BYTES_MAX: usize = 1 << 20;
