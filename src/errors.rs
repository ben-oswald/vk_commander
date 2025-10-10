use crate::state::{Event, Message};
use std::env::VarError;
use std::fmt::Debug;
use std::num::ParseIntError;
use std::sync::mpsc::{SendError, Sender};
use std::sync::{Arc, PoisonError, RwLockReadGuard, RwLockWriteGuard, TryLockError};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    SystemTime(std::time::SystemTimeError),
    Internal(SendError<Message>),
    Network(String),
    Critical(String),
    Any(String),
    InvalidInput(String),
}

impl AsRef<Error> for Error {
    fn as_ref(&self) -> &Error {
        self
    }
}

impl From<Box<Error>> for Error {
    fn from(value: Box<Error>) -> Self {
        *value
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<std::io::Error> for Box<Error> {
    fn from(value: std::io::Error) -> Self {
        Box::new(Error::Io(value))
    }
}

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Error::Any(e.to_owned())
    }
}

impl From<String> for Error {
    fn from(e: String) -> Self {
        Error::Any(e)
    }
}

impl From<std::time::SystemTimeError> for Error {
    fn from(e: std::time::SystemTimeError) -> Self {
        Error::SystemTime(e)
    }
}

impl From<SendError<Message>> for Error {
    fn from(value: SendError<Message>) -> Self {
        Error::Internal(value)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Error::Any(e.to_string())
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Error::Any(value.to_string())
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(value: PoisonError<T>) -> Self {
        Self::Any(value.to_string())
    }
}

impl<T> From<PoisonError<T>> for Box<Error> {
    fn from(value: PoisonError<T>) -> Self {
        Box::new(Error::Any(value.to_string()))
    }
}

impl<T> From<TryLockError<RwLockReadGuard<'_, T>>> for Error {
    fn from(value: TryLockError<RwLockReadGuard<'_, T>>) -> Self {
        Self::Any(value.to_string())
    }
}

impl<T> From<TryLockError<RwLockReadGuard<'_, T>>> for Box<Error> {
    fn from(value: TryLockError<RwLockReadGuard<'_, T>>) -> Self {
        Box::new(Error::Any(value.to_string()))
    }
}

impl<T> From<TryLockError<RwLockWriteGuard<'_, T>>> for Error {
    fn from(value: TryLockError<RwLockWriteGuard<'_, T>>) -> Self {
        Self::Any(value.to_string())
    }
}

impl<T> From<TryLockError<RwLockWriteGuard<'_, T>>> for Box<Error> {
    fn from(value: TryLockError<RwLockWriteGuard<'_, T>>) -> Self {
        Box::new(Error::Any(value.to_string()))
    }
}

impl From<VarError> for Error {
    fn from(value: VarError) -> Self {
        Self::Any(value.to_string())
    }
}

impl Error {
    pub fn error_type(&self) -> String {
        match self {
            Error::Io(_) => "I/O error".into(),
            Error::Network(_) => "Network error".into(),
            Error::Critical(_) => "Critical Error".into(),
            Error::Any(_) => "Error".into(),
            Error::Internal(_) => "Internal Error".into(),
            Error::SystemTime(_) => "System Time Error".into(),
            Error::InvalidInput(_) => "Invalid input".into(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Internal(e) => {
                write!(f, "{e}")
            }
            Error::Any(e) => write!(f, "{e}"),
            Error::Network(e) => write!(f, "{e}"),
            Error::Critical(e) => write!(f, "{e}"),
            Error::SystemTime(e) => {
                write!(f, "{e}")
            }
            Error::InvalidInput(e) => {
                write!(f, "{e}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Internal(e) => Some(e),
            Error::SystemTime(e) => Some(e),
            Error::Any(_) => None,
            Error::Network(_) => None,
            Error::Critical(_) => None,
            Error::InvalidInput(_) => None,
        }
    }
}

impl Error {
    pub fn show_error_dialog(self, sender: Arc<Sender<Message>>) {
        sender
            .send(Message::Event(Arc::from(Event::ShowError(Error::from(
                self.to_string(),
            )))))
            .unwrap_or_else(|e| {
                Error::from(e).log_error();
            });
    }
    pub fn log_error(self) {
        eprintln!("Failed to send error message: {}", self);
    }
}
