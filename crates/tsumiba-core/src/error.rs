use std::fmt;
use std::io;

/// Everything that can go wrong in the storage layer.
#[derive(Debug)]
pub enum Error {
    /// A filesystem operation failed. `context` says which one and on what.
    Io { context: String, source: io::Error },
    /// The caller supplied something that cannot be stored.
    Invalid(String),
    /// No note or file has that id.
    NotFound(String),
    /// A payload exceeded the configured size limit.
    TooLarge { limit: u64 },
    /// The configuration file exists but cannot be understood.
    Config(String),
    /// The index file exists but cannot be understood.
    Index(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { context, source } => write!(f, "{context}: {source}"),
            Error::Invalid(msg) => write!(f, "{msg}"),
            Error::NotFound(what) => write!(f, "not found: {what}"),
            Error::TooLarge { limit } => {
                write!(f, "too large: the limit is {} MiB", limit / (1024 * 1024))
            }
            Error::Config(msg) => write!(f, "config: {msg}"),
            Error::Index(msg) => write!(f, "index: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Attach a human-readable context to an [`io::Error`].
pub(crate) trait IoContext<T> {
    fn context(self, what: impl FnOnce() -> String) -> Result<T>;
}

impl<T> IoContext<T> for io::Result<T> {
    fn context(self, what: impl FnOnce() -> String) -> Result<T> {
        self.map_err(|source| Error::Io {
            context: what(),
            source,
        })
    }
}
