use std::fmt;

/// Errors produced by this crate.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DracoError {
    /// Input validation failed.
    InvalidInput(String),
    /// Encoding failed in the underlying Draco wrapper.
    EncodeFailed(String),
    /// Decoding failed in the underlying Draco wrapper.
    DecodeFailed(String),
}

impl fmt::Display for DracoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DracoError::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
            DracoError::EncodeFailed(msg) => write!(f, "Encode failed: {msg}"),
            DracoError::DecodeFailed(msg) => write!(f, "Decode failed: {msg}"),
        }
    }
}

impl std::error::Error for DracoError {}
