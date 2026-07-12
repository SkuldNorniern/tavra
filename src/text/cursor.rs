use super::error::Error;

/// Char-indexed cursor over source text, tracking 1-based line/column for
/// error reporting.
pub struct Cursor {
    chars: Vec<char>,
    idx: usize,
    line: u32,
    col: u32,
}

impl Cursor {
    pub fn new(input: &str) -> Cursor {
        // Skip a leading BOM.
        let chars: Vec<char> = input.chars().collect();
        let chars = if chars.first() == Some(&'\u{FEFF}') { chars[1..].to_vec() } else { chars };
        Cursor { chars, idx: 0, line: 1, col: 1 }
    }

    pub fn peek(&self) -> Option<char> {
        self.chars.get(self.idx).copied()
    }

    pub fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.idx + n).copied()
    }

    pub fn eof(&self) -> bool {
        self.idx >= self.chars.len()
    }

    pub fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.idx += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    pub fn error(&self, message: impl Into<String>) -> Error {
        Error::new(self.line, self.col, message)
    }
}
