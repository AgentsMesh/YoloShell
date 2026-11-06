use crate::pre_exec_rlimit_contract as contract;
use std::ops::Range;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

pub struct NamedCallback {
    pub call: usize,
    pub name: String,
}

pub fn callback_bodies(tokens: &[Token]) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if !token.identifier(contract::PRE_EXEC_METHOD) || !qualified_call(tokens, index) {
            continue;
        }
        let open = index + contract::NEXT_TOKEN_OFFSET;
        let Some(close) = closing(tokens, open, contract::CALL_OPEN, contract::CALL_CLOSE) else {
            continue;
        };
        result.extend(inline_closure_bodies(
            tokens,
            open + contract::NEXT_TOKEN_OFFSET..close,
        ));
    }
    result
}

pub fn named_callbacks(tokens: &[Token]) -> Vec<NamedCallback> {
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.identifier(contract::PRE_EXEC_METHOD) && qualified_call(tokens, *index)
        })
        .filter_map(|(call, _)| {
            let open = call + contract::NEXT_TOKEN_OFFSET;
            let close = closing(tokens, open, contract::CALL_OPEN, contract::CALL_CLOSE)?;
            let arguments = open + contract::NEXT_TOKEN_OFFSET..close;
            callback_name(tokens, arguments).map(|name| NamedCallback {
                call,
                name: name.to_owned(),
            })
        })
        .collect()
}

pub fn setrlimit_call(tokens: &[Token], index: usize) -> bool {
    if !tokens[index].identifier(contract::SETRLIMIT_FUNCTION) {
        return false;
    }
    let mut call = index + contract::NEXT_TOKEN_OFFSET;
    while tokens
        .get(call)
        .is_some_and(|token| token.symbol(contract::CALL_CLOSE))
    {
        call += contract::NEXT_TOKEN_OFFSET;
    }
    tokens
        .get(call)
        .is_some_and(|token| token.symbol(contract::CALL_OPEN))
}

fn qualified_call(tokens: &[Token], index: usize) -> bool {
    tokens
        .get(index.wrapping_sub(contract::NEXT_TOKEN_OFFSET))
        .is_some_and(|token| {
            token.symbol(contract::MEMBER_ACCESS) || token.symbol(contract::PATH_SEPARATOR)
        })
        && tokens
            .get(index + contract::NEXT_TOKEN_OFFSET)
            .is_some_and(|token| token.symbol(contract::CALL_OPEN))
}

fn callback_name(tokens: &[Token], arguments: Range<usize>) -> Option<&str> {
    if tokens[arguments.clone()]
        .iter()
        .any(|token| token.symbol(contract::CLOSURE_BAR))
    {
        return None;
    }
    let start = last_argument_start(tokens, arguments.clone());
    tokens[start..arguments.end]
        .iter()
        .rev()
        .find_map(identifier)
}

fn last_argument_start(tokens: &[Token], arguments: Range<usize>) -> usize {
    let mut depth = Depth::default();
    let mut start = arguments.start;
    for index in arguments {
        if depth.is_top() && tokens[index].symbol(contract::ARGUMENT_SEPARATOR) {
            start = index + contract::NEXT_TOKEN_OFFSET;
        }
        depth.observe(&tokens[index]);
    }
    start
}

fn inline_closure_bodies(tokens: &[Token], arguments: Range<usize>) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    let mut index = arguments.start;
    while index < arguments.end {
        if closure_open(tokens, index, arguments.start)
            && let Some(close) =
                closure_close(tokens, index + contract::NEXT_TOKEN_OFFSET, arguments.end)
        {
            result.push(close + contract::NEXT_TOKEN_OFFSET..arguments.end);
            index = close;
        }
        index += contract::NEXT_TOKEN_OFFSET;
    }
    result
}

pub(crate) fn closure_open(tokens: &[Token], index: usize, lower: usize) -> bool {
    if index == lower || !tokens[index].symbol(contract::CLOSURE_BAR) {
        return index == lower && tokens[index].symbol(contract::CLOSURE_BAR);
    }
    let previous = &tokens[index - contract::NEXT_TOKEN_OFFSET];
    previous.symbol(contract::CALL_OPEN)
        || previous.symbol(contract::ARRAY_OPEN)
        || previous.symbol(contract::BLOCK_OPEN)
        || previous.symbol(contract::ARGUMENT_SEPARATOR)
        || previous.symbol(contract::ASSIGNMENT)
        || previous.symbol(contract::ITEM_END)
        || previous.identifier(contract::MOVE_KEYWORD)
        || previous.identifier(contract::ASYNC_KEYWORD)
        || previous.identifier(contract::RETURN_KEYWORD)
}

pub(crate) fn closure_close(tokens: &[Token], start: usize, upper: usize) -> Option<usize> {
    let mut depth = Depth::default();
    for index in start..upper {
        if depth.is_top() && tokens[index].symbol(contract::CLOSURE_BAR) {
            return Some(index);
        }
        depth.observe(&tokens[index]);
    }
    None
}

pub(crate) fn statement_end(tokens: &[Token], start: usize, upper: usize) -> usize {
    let mut depth = Depth::default();
    for index in start..upper {
        if depth.is_top() && tokens[index].symbol(contract::ITEM_END) {
            return index;
        }
        depth.observe(&tokens[index]);
    }
    upper
}

#[derive(Default)]
struct Depth {
    groups: usize,
    arrays: usize,
    blocks: usize,
}

impl Depth {
    fn is_top(&self) -> bool {
        self.groups == usize::default()
            && self.arrays == usize::default()
            && self.blocks == usize::default()
    }

    fn observe(&mut self, token: &Token) {
        if token.symbol(contract::CALL_OPEN) {
            self.groups += contract::NEXT_TOKEN_OFFSET;
        } else if token.symbol(contract::CALL_CLOSE) {
            self.groups = self.groups.saturating_sub(contract::NEXT_TOKEN_OFFSET);
        } else if token.symbol(contract::ARRAY_OPEN) {
            self.arrays += contract::NEXT_TOKEN_OFFSET;
        } else if token.symbol(contract::ARRAY_CLOSE) {
            self.arrays = self.arrays.saturating_sub(contract::NEXT_TOKEN_OFFSET);
        } else if token.symbol(contract::BLOCK_OPEN) {
            self.blocks += contract::NEXT_TOKEN_OFFSET;
        } else if token.symbol(contract::BLOCK_CLOSE) {
            self.blocks = self.blocks.saturating_sub(contract::NEXT_TOKEN_OFFSET);
        }
    }
}

#[cfg(all(test, feature = "yolo-runtime-process-tests"))]
#[path = "../unit_tests/pre_exec_syntax_boundary_tests.rs"]
mod boundary_tests;
