use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column_utf16: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionError;

impl fmt::Display for PositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("offset is outside the source, inside a UTF-8 character, or inside CRLF")
    }
}

impl std::error::Error for PositionError {}

#[derive(Debug)]
pub struct Source {
    text: String,
    line_starts: Vec<usize>,
}

impl Source {
    pub fn new(text: String) -> Self {
        let mut line_starts = vec![0];
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\r' => {
                    i += 1;
                    if bytes.get(i) == Some(&b'\n') {
                        i += 1;
                    }
                    line_starts.push(i);
                }
                b'\n' => {
                    i += 1;
                    line_starts.push(i);
                }
                _ => i += 1,
            }
        }
        Self { text, line_starts }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn position(&self, byte: usize) -> Result<Position, PositionError> {
        if !self.text.is_char_boundary(byte)
            || (byte > 0
                && self.text.as_bytes().get(byte) == Some(&b'\n')
                && self.text.as_bytes()[byte - 1] == b'\r')
        {
            return Err(PositionError);
        }
        let line = self.line_starts.partition_point(|&start| start <= byte) - 1;
        Ok(Position {
            line: line + 1,
            column_utf16: utf16_len(&self.text[self.line_starts[line]..byte]) + 1,
        })
    }
}

pub fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}
