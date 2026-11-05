use std::collections::BTreeSet;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_sites::AUDIT_SIGNAL_SITE;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::Token;

const TYPE_SEPARATOR: &str = ":";
const ASSIGNMENT: &str = "=";
const ITEM_END: &str = ";";
const ARGUMENT_SEPARATOR: &str = ",";
const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const BLOCK_OPEN: &str = "{";
const LET_KEYWORD: &str = "let";
const MUTABLE_KEYWORD: &str = "mut";

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    if file.relative != contract::MACOS_SIGNAL_PATH {
        return;
    }
    for function in source.functions() {
        inspect_function(file, source, function, violations);
    }
}

fn inspect_function(
    file: &SourceFile,
    source: &RustSource,
    function: &RustFunction,
    violations: &mut Vec<Violation>,
) {
    let mut sensitive = typed_bindings(source, function);
    propagate_aliases(source, function, &mut sensitive);
    for index in function.body.clone() {
        let Some(name) = identifier(&source.tokens[index]) else {
            continue;
        };
        if !sensitive.contains(name) || !direct_call(&source.tokens, index) {
            continue;
        }
        let allowed = name == contract::AUDIT_CALL_BINDING
            && AUDIT_SIGNAL_SITE.matches(&file.relative, Some(function));
        if !allowed {
            violations.push(Violation::line(
                &file.relative,
                source.tokens[index].line,
                contract::AUDIT_SIGNAL_MESSAGE,
            ));
        }
    }
}

fn typed_bindings(source: &RustSource, function: &RustFunction) -> BTreeSet<String> {
    function
        .body
        .clone()
        .filter_map(|index| {
            let name = identifier(&source.tokens[index])?;
            let typed = source
                .tokens
                .get(index + 1)
                .is_some_and(|token| token.symbol(TYPE_SEPARATOR));
            let sensitive = typed
                && type_tail(&source.tokens, index + 2)
                    .any(|token| token.identifier(contract::AUDIT_SIGNAL_FUNCTION));
            sensitive.then(|| name.to_owned())
        })
        .collect()
}

fn propagate_aliases(
    source: &RustSource,
    function: &RustFunction,
    sensitive: &mut BTreeSet<String>,
) {
    loop {
        let aliases: Vec<_> = function
            .body
            .clone()
            .filter_map(|index| alias(source, function, index, sensitive))
            .collect();
        let changed = aliases.into_iter().any(|alias| sensitive.insert(alias));
        if !changed {
            break;
        }
    }
}

fn alias(
    source: &RustSource,
    function: &RustFunction,
    index: usize,
    sensitive: &BTreeSet<String>,
) -> Option<String> {
    if !source.tokens[index].identifier(LET_KEYWORD) {
        return None;
    }
    let binding = index
        + usize::from(
            source
                .tokens
                .get(index + 1)
                .is_some_and(|token| token.identifier(MUTABLE_KEYWORD)),
        )
        + 1;
    let name = source.tokens.get(binding).and_then(identifier)?;
    let assignment = source.tokens[binding + 1..function.body.end]
        .iter()
        .position(|token| token.symbol(ASSIGNMENT))?
        + binding
        + 1;
    let end = source.tokens[assignment + 1..function.body.end]
        .iter()
        .position(|token| token.symbol(ITEM_END))?
        + assignment
        + 1;
    source.tokens[assignment + 1..end]
        .iter()
        .any(|token| {
            identifier(token).is_some_and(|value| sensitive.contains(value))
                || token.identifier(contract::AUDIT_STATIC_BINDING)
        })
        .then(|| name.to_owned())
}

fn type_tail(tokens: &[Token], start: usize) -> impl Iterator<Item = &Token> + '_ {
    tokens[start..].iter().take_while(|token| {
        !token.symbol(ARGUMENT_SEPARATOR)
            && !token.symbol(ASSIGNMENT)
            && !token.symbol(ITEM_END)
            && !token.symbol(CALL_CLOSE)
            && !token.symbol(BLOCK_OPEN)
    })
}

fn direct_call(tokens: &[Token], index: usize) -> bool {
    tokens
        .get(index + 1)
        .is_some_and(|token| token.symbol(CALL_OPEN))
}
