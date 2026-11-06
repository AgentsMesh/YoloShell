// Cargo dependencies use full TOML, but policy-owned target declarations use a fixed subset.
// Unsupported quoting, multiline values, or malformed headers fail closed in the policy layer.
#[derive(Debug)]
pub struct Document {
    pub tables: Vec<LocatedName>,
    pub entries: Vec<Entry>,
    pub unsupported_lines: Vec<usize>,
}

#[derive(Debug)]
pub struct LocatedName {
    pub name: String,
    pub line: usize,
    pub array: bool,
}

#[derive(Debug)]
pub struct Entry {
    pub table: String,
    pub key: String,
    pub value: Value,
    pub line: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Value {
    False,
    Text(String),
    Other,
}

pub fn parse(source: &str) -> Document {
    let mut document = Document {
        tables: Vec::new(),
        entries: Vec::new(),
        unsupported_lines: Vec::new(),
    };
    let mut table = String::new();
    for (index, source_line) in source.lines().enumerate() {
        let number = index + 1;
        let Ok(code) = code_before_comment(source_line) else {
            document.unsupported_lines.push(number);
            continue;
        };
        let line = code.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            match table_name(line) {
                Some((name, array)) => {
                    table.clone_from(&name);
                    document.tables.push(LocatedName {
                        name,
                        line: number,
                        array,
                    });
                }
                None => document.unsupported_lines.push(number),
            }
            continue;
        }
        let Some(separator) = assignment_separator(line) else {
            continue;
        };
        let Some(key) = normalized_name(&line[..separator]) else {
            document.unsupported_lines.push(number);
            continue;
        };
        document.entries.push(Entry {
            table: table.clone(),
            key,
            value: value(&line[separator + 1..]),
            line: number,
        });
    }
    document
}

fn table_name(line: &str) -> Option<(String, bool)> {
    let (inner, array) = if line.starts_with("[[") && line.ends_with("]]") {
        (&line[2..line.len() - 2], true)
    } else if line.starts_with('[') && line.ends_with(']') {
        (&line[1..line.len() - 1], false)
    } else {
        return None;
    };
    normalized_name(inner).map(|name| (name, array))
}

fn normalized_name(value: &str) -> Option<String> {
    if value.contains('\\') {
        return None;
    }
    let normalized = value
        .chars()
        .filter(|character| !character.is_whitespace() && !matches!(character, '\'' | '"'))
        .collect::<String>()
        .to_ascii_lowercase();
    (!normalized.is_empty()
        && normalized
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')))
    .then_some(normalized)
}

fn assignment_separator(line: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' && quote == Some('"') {
            escaped = true;
        } else if quote == Some(character) {
            quote = None;
        } else if quote.is_none() && matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if quote.is_none() && character == '=' {
            return Some(index);
        }
    }
    None
}

fn value(source: &str) -> Value {
    let source = source.trim();
    if source == crate::policy_constants::CARGO_FALSE_LITERAL {
        return Value::False;
    }
    for quote in ['"', '\''] {
        if let Some(inner) = source
            .strip_prefix(quote)
            .and_then(|value| value.strip_suffix(quote))
            && (quote == '\'' || !inner.contains('\\'))
        {
            return Value::Text(inner.to_owned());
        }
    }
    Value::Other
}

fn code_before_comment(line: &str) -> Result<String, ()> {
    let characters = line.chars().collect::<Vec<_>>();
    let mut result = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if quote.is_none()
            && matches!(character, '\'' | '"')
            && characters.get(index + 1) == Some(&character)
            && characters.get(index + 2) == Some(&character)
        {
            return Err(());
        }
        if escaped {
            escaped = false;
        } else if character == '\\' && quote == Some('"') {
            escaped = true;
        } else if quote == Some(character) {
            quote = None;
        } else if quote.is_none() && matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if quote.is_none() && character == '#' {
            break;
        }
        result.push(character);
        index += 1;
    }
    quote.is_none().then_some(result).ok_or(())
}
