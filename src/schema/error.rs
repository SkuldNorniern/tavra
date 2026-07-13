use std::error::Error as StdError;
use std::fmt::{self, Display, Formatter};

/// The schema document itself is malformed — distinct from a document
/// failing to match a well-formed schema (`Violation`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SchemaError {
    /// Dot-separated location within the schema document, e.g.
    /// `fields.server.fields.port`. Empty for a root-level problem.
    pub path: String,
    pub message: String,
}

impl SchemaError {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> SchemaError {
        SchemaError { path: path.into(), message: message.into() }
    }
}

impl Display for SchemaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

impl StdError for SchemaError {}

/// A document fails to match an otherwise well-formed schema.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Violation {
    /// Dot-separated key path within the document, with `[N]` for array
    /// indices, e.g. `server.tags[2]`. `<root>` for the document root.
    pub path: String,
    pub message: String,
}

impl Violation {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Violation {
        Violation { path: path.into(), message: message.into() }
    }
}

impl Display for Violation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl StdError for Violation {}
