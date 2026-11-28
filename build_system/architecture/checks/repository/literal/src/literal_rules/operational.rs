use crate::constants::{CLI_PREFIX, OPERATIONAL_MODE_MARKER};
use std::path::Path;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_foundation::text::line_at;

#[path = "operational_permissions.rs"]
mod permissions;
#[path = "operational_values.rs"]
mod values;

const NUMBER_MARKERS: &[&str] = &[
    "attempt",
    "capacity",
    "deadline",
    "jobs",
    "limit",
    OPERATIONAL_MODE_MARKER,
    "protocol",
    "retry",
    "schema",
    "sleep",
    "timeout",
];
const STRING_MARKERS: &[&str] = &[
    "jobs",
    OPERATIONAL_MODE_MARKER,
    "protocol",
    "schema",
    "timeout",
];
pub(super) fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !formal_contract_source(&file.relative) || centralized_source(&file.relative) {
        return;
    }
    for (line, value) in values::string_literals(file) {
        if !named_declaration(file, line)
            && (values::high_risk_string(file, line, &value)
                || permissions::inline_string(file, line, &value))
        {
            violations.push(Violation::line(
                &file.relative,
                line,
                format!("operational literal {value:?} must use a named contract"),
            ));
        }
    }
    if !shell_source(&file.relative) && !starlark_source(&file.relative) {
        return;
    }
    for (index, line) in file.text.lines().enumerate() {
        let number = index + 1;
        if named_declaration(file, number) {
            continue;
        }
        let context = unquoted_code(line).to_ascii_lowercase();
        if (numeric_context(&context) || permissions::command(&context)) && unquoted_number(line) {
            violations.push(Violation::line(
                &file.relative,
                number,
                "operational number must use a named contract",
            ));
        }
        if shell_source(&file.relative) && unquoted_cli(line) {
            violations.push(Violation::line(
                &file.relative,
                number,
                "operational CLI argument must use a named contract",
            ));
        }
    }
}

fn formal_contract_source(relative: &str) -> bool {
    let lower = relative.to_ascii_lowercase();
    let formal = lower.starts_with("build_system/release/")
        || lower.starts_with("plugins/macos/tests/e2e/")
        || lower.starts_with("build_system/platform/macos/release_evidence/")
        || lower == "build_system/platform/macos/release.bzl"
        || lower == "build_system/platform/macos/packaged_runtime_e2e.bzl"
        || lower == "build_system/bazel/rust/launcher.bzl";
    formal
        && [".rs", ".swift", ".sh", ".bash", ".zsh", ".bzl", ".bazel"]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
}

fn centralized_source(relative: &str) -> bool {
    if super::centralized(relative) {
        return true;
    }
    let name = Path::new(relative)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.ends_with("_constants.sh") || name == "constants.sh"
}

fn numeric_context(context: &str) -> bool {
    NUMBER_MARKERS.iter().any(|marker| context.contains(marker))
}

pub(super) fn string_context(context: &str) -> bool {
    STRING_MARKERS.iter().any(|marker| context.contains(marker))
}

fn named_declaration(file: &SourceFile, line: usize) -> bool {
    let code = line_at(&file.text, line).trim_start();
    if shell_source(&file.relative) {
        if code.starts_with("readonly ") || code.starts_with("declare -r ") {
            return true;
        }
        return code.split_once('=').is_some_and(|(name, _)| {
            let name = name.trim_start_matches("export ").trim();
            !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        });
    }
    if file.relative.ends_with(".bzl") || file.relative.ends_with(".bazel") {
        return code.split_once('=').is_some_and(|(name, _)| {
            let name = name.trim();
            !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        });
    }
    code.starts_with("const ")
        || code.starts_with("pub const ")
        || code.starts_with("static let ")
        || code.starts_with("private static let ")
}

fn unquoted_number(line: &str) -> bool {
    unquoted_code(line)
        .split_ascii_whitespace()
        .map(|word| word.trim_matches(|value: char| ",;()[]{}".contains(value)))
        .any(numeric_word)
}

fn numeric_word(word: &str) -> bool {
    !word.is_empty()
        && !word.contains('$')
        && word.bytes().any(|byte| byte.is_ascii_digit())
        && word.bytes().all(|byte| {
            byte.is_ascii_digit()
                || matches!(
                    byte,
                    b'_' | b'.' | b'-' | b'x' | b'X' | b'o' | b'O' | b'b' | b'B'
                )
                || matches!(byte, b'a'..=b'f' | b'A'..=b'F')
        })
}

fn unquoted_cli(line: &str) -> bool {
    unquoted_code(line)
        .split_ascii_whitespace()
        .map(|word| word.trim_matches(|value: char| !value.is_ascii_alphanumeric() && value != '-'))
        .any(|word| word.starts_with(CLI_PREFIX) && word.len() > CLI_PREFIX.len())
}

fn unquoted_code(line: &str) -> String {
    let mut quote = None;
    let mut result = String::new();
    for value in line.chars() {
        if quote.is_none() && value == '#' {
            break;
        }
        if matches!(value, '\'' | '"') {
            quote = if quote == Some(value) {
                None
            } else {
                quote.or(Some(value))
            };
        }
        result.push(if quote.is_some() { ' ' } else { value });
    }
    result
}

fn shell_source(relative: &str) -> bool {
    [".sh", ".bash", ".zsh"]
        .iter()
        .any(|suffix| relative.ends_with(suffix))
}

fn starlark_source(relative: &str) -> bool {
    relative.ends_with(".bzl") || relative.ends_with(".bazel")
}
