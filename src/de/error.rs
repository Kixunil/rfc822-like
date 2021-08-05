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

#[derive(Debug, thiserror::Error)]
pub enum ReadFileError {
    #[error("failed to open file {path} for reading")]
    Open { path: std::path::PathBuf, #[source] error: std::io::Error, },
    #[error("failed to load file {path}")]
    Load { path: std::path::PathBuf,  #[source] error: Error, },
}
