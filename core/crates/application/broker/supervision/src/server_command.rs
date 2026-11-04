use crate::{ChildLaunch, CredentialSource};
use anyhow::Result;
use std::ffi::OsString;
use std::time::Instant;
use yolo_cli::constants;

pub(super) struct ServerCommand {
    pub(super) arguments: Vec<OsString>,
    pub(super) deadline: Instant,
}

pub(super) fn build(
    command: &str,
    launch: &ChildLaunch,
    credential_source: CredentialSource,
    activation_fd: Option<i32>,
) -> Result<ServerCommand> {
    let expected_revision = launch.config_revision();
    let expected_anchor = launch.state_anchor().to_string();
    let expected_authority = launch.authority().to_string();
    let expected_deadline = launch.startup_deadline();
    let deadline = expected_deadline.to_instant()?.into_std();
    let mut arguments = yolo_cli::ProcessRole::Remote.prepend([
        command.into(),
        constants::CONFIG_ARGUMENT.into(),
        launch.config().as_os_str().to_owned(),
        constants::STATE_DIRECTORY_ARGUMENT.into(),
        launch.state_dir().as_os_str().to_owned(),
        constants::EXPECTED_REVISION_ARGUMENT.into(),
        expected_revision.into(),
        constants::EXPECTED_ANCHOR_ARGUMENT.into(),
        expected_anchor.into(),
        constants::EXPECTED_AUTHORITY_ARGUMENT.into(),
        expected_authority.into(),
        constants::EXPECTED_STARTUP_DEADLINE_ARGUMENT.into(),
        expected_deadline.to_string().into(),
    ]);
    if matches!(credential_source, CredentialSource::PrivateStdin) {
        arguments.push(constants::IDENTITY_PASSPHRASES_STDIN_ARGUMENT.into());
    }
    if let Some(descriptor) = activation_fd {
        arguments.extend([
            constants::SUPERVISION_ACTIVATION_FD_ARGUMENT.into(),
            descriptor.to_string().into(),
        ]);
    }
    Ok(ServerCommand {
        arguments,
        deadline,
    })
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/command_encoding.rs"]
mod tests;
