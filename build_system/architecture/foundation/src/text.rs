#[derive(Debug, Eq, PartialEq)]
pub struct Literal {
    pub line: usize,
    pub value: String,
}

pub fn line_count(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    text.lines().count()
}

pub fn contains_ascii(text: &str, needle: &str) -> bool {
    text.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

pub fn joined(parts: &[&str]) -> String {
    parts.concat()
}

pub fn first_line_ascii(text: &str, needle: &str) -> Option<usize> {
    text.lines()
        .position(|line| contains_ascii(line, needle))
        .map(|index| index + 1)
}

pub fn contains_identifier(text: &str, needle: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    lower.match_indices(&needle).any(|(start, value)| {
        let before = lower[..start].chars().next_back();
        let after = lower[start + value.len()..].chars().next();
        !before.is_some_and(identifier_character) && !after.is_some_and(identifier_character)
    })
}

pub fn first_call_line(text: &str, name: &str) -> Option<usize> {
    let name = name.to_ascii_lowercase();
    text.lines()
        .position(|line| line_contains_call(&line.to_ascii_lowercase(), &name))
        .map(|index| index + 1)
}

#[cfg(test)]
pub fn contains_call(text: &str, name: &str) -> bool {
    first_call_line(text, name).is_some()
}

pub fn quoted_literals(text: &str) -> Vec<Literal> {
    let mut result = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut cursor = 0;
        while cursor < bytes.len() {
            if bytes[cursor] != b'"' || escaped(bytes, cursor) {
                cursor += 1;
                continue;
            }
            let start = cursor + 1;
            cursor = start;
            while cursor < bytes.len() && (bytes[cursor] != b'"' || escaped(bytes, cursor)) {
                cursor += 1;
            }
            if cursor < bytes.len() {
                result.push(Literal {
                    line: index + 1,
                    value: line[start..cursor].to_owned(),
                });
                cursor += 1;
            }
        }
    }
    result
}

pub fn line_at(text: &str, number: usize) -> &str {
    text.lines()
        .nth(number.saturating_sub(1))
        .unwrap_or_default()
}

pub fn protocol_shaped(value: &str) -> bool {
    value.len() >= crate::MIN_SEMANTIC_LITERAL_LENGTH
        && value.contains('_')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn escaped(bytes: &[u8], index: usize) -> bool {
    let mut slashes = 0;
    let mut cursor = index;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        slashes += 1;
        cursor -= 1;
    }
    slashes % 2 == 1
}

fn identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn line_contains_call(line: &str, name: &str) -> bool {
    line.match_indices(name).any(|(start, value)| {
        let before = line[..start].chars().next_back();
        let tail = line[start + value.len()..].trim_start();
        !before.is_some_and(identifier_character) && tail.starts_with('(')
    })
}
