use super::{constants, metadata};
use crate::agent::ObservedProcess;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SessionIdentity {
    Fresh,
    AssociatedOnly,
    Invalid,
    Resume(String),
}

impl SessionIdentity {
    pub(super) fn is_valid(&self) -> bool {
        !matches!(self, Self::Invalid)
    }

    pub(super) fn into_session_id(self) -> Option<String> {
        match self {
            Self::Resume(session_id) => Some(session_id),
            Self::Fresh | Self::AssociatedOnly | Self::Invalid => None,
        }
    }
}

pub(super) fn session_identity(process: &ObservedProcess) -> SessionIdentity {
    let Some(arguments) = command_arguments(process) else {
        return SessionIdentity::Invalid;
    };
    if arguments.iter().any(|argument| remote_option(argument)) {
        return SessionIdentity::Invalid;
    }
    let Some(resume_index) = resume_argument_index(arguments) else {
        return SessionIdentity::Fresh;
    };
    resume_identity(&arguments[resume_index + constants::NEXT_ARGUMENT_OFFSET..])
}

fn resume_argument_index(arguments: &[String]) -> Option<usize> {
    let mut index = 0;
    while let Some(argument) = arguments.get(index) {
        if argument == constants::RESUME_ARGUMENT {
            return Some(index);
        }
        if argument == constants::OPTION_TERMINATOR {
            return None;
        }
        index += option_consumption(arguments, index)?;
    }
    None
}

fn resume_identity(arguments: &[String]) -> SessionIdentity {
    let mut index = 0;
    while let Some(argument) = arguments.get(index) {
        if argument == constants::LAST_ARGUMENT {
            return SessionIdentity::AssociatedOnly;
        }
        if argument == constants::OPTION_TERMINATOR {
            return arguments
                .get(index + constants::NEXT_ARGUMENT_OFFSET)
                .and_then(|value| metadata::normalized_session_id(value))
                .map(SessionIdentity::Resume)
                .unwrap_or(SessionIdentity::AssociatedOnly);
        }
        if constants::VARIABLE_VALUE_ARGUMENTS.contains(&argument.as_str()) {
            return SessionIdentity::AssociatedOnly;
        }
        if constants::VALUE_ARGUMENTS.contains(&argument.as_str()) {
            if arguments
                .get(index + constants::NEXT_ARGUMENT_OFFSET)
                .is_none()
            {
                return SessionIdentity::Invalid;
            }
            index += constants::OPTION_AND_VALUE_COUNT;
            continue;
        }
        if let Some(consumed) = option_consumption(arguments, index) {
            index += consumed;
            continue;
        }
        if argument.starts_with(constants::OPTION_PREFIX) {
            return SessionIdentity::Invalid;
        }
        return metadata::normalized_session_id(argument)
            .map(SessionIdentity::Resume)
            .unwrap_or(SessionIdentity::AssociatedOnly);
    }
    SessionIdentity::AssociatedOnly
}

fn option_consumption(arguments: &[String], index: usize) -> Option<usize> {
    let argument = arguments.get(index)?;
    if constants::BOOLEAN_ARGUMENTS.contains(&argument.as_str()) {
        return Some(constants::SINGLE_ARGUMENT_COUNT);
    }
    if constants::VARIABLE_VALUE_ARGUMENTS.contains(&argument.as_str()) {
        return None;
    }
    if constants::VALUE_ARGUMENTS.contains(&argument.as_str()) {
        return arguments
            .get(index + constants::NEXT_ARGUMENT_OFFSET)
            .map(|_| constants::OPTION_AND_VALUE_COUNT);
    }
    is_inline_value_option(argument).then_some(constants::SINGLE_ARGUMENT_COUNT)
}

fn is_inline_value_option(argument: &str) -> bool {
    constants::VALUE_ARGUMENTS
        .iter()
        .chain(constants::VARIABLE_VALUE_ARGUMENTS)
        .any(|option| option_has_inline_value(argument, option))
}

fn option_has_inline_value(argument: &str, option: &str) -> bool {
    option_inline_value(argument, option).is_some()
}

pub(super) fn option_inline_value<'a>(argument: &'a str, option: &str) -> Option<&'a str> {
    let Some(value) = argument.strip_prefix(option) else {
        return None;
    };
    if option.starts_with(constants::LONG_OPTION_PREFIX) {
        value
            .strip_prefix(constants::LONG_OPTION_ASSIGNMENT)
            .filter(|value| !value.is_empty())
    } else {
        (!value.is_empty()).then_some(value)
    }
}

fn remote_option(argument: &str) -> bool {
    constants::REMOTE_ARGUMENTS
        .iter()
        .any(|option| argument == *option || option_has_inline_value(argument, option))
}

pub(super) fn command_arguments(process: &ObservedProcess) -> Option<&[String]> {
    let executable = file_name(&process.executable)?;
    let argv_executable = process.argv.first().and_then(|value| file_name(value))?;
    if constants::EXECUTABLE_NAMES.contains(&executable) {
        if !constants::EXECUTABLE_NAMES.contains(&argv_executable) {
            return None;
        }
        return process.argv.get(constants::DIRECT_COMMAND_ARGUMENT_INDEX..);
    }
    if !crate::agent::constants::NODE_EXECUTABLES.contains(&executable)
        || !crate::agent::constants::NODE_EXECUTABLES.contains(&argv_executable)
        || !crate::agent::matching::matches_executable_or_node_entrypoint(
            process,
            constants::EXECUTABLE_NAMES,
            constants::NODE_PACKAGE_COMPONENTS,
            constants::NODE_ENTRYPOINTS,
        )
    {
        return None;
    }
    process.argv.get(constants::NODE_COMMAND_ARGUMENT_INDEX..)
}

fn file_name(value: &str) -> Option<&str> {
    Path::new(value).file_name()?.to_str()
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/argv_tests.rs"]
mod tests;
