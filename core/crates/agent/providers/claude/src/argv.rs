use super::constants;
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
        let parsed = if is_identity_flag(argument) {
            index += 1;
            arguments.get(index).map(String::as_str)
        } else {
            identity_value(argument)
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
            resume_identity_value(argument)
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

fn identity_value(argument: &str) -> Option<&str> {
    // Claude documents the short resume flag only in its separate-value form.
    [constants::RESUME_ARGUMENT, constants::SESSION_ID_ARGUMENT]
        .iter()
        .find_map(|flag| {
            argument
                .strip_prefix(flag)?
                .strip_prefix(constants::ARGUMENT_VALUE_SEPARATOR)
        })
}

fn resume_identity_value(argument: &str) -> Option<&str> {
    argument
        .strip_prefix(constants::RESUME_ARGUMENT)?
        .strip_prefix(constants::ARGUMENT_VALUE_SEPARATOR)
}

fn conflicts_with_exact_resume(argument: &str) -> bool {
    [
        constants::CONTINUE_ARGUMENT,
        constants::SHORT_CONTINUE_ARGUMENT,
        constants::FORK_SESSION_ARGUMENT,
        constants::SESSION_ID_ARGUMENT,
    ]
    .iter()
    .any(|flag| {
        argument == *flag
            || argument
                .strip_prefix(flag)
                .is_some_and(|value| value.starts_with(constants::ARGUMENT_VALUE_SEPARATOR))
    })
}

fn canonical_session_id(value: &str) -> Option<String> {
    let parsed = Uuid::parse_str(value).ok()?;
    let canonical = parsed.hyphenated().to_string();
    (canonical == value).then_some(canonical)
}
