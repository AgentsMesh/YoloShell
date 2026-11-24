use std::collections::BTreeMap;
use yolo_architecture_syntax::cargo_starlark_tokens::Token;

const ARGUMENT_SEPARATOR: char = ',';
const ASSIGNMENT: char = '=';
const DEFINITION_KEYWORD: &str = "def";
const EMPTY_DEPTH: usize = 0;
const LOAD_KEYWORD: &str = "load";
const NEXT_TOKEN_OFFSET: usize = 1;
const RULES_RUST_DEFINITIONS: &str = "@rules_rust//rust:defs.bzl";
const SOURCE_OFFSET: usize = 1;
const VALUE_OFFSET: usize = 2;

#[derive(Default)]
pub struct RuleOrigins {
    aliases: BTreeMap<String, String>,
    uncertain: bool,
}

impl RuleOrigins {
    pub fn parse(tokens: &[Token]) -> Self {
        let mut origins = Self::default();
        for (open, close) in calls(tokens) {
            if tokens[open - NEXT_TOKEN_OFFSET].identifier() == Some(LOAD_KEYWORD) {
                origins.inspect_load(tokens, open, close);
            }
        }
        origins
    }

    pub fn canonical<'a>(&'a self, name: &'a str) -> &'a str {
        self.aliases.get(name).map_or(name, String::as_str)
    }

    pub fn uncertain(&self) -> bool {
        self.uncertain
    }

    fn inspect_load(&mut self, tokens: &[Token], open: usize, close: usize) {
        let trusted = tokens
            .get(open + SOURCE_OFFSET)
            .and_then(Token::string)
            .is_some_and(|source| source == RULES_RUST_DEFINITIONS);
        let mut cursor = open + VALUE_OFFSET;
        while cursor < close {
            if tokens[cursor].symbol(ARGUMENT_SEPARATOR) {
                cursor += NEXT_TOKEN_OFFSET;
                continue;
            }
            let Some((local, original, next)) = load_binding(tokens, cursor, close) else {
                self.uncertain |= trusted;
                return;
            };
            let rust_rule = super::labels::default_root(original).is_some();
            if !trusted {
                self.uncertain |= rust_rule;
            } else if self
                .aliases
                .insert(local.to_owned(), original.to_owned())
                .is_some()
            {
                self.uncertain = true;
            }
            cursor = next;
        }
    }
}

pub(super) fn calls(tokens: &[Token]) -> Vec<(usize, usize)> {
    tokens
        .iter()
        .enumerate()
        .filter(|(open, token)| token.symbol('(') && *open > EMPTY_DEPTH)
        .filter(|(open, _)| tokens[*open - NEXT_TOKEN_OFFSET].identifier().is_some())
        .filter(|(open, _)| {
            open.checked_sub(VALUE_OFFSET)
                .and_then(|index| tokens.get(index))
                .and_then(Token::identifier)
                != Some(DEFINITION_KEYWORD)
        })
        .filter_map(|(open, _)| closing(tokens, open).map(|close| (open, close)))
        .collect()
}

fn closing(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = EMPTY_DEPTH;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        depth += usize::from(token.symbol('('));
        if token.symbol(')') {
            depth = depth.checked_sub(NEXT_TOKEN_OFFSET)?;
            if depth == EMPTY_DEPTH {
                return Some(index);
            }
        }
    }
    None
}

fn load_binding(tokens: &[Token], start: usize, close: usize) -> Option<(&str, &str, usize)> {
    if let Some(original) = tokens.get(start)?.string() {
        return Some((original, original, start + NEXT_TOKEN_OFFSET));
    }
    let local = tokens.get(start)?.identifier()?;
    tokens
        .get(start + NEXT_TOKEN_OFFSET)?
        .symbol(ASSIGNMENT)
        .then_some(())?;
    let original = tokens.get(start + VALUE_OFFSET)?.string()?;
    (start + VALUE_OFFSET < close).then_some((local, original, start + VALUE_OFFSET + 1))
}

pub fn direct_call_receiver(tokens: &[Token], cursor: usize, expected: &str) -> bool {
    let Some(close) = cursor.checked_sub(VALUE_OFFSET) else {
        return false;
    };
    if !tokens[close].symbol(')') {
        return false;
    }
    let mut depth = NEXT_TOKEN_OFFSET;
    for index in (0..close).rev() {
        depth += usize::from(tokens[index].symbol(')'));
        depth = depth.saturating_sub(usize::from(tokens[index].symbol('(')));
        if depth == EMPTY_DEPTH {
            return index > EMPTY_DEPTH
                && tokens[index - NEXT_TOKEN_OFFSET].identifier() == Some(expected);
        }
    }
    false
}
