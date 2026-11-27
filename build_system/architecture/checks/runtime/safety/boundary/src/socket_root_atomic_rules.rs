use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::constants::{
    SOCKET_ROOT_BOUNDARY_WRITER, SOCKET_ROOT_ENTRY_WRITER, SOCKET_ROOT_PATH_WRITER,
};
use yolo_architecture_syntax::targeted_tokens::{Kind, Token, tokenize};

#[path = "socket_registered_publication_rules.rs"]
mod registered_publication;
#[path = "socket_root_atomic_session_marker.rs"]
mod session_marker;

const SOCKET_NAMESPACE_PREFIX: &str = "core/crates/persistence/paths/src/socket/namespace/";
const RUST_SUFFIX: &str = ".rs";
const FUNCTION_KEYWORD: &str = "fn";
const USE_KEYWORD: &str = "use";
const ALIAS_KEYWORD: &str = "as";
const CALL_OPEN: &str = "(";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const STATEMENT_END: &str = ";";
const EXPECTED_REFERENCE_COUNT: usize = 1;
const OWNERSHIP_MESSAGE: &str =
    "socket-root generic atomic publication is retired; use registered namespace admission";

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    for file in files.iter().filter(|file| is_rust(&file.relative)) {
        inspect_file(file, violations);
    }
    registered_publication::inspect(files, violations);
    session_marker::inspect(files, violations);
}

fn inspect_file(file: &SourceFile, violations: &mut Vec<Violation>) {
    let tokens = tokenize(&file.text);
    for token in &tokens {
        if token.identifier(SOCKET_ROOT_BOUNDARY_WRITER)
            || (is_socket_namespace_source(&file.relative)
                && [SOCKET_ROOT_ENTRY_WRITER, SOCKET_ROOT_PATH_WRITER]
                    .iter()
                    .any(|writer| token.identifier(writer)))
        {
            reject(file, token, violations);
        }
    }
}

fn statement(tokens: &[Token], index: usize) -> &[Token] {
    let start = tokens[..index]
        .iter()
        .rposition(|token| token.symbol(STATEMENT_END))
        .map_or(0, |position| position + 1);
    let end = tokens[index..]
        .iter()
        .position(|token| token.symbol(STATEMENT_END))
        .map_or(tokens.len(), |position| index + position);
    &tokens[start..end]
}

fn enclosing_function(tokens: &[Token], stop: usize) -> Option<&str> {
    let mut depth = 0_usize;
    let mut pending = None;
    let mut functions = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(stop + 1) {
        if token.identifier(FUNCTION_KEYWORD) {
            pending = tokens.get(index + 1).and_then(identifier);
        } else if token.symbol(STATEMENT_END) {
            pending = None;
        } else if token.symbol(BLOCK_OPEN) {
            depth += 1;
            if let Some(name) = pending.take() {
                functions.push((depth, name));
            }
        } else if token.symbol(BLOCK_CLOSE) {
            if functions.last().is_some_and(|(body, _)| *body == depth) {
                functions.pop();
            }
            depth = depth.saturating_sub(1);
        }
    }
    functions.last().map(|(_, name)| *name)
}

fn identifier(token: &Token) -> Option<&str> {
    match &token.kind {
        Kind::Identifier(value) => Some(value),
        _ => None,
    }
}

fn require_count(
    file: &SourceFile,
    path: &str,
    actual: usize,
    message: &str,
    violations: &mut Vec<Violation>,
) {
    if file.relative == path && actual != EXPECTED_REFERENCE_COUNT {
        violations.push(Violation::file(&file.relative, message));
    }
}

fn reject(file: &SourceFile, token: &Token, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(
        &file.relative,
        token.line,
        OWNERSHIP_MESSAGE,
    ));
}

fn is_rust(relative: &str) -> bool {
    relative.ends_with(RUST_SUFFIX)
}

fn is_socket_namespace_source(relative: &str) -> bool {
    relative.starts_with(SOCKET_NAMESPACE_PREFIX)
}

#[cfg(all(test, feature = "yolo-runtime-boundary-tests"))]
#[path = "../unit_tests/socket_publication_scope_tests.rs"]
mod scope_tests;
