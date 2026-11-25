use super::lexer::{identifier_continue, identifier_start, starts, take_while};
use super::{Kind, Token};

const RAW_PREFIX: u8 = b'r';
const BYTE_PREFIX: u8 = b'b';
const C_PREFIX: u8 = b'c';
const HASH: u8 = b'#';
const STRING_QUOTE: u8 = b'"';
const CHARACTER_QUOTE: u8 = b'\'';

pub fn push_raw_string(
    source: &str,
    start: usize,
    token_line: usize,
    line: &mut usize,
    tokens: &mut Vec<Token>,
) -> Option<usize> {
    let (quote, hashes) = raw_string_prefix(source.as_bytes(), start)?;
    let bytes = source.as_bytes();
    let mut cursor = quote + 1;
    while cursor < bytes.len() {
        *line += usize::from(bytes[cursor] == b'\n');
        if bytes[cursor] == STRING_QUOTE && has_hashes(bytes, cursor + 1, hashes) {
            tokens.push(Token {
                kind: Kind::String(source[quote + 1..cursor].to_owned()),
                line: token_line,
            });
            return Some(cursor + 1 + hashes);
        }
        cursor += 1;
    }
    tokens.push(Token {
        kind: Kind::String(source[quote + 1..].to_owned()),
        line: token_line,
    });
    Some(bytes.len())
}

pub fn push_string(
    source: &str,
    start: usize,
    token_line: usize,
    line: &mut usize,
    tokens: &mut Vec<Token>,
) -> usize {
    let end = skip_quoted(source.as_bytes(), start, line);
    let terminated = end > start + 1 && source.as_bytes().get(end - 1) == Some(&STRING_QUOTE);
    let content_end = if terminated { end - 1 } else { end };
    let value = crate::rust_string_escape::decode(&source[start + 1..content_end]);
    tokens.push(Token {
        kind: Kind::String(value),
        line: token_line,
    });
    end
}

pub fn raw_identifier(source: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = source.as_bytes();
    if bytes.get(start) != Some(&RAW_PREFIX)
        || bytes.get(start + 1) != Some(&HASH)
        || !bytes
            .get(start + 2)
            .is_some_and(|byte| identifier_start(*byte))
    {
        return None;
    }
    let end = take_while(bytes, start + 3, identifier_continue);
    Some((&source[start + 2..end], end))
}

pub fn is_lifetime(bytes: &[u8], start: usize) -> bool {
    if bytes.get(start) != Some(&CHARACTER_QUOTE)
        || !bytes
            .get(start + 1)
            .is_some_and(|byte| identifier_start(*byte))
    {
        return false;
    }
    let end = take_while(bytes, start + 2, identifier_continue);
    bytes.get(end) != Some(&CHARACTER_QUOTE)
}

pub fn skip_quoted(bytes: &[u8], start: usize, line: &mut usize) -> usize {
    let quote = bytes[start];
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        *line += usize::from(bytes[cursor] == b'\n');
        if bytes[cursor] == quote && !escaped(bytes, cursor) {
            return cursor + 1;
        }
        cursor += 1;
    }
    cursor
}

pub fn skip_block_comment(bytes: &[u8], start: usize, line: &mut usize) -> usize {
    let mut cursor = start + 2;
    let mut depth = 1_usize;
    while cursor < bytes.len() {
        if starts(bytes, cursor, b"/*") {
            depth += 1;
            cursor += 2;
        } else if starts(bytes, cursor, b"*/") {
            depth -= 1;
            cursor += 2;
            if depth == 0 {
                return cursor;
            }
        } else {
            *line += usize::from(bytes[cursor] == b'\n');
            cursor += 1;
        }
    }
    cursor
}

fn raw_string_prefix(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut cursor = start;
    if matches!(bytes.get(cursor), Some(&BYTE_PREFIX) | Some(&C_PREFIX)) {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&RAW_PREFIX) {
        return None;
    }
    cursor += 1;
    let hashes = bytes[cursor..]
        .iter()
        .take_while(|byte| **byte == HASH)
        .count();
    cursor += hashes;
    (bytes.get(cursor) == Some(&STRING_QUOTE)).then_some((cursor, hashes))
}

fn has_hashes(bytes: &[u8], start: usize, count: usize) -> bool {
    bytes
        .get(start..start + count)
        .is_some_and(|suffix| suffix.iter().all(|byte| *byte == HASH))
}

fn escaped(bytes: &[u8], cursor: usize) -> bool {
    bytes[..cursor]
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count()
        % 2
        == 1
}

#[cfg(test)]
#[path = "../unit_tests/targeted_literal_boundary_tests.rs"]
mod boundary_tests;
