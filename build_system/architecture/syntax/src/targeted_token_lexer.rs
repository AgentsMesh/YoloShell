use super::literals;
use super::{Kind, Token};

const FIRST_LINE: usize = 1;
const DECIMAL_RADIX: u32 = 10;
const ASCII_SYMBOL_WIDTH: usize = 1;

pub fn tokenize(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    let mut line = FIRST_LINE;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\n' {
            line += 1;
            cursor += 1;
        } else if bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        } else if let Some(end) =
            literals::push_raw_string(source, cursor, line, &mut line, &mut tokens)
        {
            cursor = end;
        } else if starts(bytes, cursor, b"//") {
            cursor = skip_line(bytes, cursor);
        } else if starts(bytes, cursor, b"/*") {
            cursor = literals::skip_block_comment(bytes, cursor, &mut line);
        } else if bytes[cursor] == b'"' {
            cursor = literals::push_string(source, cursor, line, &mut line, &mut tokens);
        } else if bytes[cursor] == b'\'' && !literals::is_lifetime(bytes, cursor) {
            cursor = literals::skip_quoted(bytes, cursor, &mut line);
        } else if let Some((identifier, end)) = literals::raw_identifier(source, cursor) {
            tokens.push(Token {
                kind: Kind::Identifier(identifier.to_owned()),
                line,
            });
            cursor = end;
        } else if identifier_start(bytes[cursor]) {
            let end = take_while(bytes, cursor + 1, identifier_continue);
            tokens.push(Token {
                kind: Kind::Identifier(source[cursor..end].to_owned()),
                line,
            });
            cursor = end;
        } else if bytes[cursor].is_ascii_digit() {
            cursor = push_number(source, cursor, line, &mut tokens);
        } else {
            let width = symbol_width(source, cursor);
            tokens.push(Token {
                kind: Kind::Symbol(source[cursor..cursor + width].to_owned()),
                line,
            });
            cursor += width;
        }
    }
    tokens
}

fn push_number(source: &str, start: usize, line: usize, tokens: &mut Vec<Token>) -> usize {
    let end = take_while(source.as_bytes(), start + 1, number_continue);
    if let Some(value) = parse_number(&source[start..end]) {
        tokens.push(Token {
            kind: Kind::Number(value),
            line,
        });
    }
    end
}

fn parse_number(raw: &str) -> Option<u64> {
    let (radix, body) = [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ]
    .into_iter()
    .find_map(|(prefix, radix)| raw.strip_prefix(prefix).map(|body| (radix, body)))
    .unwrap_or((DECIMAL_RADIX, raw));
    let digits = body
        .chars()
        .take_while(|value| *value == '_' || value.is_digit(radix))
        .filter(|value| *value != '_')
        .collect::<String>();
    (!digits.is_empty())
        .then(|| u64::from_str_radix(&digits, radix).ok())
        .flatten()
}

fn skip_line(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |offset| start + offset)
}

fn symbol_width(source: &str, cursor: usize) -> usize {
    let bytes = source.as_bytes();
    [b"<<".as_slice(), b">>", b"..", b"::", b"==", b"!="]
        .iter()
        .find(|symbol| starts(bytes, cursor, symbol))
        .map_or_else(
            || {
                source[cursor..]
                    .chars()
                    .next()
                    .map_or(ASCII_SYMBOL_WIDTH, char::len_utf8)
            },
            |symbol| symbol.len(),
        )
}

pub(super) fn starts(bytes: &[u8], cursor: usize, value: &[u8]) -> bool {
    bytes.get(cursor..cursor + value.len()) == Some(value)
}

pub(super) fn take_while(bytes: &[u8], mut cursor: usize, predicate: fn(u8) -> bool) -> usize {
    while bytes.get(cursor).is_some_and(|byte| predicate(*byte)) {
        cursor += 1;
    }
    cursor
}

pub(super) fn identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

pub(super) fn identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn number_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
