use std::path::Path;
use yolo_architecture_cargo_policy::cargo_shell_prefixes::{Prefix, command_prefix};
use yolo_architecture_cargo_policy::cargo_shell_substitutions::substitutions;
use yolo_architecture_cargo_policy::cargo_shell_words::segments;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::constants::CARGO_COMMAND_PREFIXES;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_source_policy::source_policy::is_rust_test_source;
use yolo_architecture_syntax::rust_source::{RustSource, string};

const ASSIGNMENT: char = '=';
const OPTION_PREFIX: char = '-';
const SHELL_COMMAND_FLAG: char = 'c';
const MAX_NESTED_SHELLS: usize = 8;
const SHELLS: &[&str] = &["bash", "dash", "ksh", "sh", "zsh"];
const ALIAS_COMMAND: &str = "alias";

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    if is_rust_test_source(&file.relative) {
        return;
    }
    for (index, token) in source.tokens.iter().enumerate() {
        let Some(value) = string(token) else {
            continue;
        };
        if !unit_test_function(source, index) && contains_signal_command(value, 0) {
            violations.push(Violation::line(
                &file.relative,
                token.line,
                contract::SIGNAL_EXECUTABLE_MESSAGE,
            ));
        }
    }
}

fn contains_signal_command(source: &str, depth: usize) -> bool {
    if depth >= MAX_NESTED_SHELLS {
        return true;
    }
    segments(source)
        .iter()
        .any(|segment| segment_is_signal(&segment.words, depth))
        || substitutions(source)
            .iter()
            .any(|substitution| contains_signal_command(&substitution.source, depth + 1))
}

fn segment_is_signal(words: &[String], depth: usize) -> bool {
    if words.is_empty() {
        return false;
    }
    if basename(&words[0]) == ALIAS_COMMAND {
        return words[1..].iter().any(|word| assignment_is_signal(word));
    }
    if words.iter().any(|word| assignment_is_signal(word)) {
        return true;
    }
    let mut cursor = words
        .iter()
        .take_while(|word| assignment(word).is_some())
        .count();
    loop {
        let Some(command) = words.get(cursor).map(|word| basename(word)) else {
            return false;
        };
        if contract::SIGNAL_EXECUTABLES.contains(&command) {
            return true;
        }
        if SHELLS.contains(&command) {
            return nested_shell_is_signal(words, cursor, depth);
        }
        if CARGO_COMMAND_PREFIXES.contains(&command) {
            match command_prefix(command, words, cursor + 1) {
                Prefix::Continue(next) => cursor = skip_assignments(words, next),
                Prefix::Nested(source) => return contains_signal_command(&source, depth + 1),
                Prefix::NonExecuting => return false,
            }
            continue;
        }
        return false;
    }
}

fn nested_shell_is_signal(words: &[String], cursor: usize, depth: usize) -> bool {
    words[cursor + 1..].windows(2).any(|pair| {
        pair[0].starts_with(OPTION_PREFIX)
            && pair[0]
                .chars()
                .skip(1)
                .any(|flag| flag == SHELL_COMMAND_FLAG)
            && contains_signal_command(&pair[1], depth + 1)
    })
}

fn skip_assignments(words: &[String], mut cursor: usize) -> usize {
    while words
        .get(cursor)
        .is_some_and(|word| assignment(word).is_some())
    {
        cursor += 1;
    }
    cursor
}

fn assignment_is_signal(word: &str) -> bool {
    assignment(word).is_some_and(|value| contract::SIGNAL_EXECUTABLES.contains(&basename(value)))
}

fn assignment(word: &str) -> Option<&str> {
    let (name, value) = word.split_once(ASSIGNMENT)?;
    (!name.is_empty() && !value.is_empty()).then_some(value)
}

fn basename(value: &str) -> &str {
    Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value)
}

fn unit_test_function(source: &RustSource, index: usize) -> bool {
    source
        .function_at(index)
        .is_some_and(|function| source.is_unit_test_guarded(function))
}

#[cfg(test)]
#[path = "../unit_tests/shell_command_boundary_tests.rs"]
mod boundary_tests;
