use std::backtrace::Backtrace;
use std::path::Path;

use thiserror::Error as ThisError;

pub type Result<T> = core::result::Result<T, Error>;

impl<E> From<E> for Error
where
    ErrorKind: From<E>,
{
    fn from(err: E) -> Self {
        Error(Box::new(ErrorKind::from(err)))
    }
}

#[derive(ThisError, Debug)]
#[error(transparent)]
pub struct Error(pub Box<ErrorKind>);

#[derive(ThisError, Debug)]
pub enum ErrorKind {
    #[error("IO error when processing file {path}\nCause: {source}\nBacktrace: {backtrace}")]
    Fs {
        source: std::io::Error,
        path: String,
        backtrace: Backtrace,
    },
    #[error("Error: {msg}\nBacktrace: {backtrace}")]
    Custom { msg: String, backtrace: Backtrace },
}

impl ErrorKind {
    pub fn fs<P: AsRef<Path>>(err: std::io::Error, path: P) -> Self {
        Self::Fs {
            source: err,
            path: format!("{}", path.as_ref().display()),
            backtrace: Backtrace::capture(),
        }
    }
    pub fn custom(msg: String) -> Self {
        Self::Custom {
            msg,
            backtrace: Backtrace::capture(),
        }
    }
}
