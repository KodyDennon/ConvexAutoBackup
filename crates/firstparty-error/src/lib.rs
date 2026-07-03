#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

use std::{error::Error as StdError, fmt};

#[macro_export]
macro_rules! error {
    ($message:literal $(,)?) => {
        $crate::Error::message(format!($message))
    };
    ($($arg:tt)*) => {
        $crate::Error::message(format!($($arg)*))
    };
}

#[derive(Debug)]
pub enum Error {
    Message(String),
    Source {
        message: String,
        source: Box<dyn StdError + Send + Sync>,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::Source {
            message: message.into(),
            source: Box::new(source),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(message) => formatter.write_str(message),
            Self::Source { message, .. } => formatter.write_str(message),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Message(_) => None,
            Self::Source { source, .. } => Some(source.as_ref()),
        }
    }
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Self::Message(message.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Self::with_source(source.to_string(), source)
    }
}

impl From<std::env::VarError> for Error {
    fn from(source: std::env::VarError) -> Self {
        Self::with_source(source.to_string(), source)
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::ParseError> for Error {
    fn from(source: chrono::ParseError) -> Self {
        Self::with_source(source.to_string(), source)
    }
}

#[cfg(feature = "reqwest")]
impl From<reqwest::Error> for Error {
    fn from(source: reqwest::Error) -> Self {
        Self::with_source(source.to_string(), source)
    }
}

#[cfg(feature = "reqwest")]
impl From<reqwest::header::InvalidHeaderValue> for Error {
    fn from(source: reqwest::header::InvalidHeaderValue) -> Self {
        Self::with_source(source.to_string(), source)
    }
}

#[cfg(feature = "rusqlite")]
impl From<rusqlite::Error> for Error {
    fn from(source: rusqlite::Error) -> Self {
        Self::with_source(source.to_string(), source)
    }
}

#[cfg(feature = "serde_json")]
impl From<serde_json::Error> for Error {
    fn from(source: serde_json::Error) -> Self {
        Self::with_source(source.to_string(), source)
    }
}

#[cfg(feature = "uuid")]
impl From<uuid::Error> for Error {
    fn from(source: uuid::Error) -> Self {
        Self::with_source(source.to_string(), source)
    }
}

pub trait ResultContext<T> {
    fn context(self, message: impl Into<String>) -> Result<T>;
    fn with_context(self, message: impl FnOnce() -> String) -> Result<T>;
}

impl<T, E> ResultContext<T> for std::result::Result<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn context(self, message: impl Into<String>) -> Result<T> {
        self.map_err(|source| Error::with_source(message, source))
    }

    fn with_context(self, message: impl FnOnce() -> String) -> Result<T> {
        self.map_err(|source| Error::with_source(message(), source))
    }
}

impl<T> ResultContext<T> for Option<T> {
    fn context(self, message: impl Into<String>) -> Result<T> {
        self.ok_or_else(|| Error::message(message))
    }

    fn with_context(self, message: impl FnOnce() -> String) -> Result<T> {
        self.ok_or_else(|| Error::message(message()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_stable_context_message() {
        let err = std::fs::read_to_string("/path/that/does/not/exist")
            .context("failed to read config")
            .unwrap_err();

        assert_eq!(err.to_string(), "failed to read config");
        assert!(err.source().is_some());
    }

    #[test]
    fn option_context_creates_message_error() {
        let err = None::<u8>.context("missing value").unwrap_err();
        assert_eq!(err.to_string(), "missing value");
        assert!(err.source().is_none());
    }
}
