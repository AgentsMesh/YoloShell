use std::collections::BTreeSet;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_policy_contracts::process_signal_retained as retained_constants;
use yolo_architecture_process_signal_path_analysis::types::RetainedTypes;
use yolo_architecture_process_signal_path_analysis::visibility_fields as visibility;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const MACRO_MARKER: &str = "!";
const META_MARKER: &str = "$";
const FRAGMENT_SEPARATOR: &str = ":";
const VISIBILITY_FRAGMENT: &str = "vis";
const IMPL_KEYWORD: &str = "impl";
const GROUP_OPEN: &str = "(";
const GROUP_CLOSE: &str = ")";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const ARRAY_OPEN: &str = "[";
const ARRAY_CLOSE: &str = "]";
const AUTHORITY_CAPABLE_FRAGMENTS: &[&str] = &["ty", "path", "tt"];

pub fn inspect(
    file: &SourceFile,
    source: &RustSource,
    retained: &RetainedTypes,
    violations: &mut Vec<Violation>,
) {
    for (marker, token) in source.tokens.iter().enumerate() {
        if !token.symbol(MACRO_MARKER) {
            continue;
        }
        let definition =
            marker > 0 && source.tokens[marker - 1].identifier(retained_constants::MACRO_RULES);
        let start = marker + if definition { 2 } else { 1 };
        let Some((open, close)) = delimited(&source.tokens, start) else {
            if definition {
                violation(file, token, violations);
            }
            continue;
        };
        let hazard = if definition {
            definition_hazard(&source.tokens, open + 1, close, retained)
        } else {
            (open + 1..close).find(|index| retained.contains(&source.tokens, *index))
        };
        if let Some(index) = hazard {
            violation(file, &source.tokens[index], violations);
        }
    }
}

fn definition_hazard(
    tokens: &[Token],
    start: usize,
    end: usize,
    retained: &RetainedTypes,
) -> Option<usize> {
    if let Some(index) = (start..end).find(|index| retained.contains(tokens, *index)) {
        return Some(index);
    }
    unprovable_export(tokens, start, end)
}

fn unprovable_export(tokens: &[Token], start: usize, end: usize) -> Option<usize> {
    let capable = fragment_names(tokens, start, end, AUTHORITY_CAPABLE_FRAGMENTS);
    let visibility = fragment_names(tokens, start, end, &[VISIBILITY_FRAGMENT]);
    let dynamic_visibility = metavariable_references(tokens, start, end, &visibility).next();
    if visibility::visible_token(tokens, start, end).is_none() && dynamic_visibility.is_none() {
        return None;
    }
    metavariable_references(tokens, start, end, &capable)
        .find(|index| !impl_owner_reference(tokens, *index, start))
}

fn fragment_names<'a>(
    tokens: &'a [Token],
    start: usize,
    end: usize,
    fragments: &[&str],
) -> BTreeSet<&'a str> {
    (start..end.saturating_sub(3))
        .filter_map(|index| {
            if !tokens[index].symbol(META_MARKER)
                || !tokens[index + 2].symbol(FRAGMENT_SEPARATOR)
                || !fragments
                    .iter()
                    .any(|fragment| tokens[index + 3].identifier(fragment))
            {
                return None;
            }
            identifier(&tokens[index + 1])
        })
        .collect()
}

fn metavariable_references<'a>(
    tokens: &'a [Token],
    start: usize,
    end: usize,
    names: &'a BTreeSet<&str>,
) -> impl Iterator<Item = usize> + 'a {
    (start..end.saturating_sub(1)).filter(|index| {
        let Some(name) = tokens.get(*index + 1).and_then(identifier) else {
            return false;
        };
        tokens[*index].symbol(META_MARKER)
            && names.contains(name)
            && !tokens
                .get(*index + 2)
                .is_some_and(|token| token.symbol(FRAGMENT_SEPARATOR))
    })
}

fn impl_owner_reference(tokens: &[Token], reference: usize, lower: usize) -> bool {
    tokens[lower..reference]
        .iter()
        .rev()
        .find_map(identifier)
        .is_some_and(|name| name == IMPL_KEYWORD)
}

fn delimited(tokens: &[Token], open: usize) -> Option<(usize, usize)> {
    let token = tokens.get(open)?;
    let (left, right) = if token.symbol(GROUP_OPEN) {
        (GROUP_OPEN, GROUP_CLOSE)
    } else if token.symbol(BLOCK_OPEN) {
        (BLOCK_OPEN, BLOCK_CLOSE)
    } else if token.symbol(ARRAY_OPEN) {
        (ARRAY_OPEN, ARRAY_CLOSE)
    } else {
        return None;
    };
    Some((open, closing(tokens, open, left, right)?))
}

fn violation(file: &SourceFile, token: &Token, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(
        &file.relative,
        token.line,
        contract::AUTHORITY_TYPE_MESSAGE,
    ));
}

#[cfg(test)]
#[path = "../unit_tests/macro_authority_boundary_tests.rs"]
mod tests;
