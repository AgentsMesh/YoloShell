use super::constants;
use crate::agent::ObservedProcess;

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

pub(super) fn matches_executable(process: &ObservedProcess) -> bool {
    let argv_executable = process.argv.first().and_then(|value| file_name(value));
    let executable = file_name(&process.executable);
    argv_executable.is_some_and(is_launcher_name) && executable.is_some_and(is_program_name)
}

fn is_launcher_name(name: &str) -> bool {
    constants::EXECUTABLE_NAMES.contains(&name)
}

/// The launched program is either the launcher symlink itself or the versioned
/// native binary it points to (`grok-linux-aarch64`, `grok-darwin-arm64`).
fn is_program_name(name: &str) -> bool {
    is_launcher_name(name) || name.starts_with(constants::EXECUTABLE_PREFIX)
}

pub(super) fn command_arguments(process: &ObservedProcess) -> Option<&[String]> {
    matches_executable(process).then(|| {
        process
            .argv
            .get(constants::DIRECT_COMMAND_ARGUMENT_INDEX..)
            .unwrap_or_default()
    })
}

fn file_name(value: &str) -> Option<&str> {
    std::path::Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
}

/// `-r <uuid>`/`--resume <uuid>` and `-s <uuid>` name an exact session; a
/// title or `--continue` resolves at launch time and is not an identity.
pub(super) fn session_identity(arguments: &[String]) -> SessionIdentity {
    let mut found = None;
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        if argument == constants::ARGUMENT_TERMINATOR {
            break;
        }
        let (kind, value) = if let Some(flag) = exact_flag(argument) {
            index += 1;
            (flag, arguments.get(index).map(String::as_str))
        } else if let Some((flag, value)) = inline_flag(argument) {
            (flag, Some(value))
        } else {
            index += 1;
            continue;
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
            // A resume title resolves at launch time; only `--session-id`
            // names a new session that must be an exact UUID.
            None if kind == FlagKind::SessionId => return SessionIdentity::Invalid,
            None => {}
        }
        index += 1;
    }
    found.map_or(SessionIdentity::Absent, SessionIdentity::Strong)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FlagKind {
    SessionId,
    Resume,
}

fn exact_flag(argument: &str) -> Option<FlagKind> {
    match argument {
        constants::SESSION_ID_ARGUMENT | constants::SHORT_SESSION_ID_ARGUMENT => {
            Some(FlagKind::SessionId)
        }
        constants::RESUME_ARGUMENT | constants::SHORT_RESUME_ARGUMENT => Some(FlagKind::Resume),
        _ => None,
    }
}

fn inline_flag(argument: &str) -> Option<(FlagKind, &str)> {
    let (name, value) = argument.split_once(constants::ARGUMENT_VALUE_SEPARATOR)?;
    let kind = match name {
        constants::SESSION_ID_ARGUMENT => FlagKind::SessionId,
        constants::RESUME_ARGUMENT => FlagKind::Resume,
        _ => return None,
    };
    Some((kind, value))
}

pub(super) fn resume_session_id(arguments: &[String]) -> Option<String> {
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        if argument == constants::ARGUMENT_TERMINATOR {
            return None;
        }
        let value = if let Some(value) = inline_resume_value(argument) {
            Some(value)
        } else if is_resume_flag(argument) {
            index += 1;
            arguments.get(index).map(String::as_str)
        } else {
            None
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

fn is_resume_flag(argument: &str) -> bool {
    [
        constants::RESUME_ARGUMENT,
        constants::SHORT_RESUME_ARGUMENT,
    ]
    .contains(&argument)
}

fn inline_resume_value(argument: &str) -> Option<&str> {
    let (name, value) = argument.split_once(constants::ARGUMENT_VALUE_SEPARATOR)?;
    (name == constants::RESUME_ARGUMENT).then_some(value)
}

fn canonical_session_id(value: &str) -> Option<String> {
    super::metadata::normalized_session_id(value)
}
