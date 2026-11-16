use crate::targeted_regression_rules::FRAME_WIDTH_MESSAGE;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::constants::FRAME_RANGE_CALLS;
use yolo_architecture_syntax::targeted_expression::{contains_value, evaluate};
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const FRAME_LENGTH_BYTES: u64 = 4;
const FRAME_TERMS: &[&str] = &["buffer", "frame", "header", "length", "payload"];

pub fn inspect(file: &SourceFile, tokens: &[Token], violations: &mut Vec<Violation>) {
    for index in 0..tokens.len() {
        let detected = frame_capacity(tokens, index)
            .or_else(|| frame_array(tokens, index))
            .or_else(|| frame_range(tokens, index))
            .or_else(|| frame_range_call(tokens, index));
        if let Some(line) = detected {
            violations.push(Violation::line(&file.relative, line, FRAME_WIDTH_MESSAGE));
        }
    }
}

fn frame_capacity(tokens: &[Token], index: usize) -> Option<usize> {
    if !tokens[index].identifier("with_capacity")
        || !tokens.get(index + 1).is_some_and(|token| token.symbol("("))
    {
        return None;
    }
    let end = closing(tokens, index + 1, "(", ")")?;
    let argument = &tokens[index + 2..end];
    (semantic(argument) && contains_value(argument, FRAME_LENGTH_BYTES))
        .then_some(tokens[index].line)
}

fn frame_array(tokens: &[Token], index: usize) -> Option<usize> {
    if !tokens[index].symbol("[") || !semantic_before(tokens, index) {
        return None;
    }
    let end = closing(tokens, index, "[", "]")?;
    let separator = tokens[index + 1..end]
        .iter()
        .position(|token| token.symbol(";"))?
        + index
        + 1;
    (evaluate(&tokens[separator + 1..end]) == Some(FRAME_LENGTH_BYTES))
        .then_some(tokens[separator + 1].line)
}

fn frame_range(tokens: &[Token], index: usize) -> Option<usize> {
    if !tokens[index].symbol("[") || !semantic_near(tokens, index) {
        return None;
    }
    let end = closing(tokens, index, "[", "]")?;
    raw_range(&tokens[index + 1..end]).then_some(tokens[index].line)
}

fn frame_range_call(tokens: &[Token], index: usize) -> Option<usize> {
    if !FRAME_RANGE_CALLS
        .iter()
        .any(|name| tokens[index].identifier(name))
        || !semantic_near(tokens, index)
        || !tokens.get(index + 1).is_some_and(|token| token.symbol("("))
    {
        return None;
    }
    let end = closing(tokens, index + 1, "(", ")")?;
    raw_range(&tokens[index + 2..end]).then_some(tokens[index].line)
}

fn raw_range(tokens: &[Token]) -> bool {
    let Some(range) = tokens.iter().position(|token| token.symbol("..")) else {
        return false;
    };
    evaluate(&tokens[..range]) == Some(FRAME_LENGTH_BYTES)
        || evaluate(&tokens[range + 1..]) == Some(FRAME_LENGTH_BYTES)
}

fn semantic(tokens: &[Token]) -> bool {
    tokens.iter().any(|token| {
        FRAME_TERMS
            .iter()
            .any(|term| token.identifier_contains(term))
    })
}

fn semantic_before(tokens: &[Token], index: usize) -> bool {
    let start = statement_start(tokens, index);
    semantic(&tokens[start..index])
}

fn semantic_near(tokens: &[Token], index: usize) -> bool {
    semantic(&tokens[index.saturating_sub(4)..index])
}

fn statement_start(tokens: &[Token], index: usize) -> usize {
    (0..index)
        .rev()
        .find(|cursor| tokens[*cursor].symbol(";") || tokens[*cursor].symbol("{"))
        .map_or(0, |cursor| cursor + 1)
}
