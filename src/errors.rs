use thiserror::Error;

/// Errors that can occur when reading or writing PLY files.
#[derive(Debug, Error)]
pub enum PlyError {
    /// An I/O error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// A parsing error occurred.
    #[error("Parse error: {0}")]
    Parse(String),
    /// An inconsistency was detected in the PLY data structure.
    #[error("Inconsistent property: {0}")]
    Inconsistent(String),
}

/// A specialized `Result` type for PLY operations.
pub type PlyResult<T> = Result<T, PlyError>;
