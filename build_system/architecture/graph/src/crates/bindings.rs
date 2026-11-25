use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use yolo_architecture_syntax::cargo_starlark_tokens::Token;

const EMPTY_DEPTH: usize = 0;
const FIRST_SOURCE_LINE: usize = 1;
const LIST_BOUNDARY_WIDTH: usize = 2;
const NEXT_TOKEN_OFFSET: usize = 1;
const VALUE_OFFSET: usize = 2;

#[derive(Clone)]
enum StaticValue {
    String(String),
    Strings(Vec<String>),
}

#[derive(Default)]
pub(super) struct Bindings(BTreeMap<String, Option<StaticValue>>);

impl Bindings {
    pub(super) fn parse(tokens: &[Token], source: &str) -> Self {
        let top_level = top_level_lines(source);
        let mut result = Self::default();
        let mut depth = EMPTY_DEPTH;
        for (index, token) in tokens.iter().enumerate() {
            if depth == 0 && top_level.contains(&token.line) {
                result.inspect(tokens, index, &top_level);
            }
            depth += usize::from(opens(token));
            depth = depth.saturating_sub(usize::from(closes(token)));
        }
        result
    }

    pub(super) fn string(&self, tokens: &[Token]) -> Option<String> {
        if tokens.len() != 1 {
            return None;
        }
        tokens[0].string().map(str::to_owned).or_else(|| {
            let name = tokens[0].identifier()?;
            match self.0.get(name)?.as_ref()? {
                StaticValue::String(value) => Some(value.clone()),
                StaticValue::Strings(_) => None,
            }
        })
    }

    pub(super) fn strings(&self, tokens: &[Token]) -> Option<Vec<String>> {
        string_list(tokens).or_else(|| {
            if tokens.len() != 1 {
                return None;
            }
            let name = tokens[0].identifier()?;
            match self.0.get(name)?.as_ref()? {
                StaticValue::Strings(values) => Some(values.clone()),
                StaticValue::String(_) => None,
            }
        })
    }

    fn inspect(&mut self, tokens: &[Token], index: usize, top_level: &BTreeSet<usize>) {
        let Some(name) = tokens[index]
            .identifier()
            .filter(|name| constant_name(name))
        else {
            return;
        };
        let next = index + NEXT_TOKEN_OFFSET;
        let (assignment, direct) = if tokens.get(next).is_some_and(|token| token.symbol('=')) {
            (next, true)
        } else if tokens
            .get(index + VALUE_OFFSET)
            .is_some_and(|token| token.symbol('='))
        {
            (index + VALUE_OFFSET, false)
        } else {
            return;
        };
        let start = assignment + NEXT_TOKEN_OFFSET;
        let end = binding_end(tokens, start, top_level);
        let value = direct.then(|| static_value(&tokens[start..end])).flatten();
        self.0
            .entry(name.to_owned())
            .and_modify(|current| *current = None)
            .or_insert(value);
    }
}

pub(super) fn attributes(
    tokens: &[Token],
    open: usize,
    close: usize,
    expected: &str,
) -> Vec<Range<usize>> {
    let mut found = Vec::new();
    let mut start = open + NEXT_TOKEN_OFFSET;
    let mut depth = EMPTY_DEPTH;
    for index in start..=close {
        let boundary = index == close || depth == EMPTY_DEPTH && tokens[index].symbol(',');
        if boundary {
            if tokens.get(start).and_then(Token::identifier) == Some(expected)
                && tokens
                    .get(start + NEXT_TOKEN_OFFSET)
                    .is_some_and(|token| token.symbol('='))
            {
                found.push(start + VALUE_OFFSET..index);
            }
            start = index + NEXT_TOKEN_OFFSET;
        } else {
            depth += usize::from(opens(&tokens[index]));
            depth = depth.saturating_sub(usize::from(closes(&tokens[index])));
        }
    }
    found
}

fn binding_end(tokens: &[Token], start: usize, top_level: &BTreeSet<usize>) -> usize {
    let Some(first) = tokens.get(start) else {
        return start;
    };
    let mut depth = EMPTY_DEPTH;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if index > start && depth == 0 && token.line > first.line && top_level.contains(&token.line)
        {
            return index;
        }
        depth += usize::from(opens(token));
        depth = depth.saturating_sub(usize::from(closes(token)));
    }
    tokens.len()
}

fn static_value(tokens: &[Token]) -> Option<StaticValue> {
    if tokens.len() == 1 {
        return tokens[0]
            .string()
            .map(|value| StaticValue::String(value.to_owned()));
    }
    string_list(tokens).map(StaticValue::Strings)
}

fn string_list(tokens: &[Token]) -> Option<Vec<String>> {
    if tokens.len() < LIST_BOUNDARY_WIDTH
        || !tokens.first()?.symbol('[')
        || !tokens.last()?.symbol(']')
    {
        return None;
    }
    let mut values = Vec::new();
    let mut cursor = NEXT_TOKEN_OFFSET;
    let end = tokens.len() - NEXT_TOKEN_OFFSET;
    while cursor < end {
        values.push(tokens[cursor].string()?.to_owned());
        cursor += NEXT_TOKEN_OFFSET;
        if cursor == end {
            break;
        }
        if !tokens[cursor].symbol(',') {
            return None;
        }
        cursor += NEXT_TOKEN_OFFSET;
    }
    Some(values)
}

fn top_level_lines(source: &str) -> BTreeSet<usize> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            line.chars()
                .next()
                .is_none_or(|character| !character.is_whitespace())
        })
        .map(|(index, _)| index + FIRST_SOURCE_LINE)
        .collect()
}

fn constant_name(name: &str) -> bool {
    name.chars()
        .all(|character| character == '_' || character.is_ascii_uppercase())
}

fn opens(token: &Token) -> bool {
    ['(', '[', '{'].iter().any(|symbol| token.symbol(*symbol))
}

fn closes(token: &Token) -> bool {
    [')', ']', '}'].iter().any(|symbol| token.symbol(*symbol))
}

#[cfg(test)]
#[path = "../../unit_tests/static_binding_boundaries.rs"]
mod tests;
