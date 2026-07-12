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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for Error {}
