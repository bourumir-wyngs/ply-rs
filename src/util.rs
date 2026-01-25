
/// Tracks the current line number while parsing.
///
/// This is primarily used to add line-context to I/O and parse errors.
#[derive(Debug, Clone, Copy)]
pub struct LocationTracker {
    /// Current 1-based line index in the input stream.
    pub line_index: usize,
}

impl LocationTracker {
    /// Creates a new tracker at the start of a stream.
    pub fn new() -> Self {
        LocationTracker { line_index: 0 }
    }

    /// Advances the tracker to the next line.
    pub fn next_line(&mut self) {
        self.line_index += 1;
    }
}
