use crate::cargo_starlark_tokens::{Token, tokenize};
use std::ops::Range;

const ASSIGNMENT: char = '=';
const CALL_CLOSE: char = ')';
const CALL_OPEN: char = '(';
const DICTIONARY_CLOSE: char = '}';
const DICTIONARY_OPEN: char = '{';
const EMPTY_DEPTH: usize = 0;
const KEYWORD_VALUE_OFFSET: usize = 2;
const LIST_BOUNDARY_WIDTH: usize = 2;
const LIST_CLOSE: char = ']';
const LIST_OPEN: char = '[';
const MEMBER_SEPARATOR: char = '.';
const NEXT_OFFSET: usize = 1;
const QUALIFIED_RECEIVER_OFFSET: usize = 4;
const SEPARATOR: char = ',';
const SINGLE_MATCH: usize = 1;

#[derive(Clone)]
pub struct Call {
    pub name: String,
    arguments: Range<usize>,
}

pub struct Keyword {
    pub name: String,
    pub value: Range<usize>,
}

pub struct Document {
    tokens: Vec<Token>,
}

impl Document {
    pub fn parse(source: &str) -> Self {
        Self {
            tokens: tokenize(source),
        }
    }

    pub fn calls(&self, name: &str) -> Vec<Call> {
        self.all_calls()
            .into_iter()
            .filter(|call| call.name == name)
            .collect()
    }

    pub fn all_calls(&self) -> Vec<Call> {
        self.tokens
            .iter()
            .enumerate()
            .filter_map(|(index, token)| {
                let name = token.identifier()?;
                let open = index + NEXT_OFFSET;
                self.tokens
                    .get(open)
                    .is_some_and(|token| token.symbol(CALL_OPEN))
                    .then_some((name, open))
            })
            .filter_map(|(name, open)| {
                closing(&self.tokens, open, CALL_OPEN, CALL_CLOSE).map(|close| Call {
                    name: name.to_owned(),
                    arguments: open + NEXT_OFFSET..close,
                })
            })
            .collect()
    }

    pub fn arguments(&self, call: &Call) -> Vec<Range<usize>> {
        split(&self.tokens, &call.arguments, SEPARATOR)
    }

    pub fn keywords(&self, call: &Call) -> Option<Vec<Keyword>> {
        self.arguments(call)
            .into_iter()
            .map(|range| {
                let name = self.tokens.get(range.start)?.identifier()?;
                self.tokens
                    .get(range.start + NEXT_OFFSET)
                    .is_some_and(|token| token.symbol(ASSIGNMENT))
                    .then_some(Keyword {
                        name: name.to_owned(),
                        value: range.start + KEYWORD_VALUE_OFFSET..range.end,
                    })
            })
            .collect()
    }

    pub fn keyword_string<'a>(&'a self, call: &Call, name: &str) -> Option<&'a str> {
        let range = self.unique_keyword(call, name)?;
        self.string(&range)
    }

    pub fn keyword_list<'a>(&'a self, call: &Call, name: &str) -> Option<Vec<&'a str>> {
        let range = self.unique_keyword(call, name)?;
        self.string_list(&range)
    }

    pub fn string<'a>(&'a self, range: &Range<usize>) -> Option<&'a str> {
        (range.len() == SINGLE_MATCH)
            .then(|| self.tokens[range.start].string())
            .flatten()
    }

    pub fn string_list<'a>(&'a self, range: &Range<usize>) -> Option<Vec<&'a str>> {
        if range.len() < LIST_BOUNDARY_WIDTH
            || !self.tokens[range.start].symbol(LIST_OPEN)
            || !self.tokens[range.end - NEXT_OFFSET].symbol(LIST_CLOSE)
        {
            return None;
        }
        let interior = range.start + NEXT_OFFSET..range.end - NEXT_OFFSET;
        split(&self.tokens, &interior, SEPARATOR)
            .into_iter()
            .map(|item| self.string(&item))
            .collect()
    }

    pub fn string_count(&self, expected: &str) -> usize {
        self.tokens
            .iter()
            .filter(|token| token.string() == Some(expected))
            .count()
    }

    pub fn receiver<'a>(&'a self, call: &Call) -> Option<&'a str> {
        let index = call
            .arguments
            .start
            .checked_sub(QUALIFIED_RECEIVER_OFFSET)?;
        self.tokens
            .get(index + NEXT_OFFSET)?
            .symbol(MEMBER_SEPARATOR)
            .then(|| self.tokens.get(index)?.identifier())
            .flatten()
    }

    pub fn has_alias_of(&self, expected: &str) -> bool {
        self.tokens
            .windows(KEYWORD_VALUE_OFFSET + NEXT_OFFSET)
            .any(|tokens| {
                tokens[0].identifier().is_some()
                    && tokens[1].symbol(ASSIGNMENT)
                    && tokens[2].identifier() == Some(expected)
            })
    }

    fn unique_keyword(&self, call: &Call, name: &str) -> Option<Range<usize>> {
        let matches = self
            .keywords(call)?
            .into_iter()
            .filter(|keyword| keyword.name == name)
            .collect::<Vec<_>>();
        (matches.len() == SINGLE_MATCH).then(|| matches[0].value.clone())
    }
}

fn split(tokens: &[Token], range: &Range<usize>, separator: char) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    let mut start = range.start;
    let mut depth = EMPTY_DEPTH;
    for index in range.clone() {
        depth += usize::from(opens(&tokens[index]));
        depth = depth.saturating_sub(usize::from(closes(&tokens[index])));
        if depth == EMPTY_DEPTH && tokens[index].symbol(separator) {
            if start < index {
                result.push(start..index);
            }
            start = index + NEXT_OFFSET;
        }
    }
    if start < range.end {
        result.push(start..range.end);
    }
    result
}

fn opens(token: &Token) -> bool {
    token.symbol(CALL_OPEN) || token.symbol(LIST_OPEN) || token.symbol(DICTIONARY_OPEN)
}

fn closes(token: &Token) -> bool {
    token.symbol(CALL_CLOSE) || token.symbol(LIST_CLOSE) || token.symbol(DICTIONARY_CLOSE)
}

fn closing(tokens: &[Token], open: usize, left: char, right: char) -> Option<usize> {
    let mut depth = EMPTY_DEPTH;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        depth += usize::from(token.symbol(left));
        if token.symbol(right) {
            depth = depth.checked_sub(NEXT_OFFSET)?;
            if depth == EMPTY_DEPTH {
                return Some(index);
            }
        }
    }
    None
}
