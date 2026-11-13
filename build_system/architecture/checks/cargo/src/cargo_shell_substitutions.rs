pub struct Substitution {
    pub line: usize,
    pub source: String,
}

pub fn substitutions(source: &str) -> Vec<Substitution> {
    let bytes = source.as_bytes();
    let mut result = Vec::new();
    let mut cursor = 0;
    let mut line = 1;
    let mut quote = None;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\n' {
            line += 1;
            cursor += 1;
        } else if bytes[cursor] == b'\\' && quote != Some(b'\'') {
            cursor = skip_escaped(bytes, cursor, &mut line);
        } else if bytes[cursor] == b'\'' {
            quote = toggle(quote, b'\'');
            cursor += 1;
        } else if bytes[cursor] == b'"' {
            quote = toggle(quote, b'"');
            cursor += 1;
        } else if quote != Some(b'\'') && bytes[cursor] == b'`' {
            let start = cursor + 1;
            if let Some(end) = closing_backtick(bytes, start) {
                result.push(Substitution {
                    line,
                    source: source[start..end].to_owned(),
                });
                line += count_lines(&bytes[start..=end]);
                cursor = end + 1;
            } else {
                cursor += 1;
            }
        } else if quote != Some(b'\'') && starts(bytes, cursor, b"$(") {
            let start = cursor + 2;
            if let Some(end) = closing_parenthesis(bytes, start) {
                result.push(Substitution {
                    line,
                    source: source[start..end].to_owned(),
                });
                line += count_lines(&bytes[start..=end]);
                cursor = end + 1;
            } else {
                cursor += 2;
            }
        } else if quote.is_none() && comment_start(bytes, cursor) {
            cursor = skip_comment(bytes, cursor);
        } else {
            cursor += 1;
        }
    }
    result
}

fn closing_backtick(bytes: &[u8], mut cursor: usize) -> Option<usize> {
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' {
            cursor = (cursor + 2).min(bytes.len());
        } else if bytes[cursor] == b'`' {
            return Some(cursor);
        } else {
            cursor += 1;
        }
    }
    None
}

fn closing_parenthesis(bytes: &[u8], mut cursor: usize) -> Option<usize> {
    let mut depth = 1;
    let mut quote = None;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' && quote != Some(b'\'') {
            cursor = (cursor + 2).min(bytes.len());
            continue;
        }
        if matches!(bytes[cursor], b'\'' | b'"') {
            quote = toggle(quote, bytes[cursor]);
            cursor += 1;
            continue;
        }
        if quote.is_none() && bytes[cursor] == b'(' {
            depth += 1;
        } else if quote.is_none() && bytes[cursor] == b')' {
            depth -= 1;
            if depth == 0 {
                return Some(cursor);
            }
        }
        cursor += 1;
    }
    None
}

fn toggle(current: Option<u8>, quote: u8) -> Option<u8> {
    if current == Some(quote) {
        None
    } else if current.is_none() {
        Some(quote)
    } else {
        current
    }
}

fn skip_escaped(bytes: &[u8], cursor: usize, line: &mut usize) -> usize {
    if bytes.get(cursor + 1) == Some(&b'\n') {
        *line += 1;
    }
    (cursor + 2).min(bytes.len())
}

fn comment_start(bytes: &[u8], cursor: usize) -> bool {
    bytes[cursor] == b'#'
        && (cursor == 0
            || bytes[cursor - 1].is_ascii_whitespace()
            || matches!(bytes[cursor - 1], b';' | b'|' | b'&'))
}

fn skip_comment(bytes: &[u8], cursor: usize) -> usize {
    bytes[cursor..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |offset| cursor + offset)
}

fn count_lines(bytes: &[u8]) -> usize {
    bytes.iter().filter(|byte| **byte == b'\n').count()
}

fn starts(bytes: &[u8], cursor: usize, value: &[u8]) -> bool {
    bytes.get(cursor..cursor + value.len()) == Some(value)
}
