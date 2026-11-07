use super::constants;
use crate::agent::ObservedProcess;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SessionIdentity {
    Absent,
    Invalid,
    Strong(String),
}

impl SessionIdentity {
    pub(super) fn is_valid(&self) -> bool {
        !matches!(self, Self::Invalid)
    }

    pub(super) fn into_session_id(self) -> Option<String> {
        match self {
            Self::Strong(session_id) => Some(session_id),
            Self::Absent | Self::Invalid => None,
        }
    }
}

pub(super) fn session_identity(arguments: &[String]) -> SessionIdentity {
    let mut found = None;
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        if argument == constants::ARGUMENT_TERMINATOR {
            break;
        }
        if conflicts_with_exact_session(argument) {
            return SessionIdentity::Invalid;
        }
        let parsed = if is_identity_flag(argument) {
            index += 1;
            arguments.get(index).map(String::as_str)
        } else {
            inline_identity_value(argument)
        };
        if let Some(value) = parsed {
            let Some(session_id) = canonical_session_id(value) else {
                return SessionIdentity::Invalid;
            };
            if found.replace(session_id).is_some() {
                return SessionIdentity::Invalid;
            }
        } else if is_identity_flag(argument) {
            return SessionIdentity::Invalid;
        }
        index += 1;
    }
    found.map_or(SessionIdentity::Absent, SessionIdentity::Strong)
}

pub(super) fn resume_session_id(arguments: &[String]) -> Option<String> {
    let mut found = None;
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        if argument == constants::ARGUMENT_TERMINATOR {
            break;
        }
        if conflicts_with_exact_resume(argument) {
            return None;
        }
        let parsed = if is_resume_flag(argument) {
            index += 1;
            arguments.get(index).map(String::as_str)
        } else {
            inline_resume_value(argument)
        };
        if let Some(value) = parsed {
            let session_id = canonical_session_id(value)?;
            if found.replace(session_id).is_some() {
                return None;
            }
        } else if is_resume_flag(argument) {
            return None;
        }
        index += 1;
    }
    found
}

pub(super) fn command_arguments(process: &ObservedProcess) -> Option<&[String]> {
    let executable = file_name(&process.executable)?;
    let argv_executable = process.argv.first().and_then(|value| file_name(value))?;
    if constants::EXECUTABLE_NAMES.contains(&executable) {
        if !constants::EXECUTABLE_NAMES.contains(&argv_executable) {
            return None;
        }
        return process
            .argv
            .get(constants::DIRECT_COMMAND_ARGUMENT_INDEX..);
    }
    if !crate::agent::constants::NODE_EXECUTABLES.contains(&executable)
        || !crate::agent::constants::NODE_EXECUTABLES.contains(&argv_executable)
        || !matches_node_entrypoint(process)
    {
        return None;
    }
    process
        .argv
        .get(constants::NODE_COMMAND_ARGUMENT_INDEX..)
}

pub(super) fn matches_node_entrypoint(process: &ObservedProcess) -> bool {
    crate::agent::matching::matches_executable_or_node_entrypoint(
        process,
        constants::EXECUTABLE_NAMES,
        constants::NODE_PACKAGE_COMPONENTS,
        constants::NODE_ENTRYPOINTS,
    ) || crate::agent::matching::matches_executable_or_node_entrypoint(
        process,
        constants::EXECUTABLE_NAMES,
        &["@qwen-code", "qwen-code"],
        constants::NODE_PACKAGE_ENTRYPOINTS,
    )
}

fn is_identity_flag(argument: &str) -> bool {
    [
        constants::RESUME_ARGUMENT,
        constants::SHORT_RESUME_ARGUMENT,
        constants::SESSION_ID_ARGUMENT,
    ]
    .contains(&argument)
}

fn is_resume_flag(argument: &str) -> bool {
    [constants::RESUME_ARGUMENT, constants::SHORT_RESUME_ARGUMENT].contains(&argument)
}

fn inline_identity_value(argument: &str) -> Option<&str> {
    [constants::RESUME_ARGUMENT, constants::SESSION_ID_ARGUMENT]
        .iter()
        .find_map(|flag| inline_value(argument, flag))
}

fn inline_resume_value(argument: &str) -> Option<&str> {
    inline_value(argument, constants::RESUME_ARGUMENT)
}

fn inline_value<'a>(argument: &'a str, flag: &str) -> Option<&'a str> {
    argument
        .strip_prefix(flag)?
        .strip_prefix(constants::ARGUMENT_VALUE_SEPARATOR)
        .filter(|value| !value.is_empty())
}

fn conflicts_with_exact_session(argument: &str) -> bool {
    [
        constants::CONTINUE_ARGUMENT,
        constants::SHORT_CONTINUE_ARGUMENT,
        constants::FORK_SESSION_ARGUMENT,
    ]
    .contains(&argument)
}

fn conflicts_with_exact_resume(argument: &str) -> bool {
    conflicts_with_exact_session(argument) || argument == constants::SESSION_ID_ARGUMENT
}

fn canonical_session_id(value: &str) -> Option<String> {
    let parsed = Uuid::parse_str(value).ok()?;
    let canonical = parsed.hyphenated().to_string();
    (canonical == value && canonical.len() == constants::SESSION_ID_BYTES).then_some(canonical)
}

fn file_name(value: &str) -> Option<&str> {
    Path::new(value).file_name()?.to_str()
}

#[cfg(test)]
#[path = "../unit_tests/argv_tests.rs"]
mod tests;
