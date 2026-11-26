use crate::paths::{Paths, SOCKET_ROOT_ENVIRONMENT, socket_namespace::SocketNamespace};
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};
use uuid::Uuid;

pub(super) fn command(
    executable: &Path,
    paths: &Paths,
    terminal_id: Uuid,
    namespace: &SocketNamespace,
    input: Stdio,
) -> Command {
    let mut command = Command::new(executable);
    command
        .args(arguments(paths.home(), terminal_id))
        .env(SOCKET_ROOT_ENVIRONMENT, namespace.root_path())
        .stdin(input)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    crate::process::configure_detached_command(&mut command);
    command
}

fn arguments(home: &Path, terminal_id: Uuid) -> Vec<OsString> {
    crate::cli::ProcessRole::Host.prepend([
        crate::cli::constants::CUSTODY_GUARDIAN_COMMAND.into(),
        crate::cli::constants::HOME_ARGUMENT.into(),
        home.as_os_str().to_owned(),
        crate::cli::constants::TERMINAL_ID_ARGUMENT.into(),
        terminal_id.to_string().into(),
    ])
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/spawn/spawn_command_tests.rs"]
mod tests;
