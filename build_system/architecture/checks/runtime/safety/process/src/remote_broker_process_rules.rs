use crate::remote_broker_process_contract as contract;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_source_policy::source_policy::is_rust_test_source;
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::Token;

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !is_production_broker(file) {
        return;
    }
    let source = RustSource::parse(&file.text);
    for (index, token) in source.tokens.iter().enumerate() {
        if test_guarded(&source, index) || !is_escape(&source.tokens, index) {
            continue;
        }
        violations.push(Violation::line(
            &file.relative,
            token.line,
            contract::MESSAGE,
        ));
    }
}

fn is_production_broker(file: &SourceFile) -> bool {
    file.relative.ends_with(contract::RUST_SUFFIX)
        && (file.relative == contract::ROOT_SOURCE
            || file.relative.starts_with(contract::SOURCE_PREFIX))
        && !is_rust_test_source(&file.relative)
}

fn test_guarded(source: &RustSource, index: usize) -> bool {
    source
        .function_at(index)
        .is_some_and(|function| source.is_test_guarded(function))
}

fn is_escape(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    contract::PROCESS_ESCAPE_IDENTIFIERS
        .iter()
        .any(|name| token.identifier(name))
        || contract::PROCESS_WRAPPER_CRATES
            .iter()
            .any(|name| token.identifier(name))
        || process_command(tokens, index)
        || raw_clone(tokens, index)
}

fn process_command(tokens: &[Token], index: usize) -> bool {
    if !tokens[index].identifier(contract::COMMAND_IDENTIFIER) {
        return false;
    }
    command_constructor(tokens, index) || command_namespace(tokens, index)
}

fn command_constructor(tokens: &[Token], index: usize) -> bool {
    matches!(
        tokens.get(index + 1..index + 3),
        Some([separator, constructor])
            if separator.symbol(contract::PATH_SEPARATOR)
                && constructor.identifier(contract::CONSTRUCTOR_IDENTIFIER)
    )
}

fn command_namespace(tokens: &[Token], index: usize) -> bool {
    let start = statement_start(tokens, index);
    let scope = &tokens[start..index];
    contract::PROCESS_COMMAND_NAMESPACES
        .iter()
        .any(|namespace| contains_path(scope, namespace, contract::PROCESS_NAMESPACE))
}

fn raw_clone(tokens: &[Token], index: usize) -> bool {
    if !tokens[index].identifier(contract::CLONE_IDENTIFIER) {
        return false;
    }
    tokens
        .get(index.wrapping_sub(1))
        .is_some_and(|token| token.identifier(contract::FUNCTION_KEYWORD))
        || qualified_by(tokens, index, contract::LIBC_NAMESPACE)
        || libc_use(tokens, index)
}

fn qualified_by(tokens: &[Token], index: usize, namespace: &str) -> bool {
    matches!(
        tokens.get(index.wrapping_sub(2)..index),
        Some([owner, separator])
            if owner.identifier(namespace) && separator.symbol(contract::PATH_SEPARATOR)
    )
}

fn libc_use(tokens: &[Token], index: usize) -> bool {
    let start = statement_start(tokens, index);
    tokens
        .get(start)
        .is_some_and(|token| token.identifier(contract::USE_KEYWORD))
        && tokens[start..index]
            .iter()
            .any(|token| token.identifier(contract::LIBC_NAMESPACE))
}

fn statement_start(tokens: &[Token], index: usize) -> usize {
    (0..index)
        .rev()
        .find(|position| tokens[*position].symbol(contract::STATEMENT_END))
        .map_or(0, |position| position + 1)
}

fn contains_path(tokens: &[Token], left: &str, right: &str) -> bool {
    tokens.windows(3).any(|window| {
        window[0].identifier(left)
            && window[1].symbol(contract::PATH_SEPARATOR)
            && window[2].identifier(right)
    })
}

#[cfg(all(test, feature = "yolo-runtime-process-tests"))]
#[path = "../unit_tests/broker_escape_shape_tests.rs"]
mod shape_tests;
