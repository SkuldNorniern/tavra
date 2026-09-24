use crate::limits::Limits;
use crate::text::error::Error;

/// Char-indexed cursor over source text, tracking 1-based line/column for
/// error reporting.
pub struct Cursor {
    chars: Vec<char>,
    idx: usize,
    line: u32,
    col: u32,
    depth: u32,
    max_depth: u32,
}


impl Cursor {
    pub fn new(input: &str) -> Cursor {
        // Skip a leading BOM.
        let chars: Vec<char> = input.chars().collect();
        let chars = if chars.first() == Some(&'\u{FEFF}') { chars[1..].to_vec() } else { chars };
        Cursor { chars, idx: 0, line: 1, col: 1, depth: 0, max_depth: Limits::default().depth() }
    }

    pub fn with_limits(input: &str, limits: &Limits) -> Cursor {
        Cursor { max_depth: limits.depth(), ..Cursor::new(input) }
    }

    pub fn enter(&mut self) -> Result<(), Error> {
        self.depth += 1;
        if self.depth > self.max_depth {
            return Err(self.error(format!("nested deeper than {}", self.max_depth)));
        }
        Ok(())
    }

    pub fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
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
