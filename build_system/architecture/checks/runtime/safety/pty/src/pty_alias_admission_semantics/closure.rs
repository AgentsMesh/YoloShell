use crate::pty_alias_admission_rules as contract;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::constants::{
    PTY_ALIAS_ADMITTED_CLOSE as ADMITTED_CLOSE, PTY_ALIAS_POLL_CLOSE as POLL_CLOSE,
    PTY_ALIAS_SYSTEM_ADMITTED_CLOSE, PTY_ALIAS_SYSTEM_AUTHORITY_FIELD,
    PTY_ALIAS_SYSTEM_AUTHORITY_QUERY, PTY_ALIAS_SYSTEM_AUTHORIZED_CLOSE, PTY_ALIAS_SYSTEM_PERMIT,
    PTY_ALIAS_TASK_ADMITTED_CLOSE as TASK_CLOSE, PTY_ALIAS_TASK_PERMIT as TASK_PERMIT,
    PTY_ALIAS_TRY_LOCK_OPERATION as TRY_LOCK,
};
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource};
use yolo_architecture_syntax::targeted_tokens::{Kind, Token};

const DROP_FUNCTION: &str = "drop";
const WORKER_STATE: &str = "WorkerState";
const FUNCTION: &str = "fn";
const PUBLIC: &str = "pub";
const IF: &str = "if";
const LET: &str = "let";
const SUCCESS: &str = "Ok";
const PERMIT: &str = "permit";
const LEFT_PAREN: &str = "(";
const RIGHT_PAREN: &str = ")";
const ASSIGN: &str = "=";
const STATEMENT_END: &str = ";";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const POLICY: &str = "PTY alias admission forbids gate-external alias closure authority";
const SYSTEM_BYPASS_SYMBOLS: &[&str] = &[
    PTY_ALIAS_SYSTEM_PERMIT,
    PTY_ALIAS_SYSTEM_AUTHORITY_FIELD,
    PTY_ALIAS_SYSTEM_AUTHORITY_QUERY,
    PTY_ALIAS_SYSTEM_ADMITTED_CLOSE,
    PTY_ALIAS_SYSTEM_AUTHORIZED_CLOSE,
];

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    let source = RustSource::parse(&file.text);
    for (index, token) in source.tokens.iter().enumerate() {
        let system_bypass = file.relative == contract::RUNTIME_ALIAS_CLOSURE_PATH
            && SYSTEM_BYPASS_SYMBOLS
                .iter()
                .any(|symbol| token.identifier(symbol));
        let unadmitted_close =
            token.identifier(ADMITTED_CLOSE) && !allowed_admitted_close(&source, index);
        if system_bypass || unadmitted_close {
            violations.push(Violation::line(&file.relative, token.line, POLICY));
        }
    }
}

fn allowed_admitted_close(source: &RustSource, index: usize) -> bool {
    if index > 0 && source.tokens[index - 1].identifier(FUNCTION) {
        return source.function_at(index).is_some_and(|function| {
            function.matches(Some(WORKER_STATE), ADMITTED_CLOSE)
                && private_definition(&source.tokens, index)
        });
    }
    let Some(function) = source.function_at(index) else {
        return false;
    };
    if function.matches(Some(WORKER_STATE), POLL_CLOSE) {
        return successful_try_lock_branch(source, function, index);
    }
    if function.matches(Some(WORKER_STATE), TASK_CLOSE) {
        return typed_reference_parameter(source, function, TASK_PERMIT);
    }
    function.matches(Some(WORKER_STATE), DROP_FUNCTION)
}

fn successful_try_lock_branch(source: &RustSource, function: &RustFunction, index: usize) -> bool {
    enclosing_blocks(&source.tokens, function, index)
        .into_iter()
        .any(|open| {
            successful_try_lock_header(&source.tokens, open)
                && !permit_dropped_before(&source.tokens, open, index)
        })
}

fn permit_dropped_before(tokens: &[Token], open: usize, index: usize) -> bool {
    tokens[open..index].windows(4).any(|window| {
        window[0].identifier(DROP_FUNCTION)
            && window[1].symbol(LEFT_PAREN)
            && window[2].identifier(PERMIT)
            && window[3].symbol(RIGHT_PAREN)
    })
}

fn enclosing_blocks(tokens: &[Token], function: &RustFunction, index: usize) -> Vec<usize> {
    let mut blocks = Vec::new();
    for (position, token) in tokens
        .iter()
        .enumerate()
        .take(index)
        .skip(function.body.start)
    {
        if token.symbol(BLOCK_OPEN) {
            blocks.push(position);
        } else if token.symbol(BLOCK_CLOSE) {
            blocks.pop();
        }
    }
    blocks
}

fn successful_try_lock_header(tokens: &[Token], open: usize) -> bool {
    let start = tokens[..open]
        .iter()
        .rposition(|token| {
            token.symbol(STATEMENT_END) || token.symbol(BLOCK_OPEN) || token.symbol(BLOCK_CLOSE)
        })
        .map_or(0, |position| position + 1);
    let header = &tokens[start..open];
    let success = header.get(..7).is_some_and(|prefix| {
        prefix[0].identifier(IF)
            && prefix[1].identifier(LET)
            && prefix[2].identifier(SUCCESS)
            && prefix[3].symbol(LEFT_PAREN)
            && prefix[4].identifier(PERMIT)
            && prefix[5].symbol(RIGHT_PAREN)
            && prefix[6].symbol(ASSIGN)
    });
    success
        && header
            .get(header.len().saturating_sub(3)..)
            .is_some_and(|suffix| {
                suffix[0].identifier(TRY_LOCK)
                    && suffix[1].symbol(LEFT_PAREN)
                    && suffix[2].symbol(RIGHT_PAREN)
            })
}

fn typed_reference_parameter(source: &RustSource, function: &RustFunction, expected: &str) -> bool {
    source.tokens[function.body.clone()]
        .windows(3)
        .take_while(|window| !window[0].symbol(BLOCK_OPEN))
        .any(|window| {
            window[0].symbol(":")
                && window[1].symbol("&")
                && identifier(&window[2]) == Some(expected)
        })
}

fn private_definition(tokens: &[Token], index: usize) -> bool {
    let start = tokens[..index]
        .iter()
        .rposition(|token| token.symbol(BLOCK_OPEN) || token.symbol(BLOCK_CLOSE))
        .map_or(0, |position| position + 1);
    !tokens[start..index]
        .iter()
        .any(|token| token.identifier(PUBLIC))
}

fn identifier(token: &Token) -> Option<&str> {
    let Kind::Identifier(identifier) = &token.kind else {
        return None;
    };
    Some(identifier)
}

#[cfg(all(test, feature = "yolo-runtime-pty-tests"))]
#[path = "../../unit_tests/closure_gate_token_tests.rs"]
mod token_tests;
