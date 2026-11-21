use crate::pre_exec_rlimit_contract as contract;
use crate::pre_exec_rlimit_syntax as syntax;
use std::ops::Range;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::Token;

struct ClosureBinding {
    name: String,
    declaration: usize,
    body: Range<usize>,
}

pub fn bodies(source: &RustSource) -> Vec<Range<usize>> {
    let mut result = syntax::callback_bodies(&source.tokens);
    for callback in syntax::named_callbacks(&source.tokens) {
        let Some(owner) = source.function_at(callback.call) else {
            continue;
        };
        let bindings = closure_bindings(&source.tokens, owner.body.clone());
        if let Some(binding) = bindings
            .iter()
            .filter(|binding| binding.name == callback.name && binding.declaration < callback.call)
            .max_by_key(|binding| binding.declaration)
        {
            result.push(binding.body.clone());
            continue;
        }
        result.extend(
            source
                .functions()
                .filter(|function| function.name == callback.name)
                .map(|function| function.body.clone()),
        );
    }
    result
}

fn closure_bindings(tokens: &[Token], scope: Range<usize>) -> Vec<ClosureBinding> {
    let mut result = Vec::new();
    let mut index = scope.start;
    while index < scope.end {
        if syntax::closure_open(tokens, index, scope.start)
            && let Some(close) =
                syntax::closure_close(tokens, index + contract::NEXT_TOKEN_OFFSET, scope.end)
            && let Some((name, declaration)) = binding_name(tokens, scope.start, index)
        {
            let body_start = close + contract::NEXT_TOKEN_OFFSET;
            result.push(ClosureBinding {
                name: name.to_owned(),
                declaration,
                body: body_start..syntax::statement_end(tokens, body_start, scope.end),
            });
            index = close;
        }
        index += contract::NEXT_TOKEN_OFFSET;
    }
    result
}

fn binding_name(tokens: &[Token], lower: usize, closure: usize) -> Option<(&str, usize)> {
    let declaration = (lower..closure)
        .rev()
        .find(|index| {
            tokens[*index].symbol(contract::ITEM_END)
                || tokens[*index].symbol(contract::BLOCK_OPEN)
                || tokens[*index].symbol(contract::BLOCK_CLOSE)
        })
        .map_or(lower, |index| index + contract::NEXT_TOKEN_OFFSET);
    let assignment = (declaration..closure)
        .rev()
        .find(|index| tokens[*index].symbol(contract::ASSIGNMENT))?;
    let name = tokens[declaration..assignment]
        .iter()
        .rev()
        .find_map(identifier)?;
    Some((name, declaration))
}

#[cfg(all(test, feature = "yolo-runtime-process-tests"))]
#[path = "../unit_tests/pre_exec_callback_scope_tests.rs"]
mod scope_tests;
