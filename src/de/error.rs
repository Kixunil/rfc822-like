//! Error types related to deserialization of RFC822-like format.

use std::fmt;
use std::io;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ErrorInner {
    #[error("{0}")]
    Custom(String),
    #[error("Line {0} doesn't contain a colon")]
    MissingColon(usize),
    #[error("I/O error")]
    IoError(#[from] io::Error),
    #[error("The deserialized type is ambiguous and must be explicitly specified. (RFC822 is NOT self-describing.)")]
    AmbiguousType,
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        ErrorInner::Custom(msg.to_string()).into()
    }
}

/// Error that can happen during deserialization.
///
/// The error is currently encapsulated and not exposed because it's not yet certain what kind of
/// information we will store in the error type.
/// However, it does implement standard error-related traits and has a human-friendly `Display`
/// implementation.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(#[from] ErrorInner);

/// Error returned when opening a file and subsequent deserialization fail.
#[derive(Debug, thiserror::Error)]
pub enum ReadFileError {
    /// Variant returned when a file couldn't be opened.
    #[error("failed to open file {path} for reading")]
    Open {
        /// Path to file that was accessed.
        path: std::path::PathBuf,
        /// The reason why opening failed.
        #[source] error: std::io::Error,
    },
    /// Variant returned when read or deserialization fail.
    #[error("failed to load file {path}")]
    Load {
        /// Path to the file that could not be loaded.
        path: std::path::PathBuf,
        /// The reason why loading failed.
        #[source] error: Error,
    },
}
