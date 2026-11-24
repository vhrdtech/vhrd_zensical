/// Found through its `#[path]`.
pub trait Sink {
    /// How many sides.
    const SIDES: usize;
    /// Puts bytes.
    fn put(&mut self, b: &[u8]);
    /// Flushes; by default nothing.
    fn flush(&mut self) {}
}
