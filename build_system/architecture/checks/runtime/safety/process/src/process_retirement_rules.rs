use crate::process_retirement_contract as contract;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource};
use yolo_architecture_syntax::targeted_tokens::Token;

const VISIBILITY_KEYWORD: &str = "pub";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const ITEM_END: &str = ";";

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !production_rust(file) {
        return;
    }
    let source = RustSource::parse(&file.text);
    let mut definitions = 0;
    let mut direct_calls = 0;
    let mut master_calls = 0;
    let mut bootstrap_calls = 0;
    for (index, token) in source.tokens.iter().enumerate() {
        if !token.identifier(contract::RETIRE_METHOD) || test_guarded(&source, index) {
            continue;
        }
        let function = source.function_at(index);
        if private_definition(file, &source.tokens, index, function) {
            definitions += 1;
        } else if allowed_call(
            file,
            function,
            contract::CALLER_PATH,
            contract::DIRECT_OWNER,
            contract::DIRECT_COMMIT,
        ) {
            direct_calls += 1;
        } else if allowed_call(
            file,
            function,
            contract::CALLER_PATH,
            contract::MASTER_OWNER,
            contract::MASTER_COMMIT,
        ) {
            master_calls += 1;
        } else if allowed_call(
            file,
            function,
            contract::BOOTSTRAP_CALLER_PATH,
            contract::BOOTSTRAP_OWNER,
            contract::BOOTSTRAP_COMMIT,
        ) {
            bootstrap_calls += 1;
        } else {
            violation(file, token, contract::OWNERSHIP_MESSAGE, violations);
        }
    }
    require_count(
        file,
        contract::OWNER_PATH,
        definitions,
        contract::EXPECTED_DEFINITIONS,
        contract::DEFINITION_MESSAGE,
        violations,
    );
    if file.relative == contract::CALLER_PATH
        && (direct_calls != contract::EXPECTED_DIRECT_CALLS
            || master_calls != contract::EXPECTED_MASTER_CALLS)
    {
        violations.push(Violation::file(&file.relative, contract::CALLER_MESSAGE));
    }
    if file.relative == contract::BOOTSTRAP_CALLER_PATH
        && bootstrap_calls != contract::EXPECTED_BOOTSTRAP_CALLS
    {
        violations.push(Violation::file(&file.relative, contract::CALLER_MESSAGE));
    }
}

fn private_definition(
    file: &SourceFile,
    tokens: &[Token],
    index: usize,
    function: Option<&RustFunction>,
) -> bool {
    file.relative == contract::OWNER_PATH
        && function
            .is_some_and(|item| item.matches(Some(contract::TREE_OWNER), contract::RETIRE_METHOD))
        && !item_prefix(tokens, index)
            .iter()
            .any(|token| token.identifier(VISIBILITY_KEYWORD))
}

fn allowed_call(
    file: &SourceFile,
    function: Option<&RustFunction>,
    path: &str,
    owner: &str,
    name: &str,
) -> bool {
    file.relative == path && function.is_some_and(|item| item.matches(Some(owner), name))
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

fn test_guarded(source: &RustSource, index: usize) -> bool {
    source
        .function_at(index)
        .is_some_and(|function| source.is_test_guarded(function))
}

fn production_rust(file: &SourceFile) -> bool {
    yolo_architecture_source_policy::source_policy::is_rust_product_source(&file.relative)
}

fn require_count(
    file: &SourceFile,
    path: &str,
    actual: usize,
    expected: usize,
    message: &str,
    violations: &mut Vec<Violation>,
) {
    if file.relative == path && actual != expected {
        violations.push(Violation::file(&file.relative, message));
    }
}

fn violation(file: &SourceFile, token: &Token, message: &str, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(&file.relative, token.line, message));
}
