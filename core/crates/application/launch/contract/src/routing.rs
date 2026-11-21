use crate::argument_constants::END_OF_OPTIONS;
use std::ffi::OsString;

/// Command tokens and root options used to recognize an auxiliary invocation.
pub struct Routing<'a> {
    pub commands: &'a [&'a str],
    pub value_options: &'a [&'a str],
    pub terminal_options: &'a [&'a str],
}

/// Returns the position of a recognized command when root options precede it.
pub fn command_index(arguments: &[OsString], routing: &Routing<'_>) -> Option<usize> {
    let first = arguments.get(1).and_then(|argument| argument.to_str())?;
    if routing.commands.contains(&first) || !first.starts_with('-') {
        return None;
    }
    let mut index = 1;
    while index < arguments.len() {
        let argument = arguments[index].to_str()?;
        if routing.terminal_options.contains(&argument) || argument == END_OF_OPTIONS {
            return None;
        }
        if routing.value_options.contains(&argument) {
            index += 2;
            continue;
        }
        if argument.starts_with('-') {
            index += 1;
            continue;
        }
        return routing.commands.contains(&argument).then_some(index);
    }
    None
}
