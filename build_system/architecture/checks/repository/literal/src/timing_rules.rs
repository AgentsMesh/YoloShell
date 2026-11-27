use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_syntax::targeted_tokens::{Token, tokenize};

#[path = "timing_rules/declarations.rs"]
mod declarations;

const DURATION_CALLS: &[&str] = &[
    "seconds",
    "milliseconds",
    "microseconds",
    "nanoseconds",
    "from_secs",
    "from_millis",
    "from_micros",
    "from_nanos",
];
const OPERATION_MARKERS: &[&str] = &[
    "attempt", "backoff", "buffer", "capacity", "deadline", "delay", "duration", "interval",
    "limit", "millis", "nanos", "poll", "retry", "second", "sleep", "timeout",
];
const ASSIGNMENT_MARKERS: &[&str] = &[
    "attempt", "backoff", "capacity", "deadline", "delay", "duration", "interval", "limit", "poll",
    "retry", "sleep", "timeout",
];
pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    let rust = file.relative.ends_with(".rs");
    if !rust && !file.relative.ends_with(".swift") {
        return;
    }
    let tokens = tokenize(&file.text);
    let declarations = declarations::lines(&file.text, rust);
    for (index, token) in tokens.iter().enumerate() {
        if token.number().is_none()
            || declarations.contains(&token.line)
            || index > 0 && tokens[index - 1].symbol("$")
            || index > 0 && tokens[index - 1].symbol(".")
        {
            continue;
        }
        if numeric_call(&tokens, index) || numeric_assignment(&tokens, index) {
            violations.push(Violation::line(
                &file.relative,
                token.line,
                "inline operational number must use a named constant",
            ));
        }
    }
}

fn numeric_call(tokens: &[Token], number: usize) -> bool {
    let Some(callee) = enclosing_call(tokens, number) else {
        return false;
    };
    let Some(name) = identifier(tokens, callee) else {
        return false;
    };
    DURATION_CALLS.contains(&name) || operational_name(name) || duration_owner(tokens, callee)
}

fn enclosing_call(tokens: &[Token], start: usize) -> Option<usize> {
    let mut depth = 0_usize;
    for index in (0..start).rev() {
        if tokens[index].symbol(")") {
            depth += 1;
        } else if tokens[index].symbol("(") {
            if depth == 0 {
                return index.checked_sub(1);
            }
            depth -= 1;
        } else if depth == 0
            && [";", "{", "}"]
                .iter()
                .any(|value| tokens[index].symbol(value))
        {
            return None;
        }
    }
    None
}

fn duration_owner(tokens: &[Token], callee: usize) -> bool {
    callee >= 2
        && (tokens[callee - 1].symbol("::") || tokens[callee - 1].symbol("."))
        && tokens[callee - 2].identifier("Duration")
}

fn numeric_assignment(tokens: &[Token], number: usize) -> bool {
    let Some(operator) = assignment_operator(tokens, number) else {
        return false;
    };
    tokens[assignment_start(tokens, operator)..operator]
        .iter()
        .filter_map(|token| match &token.kind {
            yolo_architecture_syntax::targeted_tokens::Kind::Identifier(value) => {
                Some(value.as_str())
            }
            _ => None,
        })
        .any(|name| semantic_name(name, ASSIGNMENT_MARKERS))
}

fn assignment_operator(tokens: &[Token], number: usize) -> Option<usize> {
    for index in (0..number).rev() {
        if tokens[index].symbol("=") || tokens[index].symbol(":") {
            if tokens[index].symbol("=")
                && index > 0
                && ["+", "-", "*", "/", ">", "<"]
                    .iter()
                    .any(|value| tokens[index - 1].symbol(value))
            {
                return None;
            }
            return Some(index);
        }
        if ["(", ",", ";", "{", "}"]
            .iter()
            .any(|value| tokens[index].symbol(value))
        {
            return None;
        }
    }
    None
}

fn assignment_start(tokens: &[Token], operator: usize) -> usize {
    let line = tokens[operator].line;
    (0..operator)
        .rev()
        .find(|index| {
            tokens[*index].line < line
                || [";", "{", "}", ","]
                    .iter()
                    .any(|value| tokens[*index].symbol(value))
        })
        .map_or(0, |index| index + 1)
}

fn operational_name(name: &str) -> bool {
    semantic_name(name, OPERATION_MARKERS)
}

fn semantic_name(name: &str, markers: &[&str]) -> bool {
    let lower = name.to_ascii_lowercase();
    markers.iter().any(|marker| lower.contains(marker))
}

fn identifier(tokens: &[Token], index: usize) -> Option<&str> {
    match &tokens.get(index)?.kind {
        yolo_architecture_syntax::targeted_tokens::Kind::Identifier(value) => Some(value),
        _ => None,
    }
}
