use std::path::Path;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_source_policy::source_policy::is_rust_test_source;
use yolo_architecture_syntax::rust_source::{RustSource, identifier, string};
use yolo_architecture_syntax::rust_source_cfg::is_unit_test_guarded;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const MODULE_KEYWORD: &str = "mod";
const INCLUDE_MACRO: &str = "include";
const MACRO_MARKER: &str = "!";
const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const ITEM_END: &str = ";";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const TEST_MODULE: &str = "tests";

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    if is_rust_test_source(&file.relative) {
        return;
    }
    for (index, token) in source.tokens.iter().enumerate() {
        if token.identifier(MODULE_KEYWORD) {
            inspect_module(file, source, index, violations);
        } else if token.identifier(INCLUDE_MACRO) {
            inspect_include(file, source, index, violations);
        }
    }
}

fn inspect_module(
    file: &SourceFile,
    source: &RustSource,
    index: usize,
    violations: &mut Vec<Violation>,
) {
    let Some(name) = source.tokens.get(index + 1).and_then(identifier) else {
        return;
    };
    let prefix = item_prefix(&source.tokens, index);
    let explicit_path = prefix.iter().filter_map(string).last();
    if !test_name(name) && !explicit_path.is_some_and(test_path) {
        return;
    }
    if !is_unit_test_guarded(prefix) {
        violation(file, &source.tokens[index], violations);
    }
}

fn inspect_include(
    file: &SourceFile,
    source: &RustSource,
    index: usize,
    violations: &mut Vec<Violation>,
) {
    let Some(_) = include_path(&source.tokens, index).filter(|path| test_path(path)) else {
        return;
    };
    let guarded = source
        .function_at(index)
        .is_some_and(|function| source.is_unit_test_guarded(function))
        || is_unit_test_guarded(item_prefix(&source.tokens, index));
    if !guarded {
        violation(file, &source.tokens[index], violations);
    }
}

fn include_path(tokens: &[Token], index: usize) -> Option<String> {
    let [marker, open] = tokens.get(index + 1..index + 3)? else {
        return None;
    };
    if !marker.symbol(MACRO_MARKER) || !open.symbol(CALL_OPEN) {
        return None;
    }
    let close = closing(tokens, index + 2, CALL_OPEN, CALL_CLOSE)?;
    let path = tokens[index + 3..close]
        .iter()
        .filter_map(string)
        .collect::<String>();
    (!path.is_empty()).then_some(path)
}

fn item_prefix(tokens: &[Token], index: usize) -> &[Token] {
    let start = tokens[..index]
        .iter()
        .rposition(|token| {
            token.symbol(ITEM_END) || token.symbol(BLOCK_OPEN) || token.symbol(BLOCK_CLOSE)
        })
        .map_or(0, |position| position + 1);
    &tokens[start..index]
}

fn test_name(name: &str) -> bool {
    name == TEST_MODULE
        || name.ends_with(contract::TEST_FILE_SUFFIX)
        || name.ends_with(contract::TESTS_FILE_SUFFIX)
}

fn test_path(path: &str) -> bool {
    let path = Path::new(path);
    path.ends_with(contract::PTY_SESSION_LINUX_INTEGRATION_HOLDER_FILE)
        || path
            .components()
            .any(|component| component.as_os_str() == TEST_MODULE)
        || path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(test_name)
}

fn violation(file: &SourceFile, token: &Token, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(
        &file.relative,
        token.line,
        contract::TEST_MODULE_MESSAGE,
    ));
}

#[cfg(test)]
#[path = "../unit_tests/test_module_boundary_tests.rs"]
mod boundary_tests;
