use crate::cargo_shell_words::{segments, shell_command_option};
use crate::policy_constants::*;
use std::collections::BTreeMap;

pub fn execution_lines(source: &str) -> Vec<usize> {
    let mut state = State::default();
    let mut result: Vec<_> = segments(source)
        .into_iter()
        .filter_map(|segment| state.executes(&segment.words).then_some(segment.line))
        .collect();
    result.extend(
        crate::cargo_shell_substitutions::substitutions(source)
            .into_iter()
            .filter_map(|item| {
                execution_lines_with_state(&item.source, &mut state).then_some(item.line)
            }),
    );
    result
}

#[derive(Clone, Default)]
struct State {
    variables: BTreeMap<String, String>,
}

impl State {
    fn executes(&mut self, words: &[String]) -> bool {
        let mut cursor = crate::cargo_shell_prefixes::skip_controls(words, 0);
        if words
            .first()
            .is_some_and(|word| CARGO_DECLARATION_COMMANDS.contains(&word.as_str()))
        {
            cursor += 1;
            while let Some(word) = words.get(cursor) {
                self.record_assignment(word);
                cursor += 1;
            }
            return false;
        }
        loop {
            while words
                .get(cursor)
                .is_some_and(|word| self.record_assignment(word))
            {
                cursor += 1;
            }
            let Some(prefix) = words.get(cursor).map(|word| basename(word)) else {
                return false;
            };
            if !CARGO_COMMAND_PREFIXES.contains(&prefix) {
                break;
            }
            cursor += 1;
            match crate::cargo_shell_prefixes::command_prefix(prefix, words, cursor) {
                crate::cargo_shell_prefixes::Prefix::Continue(next) => cursor = next,
                crate::cargo_shell_prefixes::Prefix::Nested(source) => {
                    let mut nested = self.clone();
                    return execution_lines_with_state(&source, &mut nested);
                }
                crate::cargo_shell_prefixes::Prefix::NonExecuting => return false,
            }
        }
        let Some(command_word) = words.get(cursor) else {
            return false;
        };
        let command = self.expand(command_word);
        if expanded_command_basename(&command).eq_ignore_ascii_case(CARGO_EXECUTABLE) {
            return true;
        }
        if !SHELL_WRAPPERS.contains(&basename(&command)) {
            return false;
        }
        words[cursor + 1..]
            .windows(2)
            .find(|pair| shell_command_option(&pair[0]))
            .is_some_and(|pair| {
                let mut nested = self.clone();
                execution_lines_with_state(&pair[1], &mut nested)
            })
    }

    fn record_assignment(&mut self, word: &str) -> bool {
        let Some((name, value)) = word.split_once('=') else {
            return false;
        };
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return false;
        }
        let expanded = self.expand(value);
        self.variables.insert(name.to_owned(), expanded);
        true
    }

    fn expand(&self, word: &str) -> String {
        let bytes = word.as_bytes();
        let mut result = String::new();
        let mut cursor = 0;
        while cursor < bytes.len() {
            if bytes[cursor] == b'\\' && bytes.get(cursor + 1) == Some(&b'$') {
                result.push('$');
                cursor += 2;
            } else if bytes[cursor] != b'$' {
                result.push(char::from(bytes[cursor]));
                cursor += 1;
            } else {
                let (value, end) = self.expansion(word, cursor);
                result.push_str(&value);
                cursor = end;
            }
        }
        result
    }

    fn expansion(&self, word: &str, dollar: usize) -> (String, usize) {
        let tail = &word[dollar + 1..];
        if let Some(body) = tail.strip_prefix('{')
            && let Some(close) = body.find('}')
        {
            let expression = &body[..close];
            let (name, fallback) = default_expression(expression);
            let value = self
                .variables
                .get(name)
                .filter(|value| !value.is_empty())
                .cloned()
                .or_else(|| fallback.map(|value| self.expand(value)))
                .unwrap_or_else(|| unresolved(name));
            return (value, dollar + close + 3);
        }
        let length = tail
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            .count();
        if length == 0 {
            return ("$".to_owned(), dollar + 1);
        }
        let name = &tail[..length];
        (
            self.variables
                .get(name)
                .cloned()
                .unwrap_or_else(|| unresolved(name)),
            dollar + length + 1,
        )
    }
}

fn execution_lines_with_state(source: &str, state: &mut State) -> bool {
    segments(source)
        .into_iter()
        .any(|segment| state.executes(&segment.words))
        || crate::cargo_shell_substitutions::substitutions(source)
            .into_iter()
            .any(|item| execution_lines_with_state(&item.source, state))
}

fn expanded_command_basename(command: &str) -> &str {
    command
        .split_ascii_whitespace()
        .next()
        .map(basename)
        .unwrap_or_default()
}

fn default_expression(expression: &str) -> (&str, Option<&str>) {
    [":-", ":=", "-", "="]
        .iter()
        .find_map(|operator| {
            expression
                .split_once(operator)
                .map(|(name, value)| (name, Some(value)))
        })
        .unwrap_or((expression, None))
}

fn unresolved(name: &str) -> String {
    if name.eq_ignore_ascii_case(CARGO_EXECUTABLE) {
        CARGO_EXECUTABLE.to_owned()
    } else {
        format!("${name}")
    }
}

fn basename(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

#[cfg(test)]
#[path = "../unit_tests/cargo_shell_state_boundaries.rs"]
mod tests;
