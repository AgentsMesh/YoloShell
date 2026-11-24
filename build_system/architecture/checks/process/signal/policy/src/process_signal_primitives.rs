use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_exec_contract as exec_contract;
use yolo_architecture_process_signal_foundation::process_signal_sites as sites;
use yolo_architecture_process_signal_foundation::process_signal_token_sequence as sequence;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const PATH_SEPARATOR: &str = "::";
const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const ITEM_END: &str = ";";
const FUNCTION_KEYWORD: &str = "fn";
const EXTERN_KEYWORD: &str = "extern";
const ARGUMENT_SEPARATOR: &str = ",";
const NO_SIGNAL_NUMBER: u64 = 0;

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    inspect_libc_calls(file, source, violations);
}

fn inspect_libc_calls(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    for (index, token) in source.tokens.iter().enumerate() {
        let Some(name) = identifier(token) else {
            continue;
        };
        let Some(message) = primitive_message(name) else {
            continue;
        };
        if foreign_definition(&source.tokens, index) {
            violation(file, token, message, violations);
            continue;
        }
        if !libc_reference(&source.tokens, index) {
            if sequence::macro_argument(&source.tokens, index) {
                violation(file, token, message, violations);
            }
            continue;
        }
        let function = source.function_at(index);
        let allowed = sites::RAW_LIBC_SITES
            .iter()
            .any(|entry| entry.name == name && entry.site.matches(&file.relative, function))
            && direct_call(&source.tokens, index);
        let zero_required = sites::ZERO_SIGNAL_SITES
            .iter()
            .any(|site| site.matches(&file.relative, function));
        if !allowed || zero_required && !zero_signal_argument(&source.tokens, index) {
            violation(file, token, message, violations);
        }
    }
}

fn primitive_message(name: &str) -> Option<&'static str> {
    if name == exec_contract::EXECVEAT_FUNCTION {
        Some(exec_contract::RAW_EXEC_MESSAGE)
    } else if contract::RAW_LIBC_SIGNAL_NAMES.contains(&name) {
        Some(contract::RAW_SIGNAL_MESSAGE)
    } else {
        None
    }
}

fn libc_reference(tokens: &[Token], index: usize) -> bool {
    index >= 2
        && tokens[index - 2].identifier(contract::LIBC_NAMESPACE)
        && tokens[index - 1].symbol(PATH_SEPARATOR)
}

fn direct_call(tokens: &[Token], index: usize) -> bool {
    tokens
        .get(index + 1)
        .is_some_and(|token| token.symbol(CALL_OPEN))
}

fn foreign_definition(tokens: &[Token], index: usize) -> bool {
    if index == 0 || !tokens[index - 1].identifier(FUNCTION_KEYWORD) {
        return false;
    }
    item_prefix_has_extern(tokens, index)
        || enclosing_block(tokens, index).is_some_and(|open| item_prefix_has_extern(tokens, open))
}

fn item_prefix_has_extern(tokens: &[Token], end: usize) -> bool {
    let start = tokens[..end]
        .iter()
        .rposition(|token| {
            token.symbol(ITEM_END) || token.symbol(BLOCK_OPEN) || token.symbol(BLOCK_CLOSE)
        })
        .map_or(0, |position| position + 1);
    tokens[start..end]
        .iter()
        .any(|token| token.identifier(EXTERN_KEYWORD))
}

fn enclosing_block(tokens: &[Token], index: usize) -> Option<usize> {
    let mut depth = 0_usize;
    for position in (0..index).rev() {
        if tokens[position].symbol(BLOCK_CLOSE) {
            depth += 1;
        } else if tokens[position].symbol(BLOCK_OPEN) {
            if depth == 0 {
                return Some(position);
            }
            depth -= 1;
        }
    }
    None
}

fn zero_signal_argument(tokens: &[Token], call: usize) -> bool {
    let open = call + 1;
    let Some(close) = closing(tokens, open, CALL_OPEN, CALL_CLOSE) else {
        return false;
    };
    let Some(comma) = tokens[open + 1..close]
        .iter()
        .rposition(|token| token.symbol(ARGUMENT_SEPARATOR))
        .map(|offset| open + 1 + offset)
    else {
        return false;
    };
    let argument = comma + 1;
    close == argument + 1 && tokens[argument].number() == Some(NO_SIGNAL_NUMBER)
}

fn violation(file: &SourceFile, token: &Token, message: &str, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(&file.relative, token.line, message));
}
