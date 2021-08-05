#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error {
    internal: ErrorInternal,
}

impl From<ErrorInternal> for Error {
    fn from(value: ErrorInternal) -> Self {
        Error {
            internal: value,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Unsupported data type {unsupported}")]
pub(crate) enum ErrorInternal {
    #[error("unsupported data type {0}")]
    Unsupported(&'static str),
    #[error("{0}")]
    Custom(String),
    #[error("invalid char {c} in key '{key}' at position {pos}")]
    InvalidKeyChar { key: String, c: char, pos: usize },
    #[error("empty key is not allowed")]
    EmptyKey,
    #[error("failed to write")]
    FmtWriteFailed,
    #[error("failed to write")]
    IoWriteFailed(#[from] std::io::Error),
}

impl Error {
    pub(crate) fn unsupported_data_type(type_name: &'static str) -> Self {
        let type_name = if type_name.starts_with("serialize_") {
            &type_name[10..]
        } else {
            type_name
        };

        ErrorInternal::Unsupported(type_name).into()
    }

    pub(crate) fn failed_write(_: std::fmt::Error) -> Self {
        ErrorInternal::FmtWriteFailed.into()
    }

    pub(crate) fn to_fmt(self) -> Result<Result<(), Self>, std::fmt::Error> {
        if let ErrorInternal::FmtWriteFailed = self.internal {
            Err(std::fmt::Error)
        } else {
            Ok(Err(self))
        }
    }
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        ErrorInternal::Custom(msg.to_string()).into()
    }
}

