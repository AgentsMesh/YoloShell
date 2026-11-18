use super::Depth;
use std::ops::Range;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const ASSIGNMENT: &str = "=";
const FAT_ARROW_END: &str = ">";
const ITEM_END: &str = ";";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const ARGUMENT_SEPARATOR: &str = ",";
const MEMBER_ACCESS: &str = ".";
const TRY_OPERATOR: &str = "?";
const FOR_KEYWORD: &str = "for";
const IN_KEYWORD: &str = "in";
const MATCH_KEYWORD: &str = "match";
const ELSE_KEYWORD: &str = "else";
const IF_KEYWORD: &str = "if";

#[path = "patterns/assignments.rs"]
mod assignments;
#[path = "patterns/closures.rs"]
mod closures;

pub struct Flow {
    pub pattern: Range<usize>,
    pub source: Range<usize>,
}

pub fn flows(tokens: &[Token], scope: Range<usize>) -> Vec<Flow> {
    let mut result = assignments::flows(tokens, &scope);
    for index in scope.clone() {
        if tokens[index].identifier(FOR_KEYWORD)
            && let Some(flow) = for_flow(tokens, index, &scope)
        {
            result.push(flow);
        }
        if tokens[index].identifier(MATCH_KEYWORD) {
            result.extend(match_flows(tokens, index, &scope));
        }
    }
    result.extend(closures::flows(tokens, &scope));
    result
}

fn for_flow(tokens: &[Token], start: usize, scope: &Range<usize>) -> Option<Flow> {
    let separator = top_level_identifier(tokens, start + 1, scope.end, IN_KEYWORD)?;
    let body = control_body(tokens, separator + 1, scope.end)?;
    Some(Flow {
        pattern: start + 1..separator,
        source: separator + 1..body,
    })
}

fn match_flows(tokens: &[Token], start: usize, scope: &Range<usize>) -> Vec<Flow> {
    let Some(open) = match_body(tokens, start + 1, scope.end) else {
        return Vec::new();
    };
    let Some(close) = closing(tokens, open, BLOCK_OPEN, BLOCK_CLOSE) else {
        return Vec::new();
    };
    let source = start + 1..open;
    let mut arm_start = open + 1;
    let mut depth = Depth::default();
    let mut result = Vec::new();
    for index in open + 1..close {
        if depth.is_top()
            && tokens[index].symbol(ASSIGNMENT)
            && tokens
                .get(index + 1)
                .is_some_and(|token| token.symbol(FAT_ARROW_END))
        {
            let pattern_end = arm_pattern_end(tokens, arm_start, index);
            result.push(Flow {
                pattern: arm_start..pattern_end,
                source: source.clone(),
            });
        } else if depth.is_top() && tokens[index].symbol(ARGUMENT_SEPARATOR) {
            arm_start = index + 1;
        }
        depth.observe(&tokens[index]);
    }
    result
}

fn arm_pattern_end(tokens: &[Token], start: usize, end: usize) -> usize {
    let mut depth = Depth::default();
    for index in start..end {
        if depth.is_top()
            && index > start
            && index + 1 < end
            && tokens[index].identifier(IF_KEYWORD)
        {
            return index;
        }
        depth.observe(&tokens[index]);
    }
    end
}

fn top_level_identifier(
    tokens: &[Token],
    start: usize,
    upper: usize,
    expected: &str,
) -> Option<usize> {
    let mut depth = Depth::default();
    for index in start..upper {
        if depth.is_top() && tokens[index].identifier(expected) {
            return Some(index);
        }
        depth.observe(&tokens[index]);
    }
    None
}

pub(super) fn statement_start(tokens: &[Token], lower: usize, end: usize) -> usize {
    let mut blocks = 0_usize;
    for index in (lower..end).rev() {
        if tokens[index].symbol(BLOCK_CLOSE) {
            blocks += 1;
        } else if tokens[index].symbol(BLOCK_OPEN) && blocks > 0 {
            blocks -= 1;
        } else if blocks == 0
            && (tokens[index].symbol(ITEM_END) || tokens[index].symbol(BLOCK_OPEN))
        {
            return index + 1;
        }
    }
    lower
}

pub(super) fn statement_end(tokens: &[Token], start: usize, upper: usize) -> usize {
    let mut depth = Depth::default();
    for index in start..upper {
        if depth.is_top() && tokens[index].symbol(ITEM_END) {
            return index;
        }
        depth.observe(&tokens[index]);
    }
    upper
}

fn next_symbol(tokens: &[Token], start: usize, upper: usize, symbol: &str) -> Option<usize> {
    (start..upper).find(|index| tokens[*index].symbol(symbol))
}

pub(super) fn control_body(tokens: &[Token], start: usize, upper: usize) -> Option<usize> {
    let mut cursor = start;
    loop {
        let open = next_symbol(tokens, cursor, upper, BLOCK_OPEN)?;
        let close = closing(tokens, open, BLOCK_OPEN, BLOCK_CLOSE)?;
        let continuation = tokens.get(close + 1).is_some_and(|token| {
            token.symbol(BLOCK_OPEN)
                || token.symbol(MEMBER_ACCESS)
                || token.symbol(TRY_OPERATOR)
                || token.identifier(ELSE_KEYWORD)
        });
        if !continuation {
            return Some(open);
        }
        cursor = close + 1;
    }
}

fn match_body(tokens: &[Token], start: usize, upper: usize) -> Option<usize> {
    let mut cursor = start;
    let mut body = None;
    let end = statement_end(tokens, start, upper);
    while let Some(open) = next_symbol(tokens, cursor, end, BLOCK_OPEN) {
        let close = closing(tokens, open, BLOCK_OPEN, BLOCK_CLOSE)?;
        if has_top_level_arrow(tokens, open + 1, close) {
            body = Some(open);
        }
        cursor = close + 1;
    }
    body
}

fn has_top_level_arrow(tokens: &[Token], start: usize, end: usize) -> bool {
    let mut depth = Depth::default();
    for index in start..end {
        if depth.is_top()
            && tokens[index].symbol(ASSIGNMENT)
            && tokens
                .get(index + 1)
                .is_some_and(|token| token.symbol(FAT_ARROW_END))
        {
            return true;
        }
        depth.observe(&tokens[index]);
    }
    false
}

#[cfg(test)]
#[path = "../../unit_tests/closure_flow_boundary_tests.rs"]
mod boundary_tests;
