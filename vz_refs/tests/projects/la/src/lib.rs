//! A second repository's library, whose page a book copies.

/// Samples of one capture, as they came.
pub struct RawChunk {
    /// The samples
    pub samples: Vec<u8>,
}

/// The transitions of a [`RawChunk`].
pub struct TransitionsChunk;
