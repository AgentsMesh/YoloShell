use crate::constants::CLI_PREFIX;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::text::line_at;
use yolo_architecture_syntax::targeted_tokens::Kind;

const ENV_PREFIXES: &[&str] = &["BUILD_", "RUNFILES_", "TEST_", "TMPDIR", "YOLO_"];
const PATH_SUFFIXES: &[&str] = &[".app", ".dylib", ".json", ".plist", ".sh", ".zip"];
const CONTEXT_LOOKBACK: usize = 3;

pub(super) fn string_literals(file: &SourceFile) -> Vec<(usize, String)> {
    if file.relative.ends_with(".rs") || file.relative.ends_with(".swift") {
        return yolo_architecture_syntax::targeted_tokens::tokenize(&file.text)
            .into_iter()
            .filter_map(|token| match token.kind {
                Kind::String(value) => Some((token.line, value)),
                _ => None,
            })
            .collect();
    }
    yolo_architecture_syntax::cargo_starlark_tokens::tokenize(&file.text)
        .into_iter()
        .filter_map(|token| token.string().map(|value| (token.line, value.to_owned())))
        .collect()
}

pub(super) fn high_risk_string(file: &SourceFile, line: usize, value: &str) -> bool {
    if starlark_load(file, line) || rust_cfg_feature(file, line) || rust_path_attribute(file, line)
    {
        return false;
    }
    let context = semantic_context(&file.text, line);
    value.starts_with(CLI_PREFIX)
        || environment_name(value)
        || (!rust_swift_test(file) && fixed_path(value))
        || (!rust_swift_test(file)
            && contract_value(value)
            && (super::string_context(&context) || contract_named_value(value)))
}

fn rust_path_attribute(file: &SourceFile, line: usize) -> bool {
    if !file.relative.ends_with(".rs") {
        return false;
    }
    let code = line_at(&file.text, line).trim_start();
    code.starts_with("#[path =") || code.starts_with("#[cfg_attr(") && code.contains("path =")
}

fn environment_name(value: &str) -> bool {
    ENV_PREFIXES.iter().any(|prefix| value.starts_with(prefix))
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn fixed_path(value: &str) -> bool {
    if value.is_empty() || value.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return false;
    }
    if value.starts_with("${") || value.starts_with("$(") || bazel_label(value) {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    value.len() > 1
        && (value.starts_with('/')
            || value.starts_with("./")
            || value.starts_with("../")
            || value.contains('/') && PATH_SUFFIXES.iter().any(|suffix| lower.ends_with(suffix)))
}

fn bazel_label(value: &str) -> bool {
    value.starts_with("//") || value.starts_with('@') || value.starts_with(':')
}

fn contract_value(value: &str) -> bool {
    !value.is_empty()
        && !value.bytes().any(|byte| byte.is_ascii_whitespace())
        && (value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'_')
            || yolo_architecture_foundation::text::protocol_shaped(value)
            || value.contains('-'))
}

fn contract_named_value(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("protocol") || lower.contains("schema")
}

pub(super) fn semantic_context(text: &str, line: usize) -> String {
    let start = line.saturating_sub(CONTEXT_LOOKBACK);
    (start..=line)
        .map(|number| super::unquoted_code(line_at(text, number)).to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

fn rust_swift_test(file: &SourceFile) -> bool {
    (file.relative.ends_with(".rs") || file.relative.ends_with(".swift"))
        && (yolo_architecture_source_policy::source_policy::is_rust_test_source(&file.relative)
            || file.relative.contains("/Tests/")
            || file.relative.ends_with("_tests.rs"))
}

fn rust_cfg_feature(file: &SourceFile, line: usize) -> bool {
    let code = line_at(&file.text, line);
    file.relative.ends_with(".rs")
        && code.trim_start().starts_with("#[cfg(")
        && code.contains("feature")
}

fn starlark_load(file: &SourceFile, line: usize) -> bool {
    if !file.relative.ends_with(".bzl") && !file.relative.ends_with(".bazel") {
        return false;
    }
    let mut active = false;
    for number in 1..=line {
        let code = line_at(&file.text, number).trim_start();
        active |= code.starts_with("load(");
        if number == line {
            return active;
        }
        active &= !code.contains(')');
    }
    false
}

#[cfg(test)]
#[path = "../../unit_tests/operational_value_boundaries.rs"]
mod tests;
