use thiserror::Error;
use serde::{de, ser};
use std::fmt;

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
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialize(String),
}

impl de::Error for PlyError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        PlyError::Parse(msg.to_string())
    }
}

impl ser::Error for PlyError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        PlyError::Serialize(msg.to_string())
    }
}

/// A specialized `Result` type for PLY operations.
pub type PlyResult<T> = Result<T, PlyError>;
