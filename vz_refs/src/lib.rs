//! The code index a book's code links and cards read, and the cards' assets.
//!
//! - [`workspace`]: the cargo workspace the book documents: its library targets, their source files, git's tracked
//!   files and commit.
//! - [`code_index`]: every public item of the book's libraries and every text file git tracks, as the code link
//!   extension reads them ([`vz_core::code_index::Index`]).
//! - [`assets`]: the cards' stylesheet and script and the code colours, which a book registers as `extra_css` and
//!   `extra_javascript`.

pub mod assets;
pub mod code_index;
pub mod workspace;
