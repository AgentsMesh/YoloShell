use crate::{command, constants::*, executable, probes};
use anyhow::{Result, ensure};
use yolo_distribution_bundle::ArtifactRole;
use yolo_distribution_publication_protocol::*;
use yolo_remote::{BundleExecutableLocation, BundleStageRequest, MultiplexedSshConnection};

pub(crate) async fn inspect(
    connection: &dyn MultiplexedSshConnection,
    stage: &BundleStageRequest,
    request: &PublicationRequest,
) -> Result<PublicationReceipt> {
    let published = stage
        .descriptor()
        .executable_path(stage.install_root(), ArtifactRole::Runtime);
    let program = match executable::verify(
        connection,
        stage,
        BundleExecutableLocation::Published,
        ArtifactRole::Runtime,
    )
    .await
    {
        Ok(()) => published,
        Err(_) => {
            let uploaded = stage.executable(ArtifactRole::Runtime);
            executable::verify(
                connection,
                stage,
                BundleExecutableLocation::Staged,
                ArtifactRole::Runtime,
            )
            .await?;
            uploaded
        }
    };
    probes::version(
        connection,
        &program,
        ArtifactRole::Runtime,
        stage.descriptor().version(),
    )
    .await?;
    let output = command::run(
        connection,
        program
            .to_str()
            .ok_or_else(|| anyhow::anyhow!(INVALID_PATH))?,
        command::hub_arguments([
            INSPECT_INSTALL_ARGUMENT.into(),
            INSTALL_ROOT_ARGUMENT.into(),
            stage.install_root().to_string_lossy().into_owned(),
        ]),
    )
    .await?;
    let inspection = decode_inspection(&output)?;
    inspection.validate(stage.install_root())?;
    let selected = inspection
        .selected()
        .ok_or_else(|| anyhow::anyhow!(RECOVERY_MISMATCH))?;
    ensure!(
        selected.descriptor() == request.descriptor()
            && selected.selection() == &request.selected()?,
        RECOVERY_MISMATCH
    );
    PublicationReceipt::new(request, stage.install_root(), selected.selection().clone())
}
