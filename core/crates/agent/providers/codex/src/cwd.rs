use super::{argv, constants, metadata};
use crate::agent::ObservedProcess;
use std::path::{Path, PathBuf};

pub(super) enum CwdOverride {
    Absent,
    Invalid,
    Resolved(PathBuf),
}

pub(super) fn process_override(process: &ObservedProcess) -> CwdOverride {
    let Some(arguments) = argv::command_arguments(process) else {
        return CwdOverride::Invalid;
    };
    let Some(base) = process.cwd.as_deref() else {
        return CwdOverride::Invalid;
    };
    let mut resolved = None;
    let mut index = 0;
    while let Some(argument) = arguments.get(index) {
        if argument == constants::OPTION_TERMINATOR {
            break;
        }
        if constants::CWD_ARGUMENTS.contains(&argument.as_str()) {
            let Some(value) = arguments.get(index + constants::NEXT_ARGUMENT_OFFSET) else {
                return CwdOverride::Invalid;
            };
            let Some(cwd) = resolve(base, value) else {
                return CwdOverride::Invalid;
            };
            resolved = Some(cwd);
            index += constants::OPTION_AND_VALUE_COUNT;
            continue;
        }
        if let Some(value) = inline_value(argument) {
            let Some(cwd) = resolve(base, value) else {
                return CwdOverride::Invalid;
            };
            resolved = Some(cwd);
        }
        index += constants::SINGLE_ARGUMENT_COUNT;
    }
    resolved.map_or(CwdOverride::Absent, CwdOverride::Resolved)
}

fn resolve(base: &Path, value: &str) -> Option<PathBuf> {
    let value = Path::new(value);
    let absolute = if value.is_absolute() {
        value.to_path_buf()
    } else {
        base.join(value)
    };
    metadata::canonical_existing_cwd(&absolute)
}

fn inline_value(argument: &str) -> Option<&str> {
    constants::CWD_ARGUMENTS
        .iter()
        .find_map(|option| argv::option_inline_value(argument, option))
}
