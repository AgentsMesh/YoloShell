use std::collections::BTreeSet;

const DECLARATION_END: char = ';';

pub(super) fn lines(text: &str, rust: bool) -> BTreeSet<usize> {
    let mut result = BTreeSet::new();
    let mut continuing = false;
    for (index, line) in text.lines().enumerate() {
        let code = code_before_comment(line).trim_start();
        let starts = if rust {
            rust_declaration_start(code)
        } else {
            swift_declaration_start(code)
        };
        if starts || continuing {
            result.insert(index + 1);
            continuing = if rust {
                !code.contains(DECLARATION_END)
            } else {
                swift_continues(code)
            };
        }
    }
    result
}

fn rust_declaration_start(code: &str) -> bool {
    let code = strip_rust_visibility(code);
    code.strip_prefix("const ")
        .is_some_and(|rest| !rest.starts_with("fn "))
        || code.starts_with("static ")
}

fn strip_rust_visibility(code: &str) -> &str {
    if let Some(rest) = code.strip_prefix("pub ") {
        return rest.trim_start();
    }
    let Some(rest) = code.strip_prefix("pub(") else {
        return code;
    };
    rest.split_once(')')
        .map(|(_, value)| value.trim_start())
        .unwrap_or(code)
}

fn swift_declaration_start(code: &str) -> bool {
    let words = code.split_whitespace().collect::<Vec<_>>();
    words.windows(2).any(|pair| pair == ["static", "let"])
}

fn swift_continues(code: &str) -> bool {
    let trimmed = code.trim_end();
    trimmed.ends_with('=')
        || trimmed.ends_with('(')
        || trimmed.ends_with('[')
        || trimmed.ends_with(',')
        || trimmed.matches('(').count() > trimmed.matches(')').count()
        || trimmed.matches('[').count() > trimmed.matches(']').count()
}

fn code_before_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(code, _)| code)
}
