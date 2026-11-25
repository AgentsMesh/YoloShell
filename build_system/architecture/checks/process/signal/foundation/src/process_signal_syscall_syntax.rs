use crate::process_signal_token_sequence as sequence;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const PATH_SEPARATOR: &str = "::";
const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const ARGUMENT_SEPARATOR: &str = ",";

pub fn libc_call(tokens: &[Token], index: usize) -> bool {
    libc_reference(tokens, index)
        && tokens
            .get(index + 1)
            .is_some_and(|token| token.symbol(CALL_OPEN))
}

pub fn libc_reference(tokens: &[Token], index: usize) -> bool {
    index >= 2
        && tokens[index - 2].identifier(contract::LIBC_NAMESPACE)
        && tokens[index - 1].symbol(PATH_SEPARATOR)
}

pub fn macro_argument(tokens: &[Token], index: usize) -> bool {
    sequence::macro_argument(tokens, index)
}

pub fn call_name(tokens: &[Token], call: usize) -> Option<&str> {
    arguments(tokens, call)?
        .first()
        .and_then(|argument| syscall_name(argument))
}

pub fn arguments(tokens: &[Token], call: usize) -> Option<Vec<&[Token]>> {
    let open = call + 1;
    let close = closing(tokens, open, CALL_OPEN, CALL_CLOSE)?;
    Some(split_arguments(tokens, open + 1, close))
}

pub fn containing_call(tokens: &[Token], index: usize) -> Option<usize> {
    (0..index).rev().find(|position| {
        tokens[*position].identifier(contract::LIBC_SYSCALL)
            && libc_call(tokens, *position)
            && closing(tokens, *position + 1, CALL_OPEN, CALL_CLOSE)
                .is_some_and(|close| index < close)
    })
}

fn split_arguments(tokens: &[Token], start: usize, end: usize) -> Vec<&[Token]> {
    let mut result = Vec::new();
    let mut argument = start;
    let mut depth = 0_i32;
    for index in start..end {
        if tokens[index].symbol(CALL_OPEN) {
            depth += 1;
        } else if tokens[index].symbol(CALL_CLOSE) {
            depth -= 1;
        } else if depth == 0 && tokens[index].symbol(ARGUMENT_SEPARATOR) {
            result.push(&tokens[argument..index]);
            argument = index + 1;
        }
    }
    if argument < end {
        result.push(&tokens[argument..end]);
    }
    result
}

fn syscall_name(tokens: &[Token]) -> Option<&str> {
    match tokens {
        [namespace, separator, name]
            if namespace.identifier(contract::LIBC_NAMESPACE)
                && separator.symbol(PATH_SEPARATOR) =>
        {
            identifier(name)
        }
        _ => None,
    }
}
