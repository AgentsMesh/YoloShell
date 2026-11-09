#[derive(Clone)]
pub enum Kind {
    Identifier(String),
    String(String),
    Symbol(char),
}

#[derive(Clone)]
pub struct Token {
    pub kind: Kind,
    pub line: usize,
}

pub fn tokenize(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let mut result = Vec::new();
    let mut cursor = 0;
    let mut line = 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\n' {
            line += 1;
            cursor += 1;
        } else if bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        } else if bytes[cursor] == b'#' {
            cursor = bytes[cursor..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |offset| cursor + offset);
        } else if matches!(bytes[cursor], b'\'' | b'"') {
            let (value, end, added_lines) = quoted(source, cursor);
            result.push(Token {
                kind: Kind::String(value),
                line,
            });
            line += added_lines;
            cursor = end;
        } else if identifier_start(bytes[cursor]) {
            let end = take_identifier(bytes, cursor + 1);
            result.push(Token {
                kind: Kind::Identifier(source[cursor..end].to_owned()),
                line,
            });
            cursor = end;
        } else {
            result.push(Token {
                kind: Kind::Symbol(char::from(bytes[cursor])),
                line,
            });
            cursor += 1;
        }
    }
    result
}

impl Token {
    pub fn identifier(&self) -> Option<&str> {
        match &self.kind {
            Kind::Identifier(value) => Some(value),
            _ => None,
        }
    }

    pub fn string(&self) -> Option<&str> {
        match &self.kind {
            Kind::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn symbol(&self, expected: char) -> bool {
        matches!(self.kind, Kind::Symbol(value) if value == expected)
    }
}

fn quoted(source: &str, start: usize) -> (String, usize, usize) {
    let bytes = source.as_bytes();
    let quote = bytes[start];
    let triple = bytes.get(start..start + 3) == Some(&[quote, quote, quote]);
    let width = if triple { 3 } else { 1 };
    let mut cursor = start + width;
    let mut value = String::new();
    let mut lines = 0;
    while cursor < bytes.len() {
        let closed = if triple {
            bytes.get(cursor..cursor + width) == Some(&[quote, quote, quote])
        } else {
            bytes[cursor] == quote && !escaped(bytes, cursor)
        };
        if closed {
            return (value, cursor + width, lines);
        }
        if bytes[cursor] == b'\n' {
            lines += 1;
        }
        value.push(char::from(bytes[cursor]));
        cursor += 1;
    }
    (value, cursor, lines)
}

fn escaped(bytes: &[u8], cursor: usize) -> bool {
    let mut slashes = 0;
    let mut index = cursor;
    while index > 0 && bytes[index - 1] == b'\\' {
        slashes += 1;
        index -= 1;
    }
    slashes % 2 == 1
}

fn take_identifier(bytes: &[u8], mut cursor: usize) -> usize {
    while bytes
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        cursor += 1;
    }
    cursor
}

fn identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}
