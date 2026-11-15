use super::{Flow, control_body, statement_end, statement_start};
use std::ops::Range;
use yolo_architecture_syntax::targeted_tokens::Token;

const ASSIGNMENT: &str = "=";
const FAT_ARROW_END: &str = ">";
const LET_KEYWORD: &str = "let";
const IF_KEYWORD: &str = "if";
const WHILE_KEYWORD: &str = "while";

pub fn flows(tokens: &[Token], scope: &Range<usize>) -> Vec<Flow> {
    scope
        .clone()
        .filter(|index| assignment(tokens, *index))
        .map(|assignment| {
            let pattern_start = pattern_start(tokens, scope.start, assignment);
            let pattern = pattern_start..assignment;
            let source_end = if conditional(tokens, pattern_start) {
                control_body(tokens, assignment + 1, scope.end).unwrap_or(scope.end)
            } else {
                statement_end(tokens, assignment + 1, scope.end)
            };
            Flow {
                pattern,
                source: assignment + 1..source_end,
            }
        })
        .collect()
}

fn assignment(tokens: &[Token], index: usize) -> bool {
    tokens[index].symbol(ASSIGNMENT)
        && !tokens
            .get(index + 1)
            .is_some_and(|token| token.symbol(FAT_ARROW_END))
}

fn pattern_start(tokens: &[Token], lower: usize, assignment: usize) -> usize {
    let statement = statement_start(tokens, lower, assignment);
    if tokens[statement].identifier(LET_KEYWORD) {
        return statement + 1;
    }
    if tokens[statement].identifier(IF_KEYWORD) || tokens[statement].identifier(WHILE_KEYWORD) {
        return (statement + 1..assignment)
            .find(|index| tokens[*index].identifier(LET_KEYWORD))
            .map_or(statement, |index| index + 1);
    }
    statement
}

fn conditional(tokens: &[Token], pattern_start: usize) -> bool {
    pattern_start > 1
        && tokens[pattern_start - 1].identifier(LET_KEYWORD)
        && (tokens[pattern_start - 2].identifier(IF_KEYWORD)
            || tokens[pattern_start - 2].identifier(WHILE_KEYWORD))
}
