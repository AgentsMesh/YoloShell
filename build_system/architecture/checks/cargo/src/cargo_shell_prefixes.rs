use crate::policy_constants::*;

const SHORT_OPTION_PREFIX: char = '-';
const ENV_SPLIT_SHORT_OPTION: char = 'S';
const ENV_VALUE_SHORT_OPTIONS: &[char] = &['C', 'P', 'a', 'u'];
const ENV_FLAG_SHORT_OPTIONS: &[char] = &['0', 'i', 'v'];

pub enum Prefix {
    Continue(usize),
    Nested(String),
    NonExecuting,
}

pub fn skip_controls(words: &[String], mut cursor: usize) -> usize {
    loop {
        let Some(word) = words.get(cursor).map(String::as_str) else {
            return cursor;
        };
        if CONTROL_WORDS.contains(&word) {
            cursor += 1;
        } else if word == TIME_COMMAND {
            cursor = skip_options(words, cursor + 1, TIME_VALUE_OPTIONS);
        } else {
            return cursor;
        }
    }
}

pub fn command_prefix(prefix: &str, words: &[String], cursor: usize) -> Prefix {
    match prefix {
        POLICY_COMMAND => command_options(words, cursor),
        POLICY_ENV => env_options(words, cursor),
        POLICY_EVAL => Prefix::Nested(words[cursor..].join(" ")),
        POLICY_EXEC => Prefix::Continue(skip_options(words, cursor, EXEC_VALUE_OPTIONS)),
        POLICY_NICE => launcher_options(words, cursor, NICE_VALUE_OPTIONS, NON_EXECUTING_OPTIONS),
        POLICY_NOHUP => launcher_options(words, cursor, &[], NON_EXECUTING_OPTIONS),
        POLICY_SUDO => launcher_options(words, cursor, SUDO_VALUE_OPTIONS, SUDO_QUERY_OPTIONS),
        POLICY_XCRUN => launcher_options(words, cursor, XCRUN_VALUE_OPTIONS, XCRUN_QUERY_OPTIONS),
        _ => Prefix::Continue(cursor),
    }
}

fn command_options(words: &[String], mut cursor: usize) -> Prefix {
    while let Some(option) = words.get(cursor).map(String::as_str) {
        if COMMAND_QUERY_OPTIONS.contains(&option) {
            return Prefix::NonExecuting;
        }
        if option == OPTION_END {
            return Prefix::Continue(cursor + 1);
        }
        if !option.starts_with('-') {
            break;
        }
        cursor += 1;
    }
    Prefix::Continue(cursor)
}

fn env_options(words: &[String], mut cursor: usize) -> Prefix {
    while let Some(option) = words.get(cursor).map(String::as_str) {
        if NON_EXECUTING_OPTIONS.contains(&option) {
            return Prefix::NonExecuting;
        }
        if option == OPTION_END {
            return Prefix::Continue(cursor + 1);
        }
        if ENV_SPLIT_OPTIONS.contains(&option) {
            return words
                .get(cursor + 1)
                .map_or(Prefix::NonExecuting, |value| Prefix::Nested(value.clone()));
        }
        if let Some(value) = option.strip_prefix(ENV_SPLIT_PREFIX) {
            return Prefix::Nested(value.to_owned());
        }
        if let Some(result) = env_short_options(words, cursor, option) {
            match result {
                Prefix::Continue(next) => cursor = next,
                other => return other,
            }
            continue;
        }
        cursor += 1;
        if ENV_VALUE_OPTIONS.contains(&option) {
            cursor += 1;
        } else if !option.starts_with('-') {
            return Prefix::Continue(cursor - 1);
        }
    }
    Prefix::Continue(cursor)
}

fn env_short_options(words: &[String], cursor: usize, option: &str) -> Option<Prefix> {
    let cluster = option
        .strip_prefix(SHORT_OPTION_PREFIX)
        .filter(|value| !value.starts_with(SHORT_OPTION_PREFIX) && !value.is_empty())?;
    for (offset, name) in cluster.char_indices() {
        if ENV_FLAG_SHORT_OPTIONS.contains(&name) {
            continue;
        }
        if name != ENV_SPLIT_SHORT_OPTION && !ENV_VALUE_SHORT_OPTIONS.contains(&name) {
            return Some(Prefix::NonExecuting);
        }
        let attached = &cluster[offset + name.len_utf8()..];
        let (value, next) = if attached.is_empty() {
            (words.get(cursor + 1)?.as_str(), cursor + 2)
        } else {
            (attached, cursor + 1)
        };
        return Some(if name == ENV_SPLIT_SHORT_OPTION {
            Prefix::Nested(value.to_owned())
        } else {
            Prefix::Continue(next)
        });
    }
    Some(Prefix::Continue(cursor + 1))
}

fn launcher_options(
    words: &[String],
    mut cursor: usize,
    value_options: &[&str],
    query_options: &[&str],
) -> Prefix {
    while let Some(option) = words.get(cursor).map(String::as_str) {
        if query_options.contains(&option) {
            return Prefix::NonExecuting;
        }
        if option == OPTION_END {
            return Prefix::Continue(cursor + 1);
        }
        if !option.starts_with('-') {
            return Prefix::Continue(cursor);
        }
        cursor += 1;
        if value_options.contains(&option) {
            cursor += 1;
        }
    }
    Prefix::Continue(cursor)
}

fn skip_options(words: &[String], mut cursor: usize, value_options: &[&str]) -> usize {
    while let Some(option) = words.get(cursor).map(String::as_str) {
        if option == OPTION_END {
            return cursor + 1;
        }
        if !option.starts_with('-') {
            return cursor;
        }
        cursor += 1;
        if value_options.contains(&option) {
            cursor += 1;
        }
    }
    cursor
}
