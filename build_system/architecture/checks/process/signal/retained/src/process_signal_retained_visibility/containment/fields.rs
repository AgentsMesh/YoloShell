use super::Context;
use super::scope;
use std::ops::Range;
use yolo_architecture_process_signal_path_analysis::visibility_fields as syntax;
use yolo_architecture_syntax::rust_source::identifier;

const PUBLIC_KEYWORD: &str = "pub";
const UNIQUE_OWNER_COUNT: usize = 1;

pub(super) fn allowed(
    context: &Context<'_>,
    declaration: usize,
    public: usize,
    range: Range<usize>,
) -> bool {
    if !context.certain
        || !context
            .tokens
            .get(public)
            .is_some_and(|token| identifier(token) == Some(PUBLIC_KEYWORD))
    {
        return false;
    }
    let owners = context.modules.paths_at(context.relative, declaration);
    if owners.len() != UNIQUE_OWNER_COUNT {
        return false;
    }
    let Some(owner) = owners.first() else {
        return false;
    };
    let Some(container) = container_audience(context.tokens, declaration, owner) else {
        return false;
    };
    let Some(field) = scope::audience(context.tokens, public, owner) else {
        return false;
    };
    let Some(audience) = narrowest(&container, &field) else {
        return false;
    };
    context.allows_range(range, audience)
}

fn container_audience(
    tokens: &[yolo_architecture_syntax::targeted_tokens::Token],
    declaration: usize,
    owner: &[String],
) -> Option<Vec<String>> {
    let start = syntax::item_start(tokens, declaration);
    match syntax::visible_index(tokens, start, declaration) {
        Some(public) => scope::audience(tokens, public, owner),
        None => Some(owner.to_vec()),
    }
}

fn narrowest<'a>(left: &'a [String], right: &'a [String]) -> Option<&'a [String]> {
    if left.starts_with(right) {
        Some(left)
    } else if right.starts_with(left) {
        Some(right)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/containment_field_boundary_tests.rs"]
mod tests;
