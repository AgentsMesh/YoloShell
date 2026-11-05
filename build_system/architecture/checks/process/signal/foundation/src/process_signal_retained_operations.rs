use crate::process_signal_authority_sites as authority_sites;
use crate::process_signal_retained_bindings::Context;
use crate::process_signal_syntax as syntax;
use crate::process_signal_token_sequence as sequence;
use std::ops::Range;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_policy_contracts::process_signal_retained as retained;
use yolo_architecture_process_signal_path_analysis::paths::Resolution;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const GROUP_OPEN: &str = "(";
const GROUP_CLOSE: &str = ")";
const ARRAY_OPEN: &str = "[";
const ARRAY_CLOSE: &str = "]";

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    let retained = Context::new(source);
    inspect_context(file, source, &retained, violations);
}

pub fn inspect_with_resolution(
    file: &SourceFile,
    source: &RustSource,
    external: &Resolution,
    violations: &mut Vec<Violation>,
) {
    let retained = Context::with_resolution(source, external);
    inspect_context(file, source, &retained, violations);
}

fn inspect_context(
    file: &SourceFile,
    source: &RustSource,
    retained: &Context,
    violations: &mut Vec<Violation>,
) {
    let macro_bodies = macro_bodies(&source.tokens);
    inspect_macro_bodies(file, source, &macro_bodies, violations);
    inspect_unscoped(file, source, retained, &macro_bodies, violations);
    for function in source.functions() {
        let bindings = retained.bindings(source, function);
        for index in function.body.clone() {
            let Some(name) = identifier(&source.tokens[index]) else {
                continue;
            };
            if !sensitive(name) {
                continue;
            }
            if macro_bodies.iter().any(|body| body.contains(&index)) {
                continue;
            }
            let method_call = syntax::method_call(&source.tokens, index);
            if sequence::macro_argument(&source.tokens, index)
                && !method_call
                && !syntax::call(&source.tokens, index)
                && syntax::qualified_reference(&source.tokens, index).is_none()
            {
                violation(file, &source.tokens[index], violations);
                continue;
            }
            let associated = retained.is_associated(&source.tokens, index);
            let method = method_call
                && retained.receiver_is_retained(&source.tokens, index, function, &bindings);
            if !associated && !method || allowed(file, function, name) {
                continue;
            }
            violation(file, &source.tokens[index], violations);
        }
    }
}

fn inspect_unscoped(
    file: &SourceFile,
    source: &RustSource,
    retained: &Context,
    macro_bodies: &[Range<usize>],
    violations: &mut Vec<Violation>,
) {
    for (index, token) in source.tokens.iter().enumerate() {
        let Some(_) = identifier(token).filter(|name| sensitive(name)) else {
            continue;
        };
        if source.function_at(index).is_some()
            || macro_bodies.iter().any(|body| body.contains(&index))
        {
            continue;
        }
        let associated = retained.is_associated(&source.tokens, index);
        if syntax::method_call(&source.tokens, index)
            || associated
            || sequence::macro_argument(&source.tokens, index)
        {
            violation(file, token, violations);
        }
    }
}

fn inspect_macro_bodies(
    file: &SourceFile,
    source: &RustSource,
    bodies: &[Range<usize>],
    violations: &mut Vec<Violation>,
) {
    for body in bodies {
        for index in body.clone() {
            let Some(name) = identifier(&source.tokens[index]) else {
                continue;
            };
            if sensitive(name)
                && (syntax::method_call(&source.tokens, index)
                    || retained_macro_operation(&source.tokens, index))
            {
                violation(file, &source.tokens[index], violations);
            }
        }
    }
}

fn retained_macro_operation(tokens: &[Token], index: usize) -> bool {
    syntax::qualified_reference(tokens, index)
        .is_some_and(|(qualifier, _)| qualifier == contract::RETAINED_EXACT_PROCESS)
}

fn macro_bodies(tokens: &[Token]) -> Vec<Range<usize>> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.identifier(retained::MACRO_RULES))
        .filter_map(|(index, _)| macro_body(tokens, index))
        .collect()
}

fn macro_body(tokens: &[Token], declaration: usize) -> Option<Range<usize>> {
    let (offset, left, right) = tokens[declaration + 1..]
        .iter()
        .enumerate()
        .find_map(|(offset, token)| macro_delimiter(token).map(|pair| (offset, pair.0, pair.1)))?;
    let open = declaration + 1 + offset;
    let close = closing(tokens, open, left, right)?;
    Some(open + 1..close)
}

fn macro_delimiter(token: &Token) -> Option<(&'static str, &'static str)> {
    if token.symbol(BLOCK_OPEN) {
        Some((BLOCK_OPEN, BLOCK_CLOSE))
    } else if token.symbol(GROUP_OPEN) {
        Some((GROUP_OPEN, GROUP_CLOSE))
    } else if token.symbol(ARRAY_OPEN) {
        Some((ARRAY_OPEN, ARRAY_CLOSE))
    } else {
        None
    }
}

fn sensitive(name: &str) -> bool {
    syntax::retained_exact_method(name) || syntax::retained_exact_observation(name)
}

fn allowed(
    file: &SourceFile,
    function: &yolo_architecture_syntax::rust_source::RustFunction,
    name: &str,
) -> bool {
    authority_sites::CONSTRUCTOR_SITES
        .iter()
        .chain(crate::process_signal_lifecycle_constructor_sites::SITES)
        .any(|entry| {
            entry.qualifier == contract::RETAINED_EXACT_PROCESS
                && entry.name == name
                && entry.site.matches(&file.relative, Some(function))
        })
}

fn violation(file: &SourceFile, token: &Token, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(
        &file.relative,
        token.line,
        contract::AUTHORITY_CONSTRUCTOR_MESSAGE,
    ));
}

#[cfg(test)]
#[path = "../unit_tests/retained_operation_boundary_tests.rs"]
mod boundary_tests;
