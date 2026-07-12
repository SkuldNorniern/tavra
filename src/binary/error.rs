use std::error::Error as StdError;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Error {
    pub offset: usize,
    pub message: String,
}

impl Error {
    pub fn new(offset: usize, message: impl Into<String>) -> Error {
        Error { offset, message: message.into() }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "byte {}: {}", self.offset, self.message)
    }
}

impl StdError for Error {}
