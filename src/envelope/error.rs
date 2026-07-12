use std::error::Error as StdError;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Error {
        Error { message: message.into() }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl StdError for Error {}
