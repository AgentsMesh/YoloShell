use crate::cargo_starlark_tokens::{Token, tokenize};
use crate::policy_constants::STARLARK_COMMAND_ATTRIBUTES;
use std::collections::BTreeMap;

const COMMAND_BOUNDARY: char = ';';

pub fn execution_lines(source: &str) -> Vec<usize> {
    let tokens = tokenize(source);
    let variables = variables(&tokens);
    let mut result = Vec::new();
    for index in 0..tokens.len() {
        let Some(name) = tokens[index].identifier() else {
            continue;
        };
        if !STARLARK_COMMAND_ATTRIBUTES.contains(&name)
            || !tokens.get(index + 1).is_some_and(|token| token.symbol('='))
        {
            continue;
        }
        let end = expression_end(&tokens, index + 2);
        let command = expression_value(&tokens[index + 2..end], &variables);
        if !crate::cargo_shell::execution_lines(&command).is_empty() {
            result.push(tokens[index].line);
        }
    }
    result
}

fn variables(tokens: &[Token]) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    for index in 0..tokens.len() {
        let Some(name) = tokens[index].identifier() else {
            continue;
        };
        if !constant_name(name) || !tokens.get(index + 1).is_some_and(|token| token.symbol('=')) {
            continue;
        }
        let end = expression_end(tokens, index + 2);
        let value = expression_value(&tokens[index + 2..end], &result);
        result.insert(name.to_owned(), value);
    }
    result
}

fn expression_value(tokens: &[Token], variables: &BTreeMap<String, String>) -> String {
    let mut result = String::new();
    let mut previous = None;
    for (index, token) in tokens.iter().enumerate() {
        let value = token.string().map(str::to_owned).or_else(|| {
            token
                .identifier()
                .and_then(|name| variables.get(name).cloned())
        });
        let Some(value) = value else {
            continue;
        };
        if let Some(previous) = previous
            && !concatenated(tokens, previous, index)
        {
            result.push(COMMAND_BOUNDARY);
        }
        result.push_str(&value);
        previous = Some(index);
    }
    result
}

fn concatenated(tokens: &[Token], previous: usize, current: usize) -> bool {
    let between = &tokens[previous + 1..current];
    between.iter().any(|token| token.symbol('+'))
        && between
            .iter()
            .all(|token| token.symbol('+') || token.symbol('(') || token.symbol(')'))
}

fn expression_end(tokens: &[Token], start: usize) -> usize {
    let mut depth = 0;
    let start_line = tokens.get(start).map_or(0, |token| token.line);
    for index in start..tokens.len() {
        if tokens[index].symbol('(') || tokens[index].symbol('[') || tokens[index].symbol('{') {
            depth += 1;
        } else if tokens[index].symbol(')')
            || tokens[index].symbol(']')
            || tokens[index].symbol('}')
        {
            if depth == 0 {
                return index;
            }
            depth -= 1;
        } else if depth == 0 && tokens[index].symbol(',') {
            return index;
        } else if depth == 0
            && tokens[index].line > start_line
            && !tokens[index.saturating_sub(1)].symbol('+')
        {
            return index;
        }
    }
    tokens.len()
}

fn constant_name(name: &str) -> bool {
    name.bytes()
        .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
#[path = "../unit_tests/cargo_starlark_boundaries.rs"]
mod tests;
