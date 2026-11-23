use super::{argv, constants, metadata};
use crate::agent::ObservedProcess;
use std::path::Path;
use yolo_codex_identity::managed_launch::{LEGACY_SETTING, NO_DAEMON};

pub(super) fn session_id(process: &ObservedProcess) -> Option<String> {
    let arguments = argv::command_arguments(process)?;
    let arguments = match arguments {
        [flag, rest @ ..] if flag == NO_DAEMON => rest,
        [flag, setting, rest @ ..] if flag == constants::CONFIG_ARGUMENT && setting == LEGACY_SETTING => rest,
        _ => arguments,
    };
    let candidate = match arguments {
        [resume, session_id] if resume == constants::RESUME_ARGUMENT => session_id,
        [resume, cwd_option, cwd, session_id]
            if resume == constants::RESUME_ARGUMENT
                && cwd_option == constants::CWD_ARGUMENT
                && Path::new(cwd).is_absolute() =>
        {
            session_id
        }
        _ => return None,
    };
    metadata::canonical_session_id(candidate)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/exact_resume_tests.rs"]
mod tests;
