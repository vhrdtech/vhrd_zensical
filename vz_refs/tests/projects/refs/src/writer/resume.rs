use super::TapeWriter;

impl TapeWriter {
    /// Reopen the unfinished piece and go on writing it.
    ///
    /// Second paragraph.
    #[must_use]
    pub fn resume_on(self, at: u64) -> Self {
        let _ = at;
        // --8<-- [start:call]
        let a = 1;
        let b = 2;
        // --8<-- [end:call]
        let c = 3;
        let d = 4;
        let e = 5;
        let f = 6;
        let g = 7;
        let h = 8;
        let i = 9;
        let j = 10;
        let k = 11;
        let _ = (a, b, c, d, e, f, g, h, i, j, k);
        self
    }

    fn helper(&self) {}
}

impl Default for TapeWriter {
    fn default() -> Self {
        Self { written: 0, kept: 0 }
    }
}
