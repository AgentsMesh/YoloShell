use crate::command;
use anyhow::Result;
use yolo_distribution_bundle::ArtifactRole;
use yolo_remote::{
    BundleExecutableLocation, BundleStageRequest, ChannelPurpose, MultiplexedSshConnection,
};

pub(crate) async fn verify(
    connection: &dyn MultiplexedSshConnection,
    stage: &BundleStageRequest,
    location: BundleExecutableLocation,
    role: ArtifactRole,
) -> Result<()> {
    let response = connection
        .verify_bundle_executable(stage, location, role)
        .await?;
    command::validate_channel(connection, response.identity, ChannelPurpose::FileRead)
}
