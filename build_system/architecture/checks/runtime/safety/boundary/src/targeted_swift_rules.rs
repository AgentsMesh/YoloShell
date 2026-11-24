use crate::targeted_regression_rules::PIPE_DESCRIPTOR_MESSAGE;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_syntax::targeted_expression::evaluate;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const PIPE_DESCRIPTOR_COUNT: u64 = 2;
const PIPE_DESCRIPTOR_INDEXES: &[u64] = &[0, 1];

pub fn inspect_pipe_descriptors(
    file: &SourceFile,
    tokens: &[Token],
    violations: &mut Vec<Violation>,
) {
    for index in 0..tokens.len() {
        if let Some(line) = raw_descriptor_count(tokens, index).or_else(|| raw_index(tokens, index))
        {
            violations.push(Violation::line(
                &file.relative,
                line,
                PIPE_DESCRIPTOR_MESSAGE,
            ));
        }
    }
}

fn raw_descriptor_count(tokens: &[Token], index: usize) -> Option<usize> {
    if !tokens[index].identifier("count")
        || !tokens.get(index + 1).is_some_and(|token| token.symbol(":"))
        || !descriptor_declaration_before(tokens, index)
    {
        return None;
    }
    let end = (index + 2..tokens.len())
        .find(|cursor| tokens[*cursor].symbol(",") || tokens[*cursor].symbol(")"))?;
    (evaluate(&tokens[index + 2..end]) == Some(PIPE_DESCRIPTOR_COUNT))
        .then_some(tokens[index + 2].line)
}

fn raw_index(tokens: &[Token], index: usize) -> Option<usize> {
    if !tokens[index].identifier_contains("descriptor")
        || !tokens.get(index + 1).is_some_and(|token| token.symbol("["))
    {
        return None;
    }
    let end = closing(tokens, index + 1, "[", "]")?;
    let expression = &tokens[index + 2..swift_cast_start(tokens, index + 2, end)];
    evaluate(expression)
        .filter(|value| PIPE_DESCRIPTOR_INDEXES.contains(value))
        .map(|_| tokens[index + 2].line)
}

fn descriptor_declaration_before(tokens: &[Token], index: usize) -> bool {
    let start = (0..index)
        .rev()
        .find(|cursor| tokens[*cursor].symbol(";") || tokens[*cursor].symbol("{"))
        .map_or(0, |cursor| cursor + 1);
    tokens[start..index]
        .iter()
        .any(|token| token.identifier_contains("descriptor"))
        && tokens[start..index]
            .iter()
            .any(|token| token.identifier("Int32"))
}

fn swift_cast_start(tokens: &[Token], start: usize, end: usize) -> usize {
    (start..end)
        .find(|index| tokens[*index].identifier("as"))
        .unwrap_or(end)
}

#[cfg(all(test, feature = "yolo-runtime-boundary-tests"))]
#[path = "../unit_tests/swift_descriptor_boundary_tests.rs"]
mod boundary_tests;
