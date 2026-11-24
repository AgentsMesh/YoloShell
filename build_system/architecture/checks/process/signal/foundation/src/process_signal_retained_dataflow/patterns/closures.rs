use super::{Flow, statement_end, statement_start};
use crate::process_signal_retained_dataflow::Depth;
use std::ops::Range;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const ASSIGNMENT: &str = "=";
const ITEM_END: &str = ";";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const GROUP_OPEN: &str = "(";
const GROUP_CLOSE: &str = ")";
const ARRAY_OPEN: &str = "[";
const ARRAY_CLOSE: &str = "]";
const ARGUMENT_SEPARATOR: &str = ",";
const CLOSURE_BAR: &str = "|";
const MOVE_KEYWORD: &str = "move";
const ASYNC_KEYWORD: &str = "async";
const RETURN_KEYWORD: &str = "return";

pub fn flows(tokens: &[Token], scope: &Range<usize>) -> Vec<Flow> {
    let mut result = Vec::new();
    let mut index = scope.start;
    while index < scope.end {
        if closure_open(tokens, index, scope.start)
            && let Some(close) = closure_end(tokens, index + 1, scope.end)
        {
            result.extend(closure_sources(tokens, scope, index, close));
            index = close;
        }
        index += 1;
    }
    result
}

fn closure_sources(tokens: &[Token], scope: &Range<usize>, open: usize, close: usize) -> Vec<Flow> {
    let patterns = split_ranges(tokens, open + 1..close);
    let statement_start = statement_start(tokens, scope.start, open);
    let statement_end = statement_end(tokens, close + 1, scope.end);
    let body_end = closure_body_end(tokens, close + 1, statement_end);
    let prefix_start = (statement_start..open)
        .rev()
        .find(|index| tokens[*index].symbol(ASSIGNMENT))
        .map_or(statement_start, |assignment| assignment + 1);
    let mut result = broadcast(&patterns, prefix_start..open);
    if let Some(arguments) = immediate_arguments(tokens, body_end, statement_end) {
        result.extend(mapped_sources(&patterns, &arguments));
    } else if body_end < statement_end {
        result.extend(broadcast(&patterns, body_end..statement_end));
    }
    if let Some(binding) = closure_binding(tokens, statement_start, open) {
        result.extend(binding_calls(
            tokens,
            scope,
            statement_end,
            binding,
            &patterns,
        ));
    }
    result
}

fn closure_body_end(tokens: &[Token], start: usize, upper: usize) -> usize {
    let mut depth = Depth::default();
    for index in start..upper {
        if depth.is_top() && tokens[index].symbol(BLOCK_OPEN) {
            return closing(tokens, index, BLOCK_OPEN, BLOCK_CLOSE)
                .map_or(upper, |close| close + 1);
        }
        if depth.is_top()
            && (tokens[index].symbol(GROUP_CLOSE)
                || tokens[index].symbol(ARRAY_CLOSE)
                || tokens[index].symbol(BLOCK_CLOSE)
                || tokens[index].symbol(ARGUMENT_SEPARATOR)
                || tokens[index].symbol(ITEM_END))
        {
            return index;
        }
        depth.observe(&tokens[index]);
    }
    upper
}

fn closure_binding<'a>(tokens: &'a [Token], start: usize, open: usize) -> Option<&'a str> {
    let assignment = (start..open)
        .rev()
        .find(|index| tokens[*index].symbol(ASSIGNMENT))?;
    tokens[start..assignment].iter().rev().find_map(identifier)
}

fn binding_calls(
    tokens: &[Token],
    scope: &Range<usize>,
    start: usize,
    binding: &str,
    patterns: &[Range<usize>],
) -> Vec<Flow> {
    let mut result = Vec::new();
    for call in start..scope.end {
        if !tokens[call].identifier(binding)
            || !tokens
                .get(call + 1)
                .is_some_and(|token| token.symbol(GROUP_OPEN))
        {
            continue;
        }
        let open = call + 1;
        if let Some(close) = closing(tokens, open, GROUP_OPEN, GROUP_CLOSE) {
            let arguments = split_ranges(tokens, open + 1..close);
            result.extend(mapped_sources(patterns, &arguments));
        }
    }
    result
}

fn immediate_arguments(tokens: &[Token], start: usize, end: usize) -> Option<Vec<Range<usize>>> {
    let open = (start..end).find(|index| tokens[*index].symbol(GROUP_OPEN))?;
    if tokens[start..open].iter().any(|token| {
        !token.symbol(GROUP_CLOSE) && !token.symbol(ARRAY_CLOSE) && !token.symbol(BLOCK_CLOSE)
    }) {
        return None;
    }
    let close = closing(tokens, open, GROUP_OPEN, GROUP_CLOSE)?;
    Some(split_ranges(tokens, open + 1..close))
}

fn split_ranges(tokens: &[Token], range: Range<usize>) -> Vec<Range<usize>> {
    let mut start = range.start;
    let mut depth = Depth::default();
    let mut result = Vec::new();
    for index in range.clone() {
        if depth.is_top() && tokens[index].symbol(ARGUMENT_SEPARATOR) {
            result.push(start..index);
            start = index + 1;
        }
        depth.observe(&tokens[index]);
    }
    if start < range.end {
        result.push(start..range.end);
    }
    result
}

fn mapped_sources(patterns: &[Range<usize>], sources: &[Range<usize>]) -> Vec<Flow> {
    if patterns.len() == sources.len() {
        return patterns
            .iter()
            .cloned()
            .zip(sources.iter().cloned())
            .map(|(pattern, source)| Flow { pattern, source })
            .collect();
    }
    let Some(source) = sources
        .first()
        .zip(sources.last())
        .map(|(first, last)| first.start..last.end)
    else {
        return Vec::new();
    };
    broadcast(patterns, source)
}

fn broadcast(patterns: &[Range<usize>], source: Range<usize>) -> Vec<Flow> {
    patterns
        .iter()
        .cloned()
        .map(|pattern| Flow {
            pattern,
            source: source.clone(),
        })
        .collect()
}

fn closure_open(tokens: &[Token], index: usize, lower: usize) -> bool {
    if !tokens[index].symbol(CLOSURE_BAR) || index == lower {
        return false;
    }
    let previous = &tokens[index - 1];
    previous.symbol(GROUP_OPEN)
        || previous.symbol(ARRAY_OPEN)
        || previous.symbol(BLOCK_OPEN)
        || previous.symbol(ARGUMENT_SEPARATOR)
        || previous.symbol(ASSIGNMENT)
        || previous.symbol(ITEM_END)
        || previous.identifier(MOVE_KEYWORD)
        || previous.identifier(ASYNC_KEYWORD)
        || previous.identifier(RETURN_KEYWORD)
}

fn closure_end(tokens: &[Token], start: usize, upper: usize) -> Option<usize> {
    let mut depth = Depth::default();
    for index in start..upper {
        if depth.is_top() && tokens[index].symbol(CLOSURE_BAR) {
            return Some(index);
        }
        depth.observe(&tokens[index]);
    }
    None
}
