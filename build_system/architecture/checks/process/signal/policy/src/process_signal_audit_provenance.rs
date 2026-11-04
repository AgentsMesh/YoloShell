use yolo_architecture_foundation::SourceFile;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_source_policy::source_policy::is_rust_test_source;
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const MEMBER_ACCESS: &str = ".";
const ITEM_END: &str = ";";
const TYPE_KEYWORD: &str = "type";
const STATIC_KEYWORD: &str = "static";
const FUNCTION_KEYWORD: &str = "fn";
const DEREFERENCE: &str = "*";

pub fn approved_symbol(file: &SourceFile, source: &RustSource, index: usize) -> bool {
    if file.relative != contract::MACOS_SIGNAL_PATH {
        return false;
    }
    source
        .function_at(index)
        .is_some_and(|function| function.matches(None, contract::AUDIT_RESOLVER_FUNCTION))
        || source.function_at(index).is_none()
            && statement_contains(&source.tokens, index, contract::AUDIT_SYMBOL_BINDING)
}

pub fn approved_function_type(file: &SourceFile, source: &RustSource, index: usize) -> bool {
    if file.relative != contract::MACOS_SIGNAL_PATH {
        return false;
    }
    let function = source.function_at(index);
    function.is_some_and(|function| {
        matches!(
            function.name.as_str(),
            contract::AUDIT_WRAPPER_FUNCTION | contract::AUDIT_RESOLVER_FUNCTION
        )
    }) || function.is_none()
        && (source
            .tokens
            .get(index.wrapping_sub(1))
            .is_some_and(|token| token.identifier(TYPE_KEYWORD))
            || statement_contains(&source.tokens, index, contract::AUDIT_STATIC_BINDING))
}

pub fn approved_static_binding(file: &SourceFile, source: &RustSource, index: usize) -> bool {
    if file.relative != contract::MACOS_SIGNAL_PATH {
        return false;
    }
    source.function_at(index).map_or_else(
        || statement_contains(&source.tokens, index, STATIC_KEYWORD),
        |function| {
            function.matches(None, contract::AUDIT_ENTRY_FUNCTION)
                && enclosing_wrapper_call(source, index)
                    .is_some_and(|call| exact_launch_bound_call(&source.tokens, call))
                || function.matches(None, contract::AUDIT_CAPABILITY_FUNCTION)
                    && exact_capability_probe(&source.tokens, index)
        },
    )
}

pub fn approved_wrapper_reference(file: &SourceFile, source: &RustSource, index: usize) -> bool {
    if definition(&source.tokens, index) {
        return file.relative == contract::MACOS_SIGNAL_PATH;
    }
    if is_rust_test_source(&file.relative)
        || source
            .function_at(index)
            .is_some_and(|function| source.is_unit_test_guarded(function))
    {
        return true;
    }
    file.relative == contract::MACOS_SIGNAL_PATH
        && source
            .function_at(index)
            .is_some_and(|function| function.matches(None, contract::AUDIT_ENTRY_FUNCTION))
        && exact_launch_bound_call(&source.tokens, index)
}

fn exact_launch_bound_call(tokens: &[Token], call: usize) -> bool {
    let open = call + 1;
    let Some(close) = closing(tokens, open, CALL_OPEN, CALL_CLOSE) else {
        return false;
    };
    let argument_end = close - usize::from(tokens[close - 1].symbol(","));
    let Some(comma) = tokens[open + 1..argument_end]
        .iter()
        .rposition(|token| token.symbol(","))
        .map(|offset| open + 1 + offset)
    else {
        return false;
    };
    matches!(tokens.get(comma + 1..argument_end),
        Some([deref, binding, member, method, nested_open, resolver, nested_close])
            if deref.symbol(DEREFERENCE)
                && binding.identifier(contract::AUDIT_STATIC_BINDING)
                && member.symbol(MEMBER_ACCESS)
                && method.identifier(contract::GET_OR_INIT)
                && nested_open.symbol(CALL_OPEN)
                && resolver.identifier(contract::AUDIT_RESOLVER_FUNCTION)
                && nested_close.symbol(CALL_CLOSE))
}

fn exact_capability_probe(tokens: &[Token], binding: usize) -> bool {
    matches!(tokens.get(binding + 1..binding + 8),
        Some([member, method, open, resolver, close, next_member, probe])
            if member.symbol(MEMBER_ACCESS)
                && method.identifier(contract::GET_OR_INIT)
                && open.symbol(CALL_OPEN)
                && resolver.identifier(contract::AUDIT_RESOLVER_FUNCTION)
                && close.symbol(CALL_CLOSE)
                && next_member.symbol(MEMBER_ACCESS)
                && probe.identifier(contract::IS_SOME_METHOD))
}

fn enclosing_wrapper_call(source: &RustSource, index: usize) -> Option<usize> {
    source.function_at(index)?.body.clone().find(|position| {
        source.tokens[*position].identifier(contract::AUDIT_WRAPPER_FUNCTION)
            && *position < index
            && closing(&source.tokens, *position + 1, CALL_OPEN, CALL_CLOSE)
                .is_some_and(|close| index < close)
    })
}

fn statement_contains(tokens: &[Token], index: usize, name: &str) -> bool {
    let start = tokens[..index]
        .iter()
        .rposition(|token| token.symbol(ITEM_END))
        .map_or(0, |position| position + 1);
    let end = tokens[index..]
        .iter()
        .position(|token| token.symbol(ITEM_END))
        .map_or(tokens.len(), |offset| index + offset);
    tokens[start..end]
        .iter()
        .any(|token| token.identifier(name))
}

fn definition(tokens: &[Token], index: usize) -> bool {
    index > 0 && tokens[index - 1].identifier(FUNCTION_KEYWORD)
}

#[cfg(test)]
#[path = "../unit_tests/audit/audit_token_provenance_tests.rs"]
mod token_tests;
