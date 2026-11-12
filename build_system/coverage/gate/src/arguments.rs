use crate::constants::{
    BUILD_WORKSPACE_DIRECTORY_ENV, CORE_SCOPE_ARGUMENT, OPTION_PREFIX, PROGRAM_NAME_ARGUMENTS,
    TOOLING_SCOPE_ARGUMENT, USAGE,
};
use std::ffi::OsString;
use std::path::PathBuf;
use yolo_coverage_protocol::ProductionScope;

pub struct Arguments {
    pub path: PathBuf,
    pub scope: ProductionScope,
}

pub fn load() -> Result<Arguments, String> {
    let mut arguments = parse(std::env::args_os().skip(PROGRAM_NAME_ARGUMENTS))?;
    arguments.path = resolve_report_path(
        arguments.path,
        std::env::var_os(BUILD_WORKSPACE_DIRECTORY_ENV).map(PathBuf::from),
    );
    Ok(arguments)
}

pub fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Arguments, String> {
    let mut arguments = arguments.into_iter();
    let first = arguments.next().ok_or_else(|| USAGE.to_owned())?;
    let scope = match first.to_str() {
        Some(CORE_SCOPE_ARGUMENT) => Some(ProductionScope::Core),
        Some(TOOLING_SCOPE_ARGUMENT) => Some(ProductionScope::Tooling),
        Some(value) if value.starts_with(OPTION_PREFIX) => return Err(USAGE.to_owned()),
        _ => None,
    };
    let path = if scope.is_some() {
        arguments.next().ok_or_else(|| USAGE.to_owned())?
    } else {
        first
    };
    if arguments.next().is_some()
        || path.is_empty()
        || path
            .to_str()
            .is_some_and(|value| value.starts_with(OPTION_PREFIX))
    {
        return Err(USAGE.to_owned());
    }
    Ok(Arguments {
        path: path.into(),
        scope: scope.unwrap_or(ProductionScope::Core),
    })
}

pub fn resolve_report_path(path: PathBuf, workspace: Option<PathBuf>) -> PathBuf {
    if path.is_absolute() {
        return path;
    }
    workspace.map_or(path.clone(), |root| root.join(path))
}
