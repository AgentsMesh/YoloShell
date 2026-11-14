use super::containment::Context;
use crate::process_signal_retained_api as api;
use std::ops::Range;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_process_signal_foundation::process_signal_retained_export_sites as sites;
use yolo_architecture_process_signal_path_analysis::types::RetainedTypes;
use yolo_architecture_process_signal_path_analysis::visibility_fields as syntax;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::Token;

const ASSIGNMENT: &str = "=";
const CONST_KEYWORD: &str = "const";
const FUNCTION_KEYWORD: &str = "fn";
const GROUP_OPEN: &str = "(";
const NEXT_TOKEN_OFFSET: usize = 1;
const STATIC_KEYWORD: &str = "static";
const TRAIT_KEYWORD: &str = "trait";
const TYPE_SEPARATOR: &str = ":";
const USE_KEYWORD: &str = "use";

pub(super) fn inspect(
    file: &SourceFile,
    source: &RustSource,
    retained: &RetainedTypes,
    context: Option<&Context<'_>>,
    violations: &mut Vec<Violation>,
) {
    inspect_reexports(file, source, retained, context, violations);
    syntax::inspect(
        file,
        source,
        retained,
        &|declaration, public, range| {
            context.is_some_and(|value| value.allows_field(declaration, public, range))
        },
        violations,
    );
    inspect_functions(file, source, retained, context, violations);
    inspect_static_items(file, source, retained, context, violations);
    inspect_traits(file, source, retained, context, violations);
    api::inspect_associated_types(
        file,
        source,
        retained,
        &|surface| allows_associated(context, surface),
        violations,
    );
}

fn inspect_reexports(
    file: &SourceFile,
    source: &RustSource,
    retained: &RetainedTypes,
    context: Option<&Context<'_>>,
    violations: &mut Vec<Violation>,
) {
    for (index, token) in source.tokens.iter().enumerate() {
        if !token.identifier(USE_KEYWORD) || !syntax::item_visible(&source.tokens, index) {
            continue;
        }
        let end = syntax::statement_end(&source.tokens, index, source.tokens.len());
        let range = index..end;
        let forbidden = range.clone().find(|candidate| {
            retained.contains(&source.tokens, *candidate)
                && identifier(&source.tokens[*candidate])
                    .is_none_or(|name| !sites::allows_type(&file.relative, name))
        });
        let Some(forbidden) = forbidden else {
            continue;
        };
        if !allows(context, index, range) {
            syntax::violation(file, &source.tokens[forbidden], violations);
        }
    }
}

fn inspect_functions(
    file: &SourceFile,
    source: &RustSource,
    retained: &RetainedTypes,
    context: Option<&Context<'_>>,
    violations: &mut Vec<Violation>,
) {
    for function in source.functions() {
        let Some(header) = syntax::function_header(&source.tokens, function.body.start) else {
            continue;
        };
        if syntax::item_visible(&source.tokens, function.body.start)
            && contains(context, retained, &source.tokens, header.clone())
            && !api::allows_surface(&file.relative, source, function)
            && !allows(context, function.body.start, header)
        {
            syntax::violation(file, &source.tokens[function.body.start], violations);
        }
    }
}

fn inspect_static_items(
    file: &SourceFile,
    source: &RustSource,
    retained: &RetainedTypes,
    context: Option<&Context<'_>>,
    violations: &mut Vec<Violation>,
) {
    for (index, token) in source.tokens.iter().enumerate() {
        if (!token.identifier(CONST_KEYWORD) && !token.identifier(STATIC_KEYWORD))
            || !syntax::item_visible(&source.tokens, index)
            || const_function(&source.tokens, index)
        {
            continue;
        }
        let end = syntax::statement_end(&source.tokens, index, source.tokens.len());
        let type_end = source.tokens[index..end]
            .iter()
            .position(|candidate| candidate.symbol(ASSIGNMENT))
            .map_or(end, |offset| index + offset);
        let range = index..type_end;
        if contains(context, retained, &source.tokens, range.clone())
            && !allows(context, index, range)
        {
            syntax::violation(file, token, violations);
        }
    }
}

fn inspect_traits(
    file: &SourceFile,
    source: &RustSource,
    retained: &RetainedTypes,
    context: Option<&Context<'_>>,
    violations: &mut Vec<Violation>,
) {
    for (index, token) in source.tokens.iter().enumerate() {
        if !token.identifier(TRAIT_KEYWORD) || !syntax::item_visible(&source.tokens, index) {
            continue;
        }
        let Some((open, close)) = api::block(&source.tokens, index) else {
            continue;
        };
        let range = open + NEXT_TOKEN_OFFSET..close;
        if contains(context, retained, &source.tokens, range.clone())
            && !allows(context, index, range)
        {
            syntax::violation(file, token, violations);
        }
    }
}

fn allows_associated(context: Option<&Context<'_>>, surface: &api::AssociatedSurface) -> bool {
    context.is_some_and(|value| {
        if surface.trait_impl {
            value.allows_trait_associated(
                surface.implementation,
                surface.open,
                surface.range.clone(),
            )
        } else {
            value.allows_surface(surface.item, surface.range.clone())
        }
    })
}

fn contains(
    context: Option<&Context<'_>>,
    retained: &RetainedTypes,
    tokens: &[Token],
    range: Range<usize>,
) -> bool {
    match context {
        Some(value) => value.contains_reference(range),
        None => range.clone().any(|index| retained.contains(tokens, index)),
    }
}

fn allows(context: Option<&Context<'_>>, index: usize, range: Range<usize>) -> bool {
    context.is_some_and(|value| value.allows_surface(index, range))
}

fn const_function(tokens: &[Token], index: usize) -> bool {
    tokens[index + NEXT_TOKEN_OFFSET..]
        .iter()
        .take_while(|token| !token.symbol(GROUP_OPEN) && !token.symbol(TYPE_SEPARATOR))
        .any(|token| token.identifier(FUNCTION_KEYWORD))
}
