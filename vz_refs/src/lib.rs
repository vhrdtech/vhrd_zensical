//! The code index a book's code links and cards read.
//!
//! - [`workspace`]: the cargo workspace the book documents: its library targets, their source files, git's tracked
//!   files and commit.
//! - [`code_index`]: every public item of the book's libraries and every text file git tracks, as the code link
//!   extension reads them ([`vz_core::code_index::Index`]).

pub mod code_index;
pub mod workspace;
