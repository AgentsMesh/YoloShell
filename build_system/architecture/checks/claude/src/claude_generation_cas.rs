use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use crate::claude_generation_constants::*;
use yolo_architecture_syntax::targeted_tokens::{Kind, Token, tokenize};

#[path = "claude_generation_cas_sites.rs"]
mod sites;

const EXPECTED_OCCURRENCES: usize = 1;
const POLICY_MESSAGE: &str =
    "Claude generation mutation capability is restricted to its exact generation owners";
const CONTRACT_MESSAGE: &str =
    "Claude generation mutation capability contract changed unexpectedly";

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Occurrence {
    Acquire,
    Borrow,
}

pub(super) fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let mut definitions = 0;
    let mut implementations = 0;
    let mut counts = vec![0_usize; sites::SITES.len()];
    for file in files.iter().filter(|file| file.relative.ends_with(".rs")) {
        inspect_file(
            file,
            &mut definitions,
            &mut implementations,
            &mut counts,
            violations,
        );
    }
    let generation_owner_exists = files.iter().any(|file| {
        file.relative == CLAUDE_GENERATION_MUTATION_PATH
            || file.relative == CLAUDE_GENERATION_STORE_PATH
    });
    if generation_owner_exists
        && (definitions != EXPECTED_OCCURRENCES
            || implementations != EXPECTED_OCCURRENCES
            || counts.iter().any(|count| *count != EXPECTED_OCCURRENCES))
    {
        violations.push(Violation::file(
            CLAUDE_GENERATION_MUTATION_PATH,
            CONTRACT_MESSAGE,
        ));
    }
}

fn inspect_file(
    file: &SourceFile,
    definitions: &mut usize,
    implementations: &mut usize,
    counts: &mut [usize],
    violations: &mut Vec<Violation>,
) {
    let tokens = tokenize(&file.text);
    for (index, token) in tokens.iter().enumerate() {
        if !token.identifier(CLAUDE_GENERATION_MUTATION_TYPE) {
            continue;
        }
        if file.relative == CLAUDE_GENERATION_MUTATION_PATH && is_private_definition(&tokens, index)
        {
            *definitions += 1;
        } else if file.relative == CLAUDE_GENERATION_MUTATION_PATH
            && is_implementation(&tokens, index)
        {
            *implementations += 1;
        } else if let Some((site_index, _)) = sites::SITES.iter().enumerate().find(|(_, site)| {
            site.path == file.relative
                && function_context(&tokens, index) == Some(site.function)
                && matches_occurrence(&tokens, index, site.occurrence)
        }) {
            counts[site_index] += 1;
        } else {
            violations.push(Violation::line(&file.relative, token.line, POLICY_MESSAGE));
        }
    }
}

fn is_private_definition(tokens: &[Token], index: usize) -> bool {
    index >= 5
        && tokens[index - 5].identifier("pub")
        && tokens[index - 4].symbol("(")
        && tokens[index - 3].identifier("super")
        && tokens[index - 2].symbol(")")
        && tokens[index - 1].identifier("struct")
}

fn is_implementation(tokens: &[Token], index: usize) -> bool {
    tokens[..index]
        .iter()
        .rev()
        .take_while(|token| ![";", "{", "}"].iter().any(|mark| token.symbol(mark)))
        .any(|token| token.identifier("impl"))
}

fn matches_occurrence(tokens: &[Token], index: usize, occurrence: Occurrence) -> bool {
    qualification_start(tokens, index).is_some_and(|start| match occurrence {
        Occurrence::Acquire => {
            tokens
                .get(index + 1)
                .is_some_and(|token| token.symbol("::"))
                && tokens
                    .get(index + 2)
                    .is_some_and(|token| token.identifier(CLAUDE_GENERATION_MUTATION_ACQUIRE))
                && tokens.get(index + 3).is_some_and(|token| token.symbol("("))
        }
        Occurrence::Borrow => start > 0 && tokens[start - 1].symbol("&"),
    })
}

fn qualification_start(tokens: &[Token], index: usize) -> Option<usize> {
    if index < 4 {
        return None;
    }
    if !tokens[index - 1].symbol("::")
        || !tokens[index - 2].identifier("mutation")
        || !tokens[index - 3].symbol("::")
    {
        return None;
    }
    let mut cursor = index - 4;
    if !tokens[cursor].identifier("super") {
        return None;
    }
    while cursor >= 2 && tokens[cursor - 1].symbol("::") && tokens[cursor - 2].identifier("super") {
        cursor -= 2;
    }
    Some(cursor)
}

fn function_context(tokens: &[Token], index: usize) -> Option<&str> {
    let mut depth = 0_usize;
    let mut pending = None;
    let mut functions = Vec::new();
    for (position, token) in tokens.iter().enumerate().take(index) {
        if token.identifier("fn") {
            pending = tokens.get(position + 1).and_then(identifier);
        } else if token.symbol(";") {
            pending = None;
        } else if token.symbol("{") {
            depth += 1;
            if let Some(name) = pending.take() {
                functions.push((depth, name));
            }
        } else if token.symbol("}") {
            if functions.last().is_some_and(|(body, _)| *body == depth) {
                functions.pop();
            }
            depth = depth.saturating_sub(1);
        }
    }
    functions.last().map(|(_, name)| *name).or(pending)
}

fn identifier(token: &Token) -> Option<&str> {
    match &token.kind {
        Kind::Identifier(value) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../unit_tests/claude_mutation_token_boundaries.rs"]
mod tests;
