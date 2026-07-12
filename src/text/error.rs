use std::error::Error as StdError;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Error {
    pub line: u32,
    pub col: u32,
    pub message: String,
}

impl Error {
    pub fn new(line: u32, col: u32, message: impl Into<String>) -> Error {
        Error { line, col, message: message.into() }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.message)
    }
}

impl StdError for Error {}
