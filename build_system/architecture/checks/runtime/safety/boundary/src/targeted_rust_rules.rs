use crate::targeted_regression_rules::{
    HASHED_HOST_MESSAGE, HEX_CAPACITY_MESSAGE, PERMISSION_MASK_MESSAGE,
};
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_syntax::targeted_expression::evaluate;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const HEX_DIGITS_PER_BYTE: u64 = 2;
const HEX_SHIFT: u64 = 1;
const HASHED_POSITION_CALLS: &[&str] = &["nth", "skip"];

pub fn inspect_permission_masks(
    file: &SourceFile,
    tokens: &[Token],
    violations: &mut Vec<Violation>,
) {
    for index in 0..tokens.len() {
        if !tokens[index].identifier("mode") || !symbols(tokens, index + 1, &["(", ")", "&"]) {
            continue;
        }
        let start = index + 4;
        let end = expression_end(tokens, start);
        if evaluate(&tokens[start..end]).is_some() {
            violations.push(Violation::line(
                &file.relative,
                tokens[start].line,
                PERMISSION_MASK_MESSAGE,
            ));
        }
    }
}

pub fn inspect_hashed_host_fields(
    file: &SourceFile,
    tokens: &[Token],
    violations: &mut Vec<Violation>,
) {
    for index in 0..tokens.len() {
        if !HASHED_POSITION_CALLS
            .iter()
            .any(|name| tokens[index].identifier(name))
            || index == 0
            || !tokens[index - 1].symbol(".")
            || !tokens.get(index + 1).is_some_and(|token| token.symbol("("))
        {
            continue;
        }
        let Some(end) = closing(tokens, index + 1, "(", ")") else {
            continue;
        };
        if evaluate(&tokens[index + 2..end]).is_some() {
            violations.push(Violation::line(
                &file.relative,
                tokens[index].line,
                HASHED_HOST_MESSAGE,
            ));
        }
    }
}

pub fn inspect_hex_capacities(
    file: &SourceFile,
    tokens: &[Token],
    violations: &mut Vec<Violation>,
) {
    for index in 0..tokens.len() {
        if !tokens[index].identifier("with_capacity")
            || !tokens.get(index + 1).is_some_and(|token| token.symbol("("))
        {
            continue;
        }
        let Some(end) = closing(tokens, index + 1, "(", ")") else {
            continue;
        };
        let argument = &tokens[index + 2..end];
        if semantic(argument, &["digest", "hash", "output_size"]) && raw_hex_scaling(argument) {
            violations.push(Violation::line(
                &file.relative,
                tokens[index].line,
                HEX_CAPACITY_MESSAGE,
            ));
        }
    }
}

fn raw_hex_scaling(tokens: &[Token]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        (token.symbol("*")
            && (numeric_side(tokens, index, false) == Some(HEX_DIGITS_PER_BYTE)
                || numeric_side(tokens, index, true) == Some(HEX_DIGITS_PER_BYTE)))
            || (token.symbol("<<") && numeric_side(tokens, index, false) == Some(HEX_SHIFT))
    })
}

fn numeric_side(tokens: &[Token], operator: usize, left: bool) -> Option<u64> {
    if left {
        (0..operator)
            .rev()
            .find_map(|start| evaluate(&tokens[start..operator]))
    } else {
        (operator + 1..tokens.len()).find_map(|end| evaluate(&tokens[operator + 1..=end]))
    }
}

fn expression_end(tokens: &[Token], start: usize) -> usize {
    (start..tokens.len())
        .find(|index| {
            ["==", "!=", ",", ";", "{", "}"]
                .iter()
                .any(|symbol| tokens[*index].symbol(symbol))
        })
        .unwrap_or(tokens.len())
}

fn semantic(tokens: &[Token], terms: &[&str]) -> bool {
    tokens
        .iter()
        .any(|token| terms.iter().any(|term| token.identifier_contains(term)))
}

fn symbols(tokens: &[Token], start: usize, expected: &[&str]) -> bool {
    expected.iter().enumerate().all(|(offset, symbol)| {
        tokens
            .get(start + offset)
            .is_some_and(|token| token.symbol(symbol))
    })
}
