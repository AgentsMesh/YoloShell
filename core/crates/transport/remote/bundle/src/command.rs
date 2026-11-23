use crate::constants::*;
use anyhow::{Result, ensure};
use yolo_distribution_publication_protocol::MAX_PUBLICATION_BYTES;
use yolo_remote::{
    ChannelIdentity, ChannelPurpose, CommandPolicy, CommandRequest, MultiplexedSshConnection,
};

pub(crate) fn hub_arguments(arguments: impl IntoIterator<Item = String>) -> Vec<String> {
    yolo_launch_contract::ProcessRole::Hub
        .arguments()
        .into_iter()
        .map(str::to_owned)
        .chain(arguments)
        .collect()
}

pub(crate) fn validate_channel(
    connection: &dyn MultiplexedSshConnection,
    identity: ChannelIdentity,
    purpose: ChannelPurpose,
) -> Result<()> {
    ensure!(
        identity.connection_id == connection.info().connection_id && identity.purpose == purpose,
        INVALID_CHANNEL
    );
    Ok(())
}

pub(crate) async fn run(
    connection: &dyn MultiplexedSshConnection,
    program: &str,
    args: Vec<String>,
) -> Result<Vec<u8>> {
    let policy = CommandPolicy::new([program.to_owned()])?;
    let command = policy.authorize(
        CommandRequest::new(program, args).with_limits(COMMAND_TIMEOUT, MAX_PUBLICATION_BYTES),
    )?;
    let response = connection.execute(&command).await?;
    validate_channel(connection, response.identity, ChannelPurpose::Command)?;
    response.value.validate(&command)?;
    ensure!(
        !response.value.timed_out
            && !response.value.truncated
            && response.value.exit_status == Some(SUCCESS),
        COMMAND_FAILED
    );
    Ok(response.value.stdout)
}
