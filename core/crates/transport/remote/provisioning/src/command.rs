use crate::constants::*;
use anyhow::{Result, ensure};
use yolo_distribution_bundle::ArtifactRole;
use yolo_distribution_publication_protocol::{INSTALL_ROOT_ARGUMENT, REQUEST_CAPNP_ARGUMENT};
use yolo_launch_contract::{HOST_HOME_ARGUMENT, HUB_STATE_ARGUMENT};
use yolo_remote_provisioning_protocol::{
    MAX_BOOTSTRAP_DOCUMENT_BYTES, REMOTE_BOOTSTRAP_ARGUMENT, REMOTE_BOOTSTRAP_RECOVERY_ARGUMENT,
};
use yolo_remote::{
    BundleExecutableLocation, BundleStageRequest, ChannelPurpose, CommandPolicy, CommandRequest,
    MultiplexedSshConnection,
};
use yolo_remote_provisioning_protocol::ProvisionRequest;

pub(crate) async fn run(
    connection: &dyn MultiplexedSshConnection,
    request: &ProvisionRequest,
    stage: &BundleStageRequest,
    recovery: bool,
) -> Result<Vec<u8>> {
    let verified = connection
        .verify_bundle_executable(
            stage,
            BundleExecutableLocation::Published,
            ArtifactRole::Runtime,
        )
        .await?;
    ensure!(
        verified.identity.connection_id == connection.info().connection_id
            && verified.identity.purpose == ChannelPurpose::FileRead,
        INVALID_CHANNEL
    );
    let fields = request.fields();
    let program = fields
        .source
        .descriptor()
        .executable_path(&fields.install_root, ArtifactRole::Runtime);
    let program = program.to_str().ok_or_else(|| anyhow::anyhow!(INVALID))?;
    let mut args: Vec<String> = yolo_launch_contract::ProcessRole::Hub
        .arguments()
        .into_iter()
        .map(str::to_owned)
        .collect();
    args.extend([
        if recovery {
            REMOTE_BOOTSTRAP_RECOVERY_ARGUMENT
        } else {
            REMOTE_BOOTSTRAP_ARGUMENT
        }
        .into(),
        INSTALL_ROOT_ARGUMENT.into(),
        fields.install_root.to_string_lossy().into_owned(),
        HUB_STATE_ARGUMENT.into(),
        fields.state.to_string_lossy().into_owned(),
        HOST_HOME_ARGUMENT.into(),
        fields.host_home.to_string_lossy().into_owned(),
        REQUEST_CAPNP_ARGUMENT.into(),
        yolo_capnp_argument::encode(&fields.bootstrap, MAX_BOOTSTRAP_DOCUMENT_BYTES)?,
    ]);
    let policy = CommandPolicy::new([program.to_owned()])?;
    let request = CommandRequest::new(program, args);
    let timeout = request.timeout();
    let command = policy.authorize(request.with_limits(timeout, MAX_BOOTSTRAP_DOCUMENT_BYTES))?;
    let response = connection.execute(&command).await?;
    ensure!(
        response.identity.connection_id == connection.info().connection_id
            && response.identity.purpose == ChannelPurpose::Command,
        INVALID_CHANNEL
    );
    response.value.validate(&command)?;
    ensure!(
        !response.value.truncated
            && !response.value.timed_out
            && response.value.exit_status == Some(SUCCESS),
        COMMAND_FAILED
    );
    Ok(response.value.stdout)
}
