//! Writing.

pub mod resume;

/// Writes a tape.
pub struct TapeWriter {
    /// Bytes written so far.
    pub written: u64,
    kept: u64,
}

/// What a write ends in.
pub enum Outcome {
    /// All of it.
    Done,
    /// Some of it.
    Partial(u64),
}

/// Not public: left out.
fn private() {}
