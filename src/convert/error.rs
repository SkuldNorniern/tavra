use std::error::Error as StdError;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConvertError {
    pub message: String,
}

impl ConvertError {
    pub fn new(message: impl Into<String>) -> ConvertError {
        ConvertError { message: message.into() }
    }
}

impl Display for ConvertError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl StdError for ConvertError {}
