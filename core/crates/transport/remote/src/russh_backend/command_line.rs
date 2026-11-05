use super::super::constants::{
    SHELL_AND_EXEC, SHELL_CHANGE_DIRECTORY, SHELL_PROGRAM_SLOT_COUNT, SHELL_SEPARATOR,
    SHELL_SINGLE_QUOTE_ESCAPE,
};
use super::{AuthorizedCommand, RemoteError, RemoteRuntimeSpec};

pub(super) fn runtime_command(runtime: &RemoteRuntimeSpec) -> Result<String, RemoteError> {
    shell_join(runtime.executable_text(), &runtime.args())
}

pub(super) fn command_line(command: &AuthorizedCommand) -> Result<String, RemoteError> {
    let request = command.request();
    let execution = shell_join(request.program(), request.args())?;
    let Some(cwd) = request.cwd() else {
        return Ok(execution);
    };
    let cwd = cwd
        .to_str()
        .ok_or_else(|| RemoteError::InvalidCommand("cwd must be valid UTF-8".into()))?;
    Ok([
        SHELL_CHANGE_DIRECTORY,
        &quote(cwd)?,
        SHELL_AND_EXEC,
        &execution,
    ]
    .join(SHELL_SEPARATOR))
}

fn shell_join(program: &str, args: &[String]) -> Result<String, RemoteError> {
    let mut values = Vec::with_capacity(args.len() + SHELL_PROGRAM_SLOT_COUNT);
    values.push(quote(program)?);
    for argument in args {
        values.push(quote(argument)?);
    }
    Ok(values.join(SHELL_SEPARATOR))
}

fn quote(value: &str) -> Result<String, RemoteError> {
    if value.contains('\0') {
        return Err(RemoteError::InvalidCommand("argument contains NUL".into()));
    }
    Ok(format!(
        "'{}'",
        value.replace('\'', SHELL_SINGLE_QUOTE_ESCAPE)
    ))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/russh_backend/command_line_tests.rs"]
mod tests;
