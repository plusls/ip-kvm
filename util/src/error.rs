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
    #[error("IO error when processing {process_info}\nCause: {source}\nBacktrace: {backtrace}")]
    Io {
        source: std::io::Error,
        process_info: String,
        backtrace: Backtrace,
    },
    #[error("Error: {msg}\nBacktrace: {backtrace}")]
    Custom { msg: String, backtrace: Backtrace },
}

impl ErrorKind {
    pub fn io<P: AsRef<Path>>(err: std::io::Error, process_info: P) -> Self {
        Self::Io {
            source: err,
            process_info: format!("{}", process_info.as_ref().display()),
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
