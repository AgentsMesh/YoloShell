use super::constants;
use crate::agent::ObservedProcess;
use crate::agent::matching::matches_executable_or_node_entrypoint;
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

pub(super) fn matches_node_entrypoint(process: &ObservedProcess) -> bool {
    // A local npm prefix runs `<prefix>/node_modules/.bin/gemini`; a global
    // install runs the bin shim at a path such as `/usr/local/bin/gemini`, so
    // the entrypoint name is accepted without a package component sequence.
    matches_executable_or_node_entrypoint(
        process,
        constants::EXECUTABLE_NAMES,
        constants::NODE_PACKAGE_COMPONENTS,
        constants::NODE_ENTRYPOINTS,
    ) || matches_executable_or_node_entrypoint(
        process,
        constants::EXECUTABLE_NAMES,
        &[],
        constants::NODE_ENTRYPOINTS,
    )
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
    // Node launchers may place options before the entrypoint; slice after the
    // located entrypoint rather than assuming a fixed index.
    let index = crate::agent::matching::node_entrypoint_index(
        &process.argv,
        constants::NODE_PACKAGE_COMPONENTS,
        constants::NODE_ENTRYPOINTS,
    )
    .or_else(|| {
        crate::agent::matching::node_entrypoint_index(
            &process.argv,
            &[],
            constants::NODE_ENTRYPOINTS,
        )
    })?;
    process.argv.get(index + 1..)
}

fn file_name(value: &str) -> Option<&str> {
    std::path::Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
}

pub(super) fn session_identity(arguments: &[String]) -> SessionIdentity {
    let mut found = None;
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        if argument == constants::ARGUMENT_TERMINATOR {
            break;
        }
        let value = if is_session_flag(argument) {
            index += 1;
            arguments.get(index).map(String::as_str)
        } else {
            inline_value(argument)
        };
        let Some(value) = value else {
            index += 1;
            continue;
        };
        match canonical_session_id(value) {
            Some(session_id) => {
                if found.replace(session_id).is_some() {
                    return SessionIdentity::Invalid;
                }
            }
            // `--resume latest` and `--resume <index>` are not exact identities.
            None if is_session_flag(argument) && !is_exact_flag(argument) => {}
            None => return SessionIdentity::Invalid,
        }
        index += 1;
    }
    found.map_or(SessionIdentity::Absent, SessionIdentity::Strong)
}

pub(super) fn resume_session_id(arguments: &[String]) -> Option<String> {
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        if argument == constants::ARGUMENT_TERMINATOR {
            return None;
        }
        let value = if is_resume_flag(argument) {
            index += 1;
            arguments.get(index).map(String::as_str)
        } else {
            inline_resume_value(argument)
        };
        if let Some(value) = value
            && let Some(session_id) = canonical_session_id(value)
        {
            return Some(session_id);
        }
        index += 1;
    }
    None
}

fn is_session_flag(argument: &str) -> bool {
    [
        constants::SESSION_ID_ARGUMENT,
        constants::RESUME_ARGUMENT,
        constants::SHORT_RESUME_ARGUMENT,
    ]
    .contains(&argument)
        || argument.starts_with(&format!("{}{}", constants::SESSION_ID_ARGUMENT, constants::ARGUMENT_VALUE_SEPARATOR))
        || argument.starts_with(&format!("{}{}", constants::RESUME_ARGUMENT, constants::ARGUMENT_VALUE_SEPARATOR))
}

fn is_resume_flag(argument: &str) -> bool {
    argument == constants::RESUME_ARGUMENT
        || argument == constants::SHORT_RESUME_ARGUMENT
        || argument.starts_with(&format!(
            "{}{}",
            constants::RESUME_ARGUMENT,
            constants::ARGUMENT_VALUE_SEPARATOR
        ))
}

fn inline_resume_value(argument: &str) -> Option<&str> {
    let (name, value) = argument.split_once(constants::ARGUMENT_VALUE_SEPARATOR)?;
    (name == constants::RESUME_ARGUMENT).then_some(value)
}

fn is_exact_flag(argument: &str) -> bool {
    argument == constants::SESSION_ID_ARGUMENT
        || argument.starts_with(&format!("{}{}", constants::SESSION_ID_ARGUMENT, constants::ARGUMENT_VALUE_SEPARATOR))
}

fn inline_value(argument: &str) -> Option<&str> {
    let (name, value) = argument.split_once(constants::ARGUMENT_VALUE_SEPARATOR)?;
    [constants::SESSION_ID_ARGUMENT, constants::RESUME_ARGUMENT]
        .contains(&name)
        .then_some(value)
}

fn canonical_session_id(value: &str) -> Option<String> {
    let parsed = Uuid::parse_str(value).ok()?;
    let canonical = parsed.hyphenated().to_string();
    (canonical == value && canonical.len() == constants::SESSION_ID_BYTES).then_some(canonical)
}
