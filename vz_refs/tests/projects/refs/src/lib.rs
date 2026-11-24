//! A library whose items the index lists.

pub mod writer;
mod hidden;
#[path = "other/renamed.rs"]
pub mod moved;

#[cfg(test)]
mod tests;

/// The largest piece.
///
/// More text, not in the card.
pub const PIECE_BYTES_MAX: usize = 4096;

/// Says hello.
#[macro_export]
macro_rules! hello {
    () => {};
}

pub mod inline {
    /// Inside an inline module.
    pub fn deep() {}
}

pub use writer::TapeWriter as Writer;
pub use self::writer::*;

/// A word or its bytes.
pub union Bits {
    /// The word.
    pub word: u32,
    bytes: [u8; 4],
}

/// The platform, on unix.
#[cfg(unix)]
pub fn platform() {}

/// The platform, elsewhere.
#[cfg(not(unix))]
pub fn platform() {}

/// A level, as a type.
pub struct level {}

/// A level, as a function.
pub fn level() {}

#[inline] pub fn quick() {}
