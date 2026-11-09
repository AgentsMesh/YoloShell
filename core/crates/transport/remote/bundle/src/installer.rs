use crate::{UncertainPublication, command, constants::*, executable, probes, recovery, selection};
use anyhow::{Result, ensure};
use uuid::Uuid;
use yolo_distribution_bundle::ArtifactRole;
use yolo_distribution_publication_protocol::*;
use yolo_remote::{
    BundleExecutableLocation, BundleStageRequest, ChannelPurpose, MultiplexedSshConnection,
};

#[derive(Default)]
pub struct RemoteBundleInstaller;

impl RemoteBundleInstaller {
    /// Reads the expected selector without staging or committing. Persist the returned
    /// request before publish if recovery must survive a caller crash.
    pub async fn prepare(
        &self,
        connection: &dyn MultiplexedSshConnection,
        stage: &BundleStageRequest,
        operation: Uuid,
    ) -> Result<PublicationRequest> {
        let expected = selection::read(connection, stage.install_root()).await?;
        PublicationRequest::new(
            stage.install_root().to_owned(),
            operation,
            stage.stage_id(),
            stage.descriptor().clone(),
            expected,
        )
    }

    pub async fn install(
        &self,
        connection: &dyn MultiplexedSshConnection,
        stage: &BundleStageRequest,
        operation: Uuid,
    ) -> Result<PublicationReceipt> {
        let request = self.prepare(connection, stage, operation).await?;
        self.publish(connection, stage, &request).await
    }

    /// At most one commit command is dispatched. Ambiguity invokes read-only inspection.
    pub async fn publish(
        &self,
        connection: &dyn MultiplexedSshConnection,
        stage: &BundleStageRequest,
        request: &PublicationRequest,
    ) -> Result<PublicationReceipt> {
        ensure!(
            request.stage() == stage.stage_id() && request.descriptor() == stage.descriptor(),
            REQUEST_MISMATCH
        );
        request.validate_root(stage.install_root())?;
        probes::target(connection, stage.descriptor().target()).await?;
        let selected = selection::read(connection, stage.install_root()).await?;
        if selected.as_ref() == Some(&request.selected()?) {
            return self.recover(connection, stage, request).await;
        }
        ensure!(selected.as_ref() == request.expected(), SELECTION_CONFLICT);
        let response = connection.stage_bundle(stage).await?;
        command::validate_channel(
            connection,
            response.identity,
            ChannelPurpose::RuntimeDeployment,
        )?;
        let uploaded = response.value;
        ensure!(
            uploaded.stage_id() == stage.stage_id()
                && uploaded.install_root() == stage.install_root()
                && uploaded.stage_directory() == stage.stage_directory()
                && uploaded.descriptor() == stage.descriptor(),
            STAGE_MISMATCH
        );
        for role in ArtifactRole::ALL {
            ensure!(
                uploaded.executable(role) == stage.executable(role),
                STAGE_MISMATCH
            );
            executable::verify(connection, stage, BundleExecutableLocation::Staged, role).await?;
            probes::version(
                connection,
                &uploaded.executable(role),
                role,
                stage.descriptor().version(),
            )
            .await?;
        }
        let outcome = commit(connection, stage, request).await;
        match outcome {
            Ok(receipt) => Ok(receipt),
            Err(primary) => self
                .recover(connection, stage, request)
                .await
                .map_err(|recovery| {
                    anyhow::Error::new(UncertainPublication {
                        request: request.clone(),
                        cause: primary.context(recovery),
                    })
                }),
        }
    }

    /// Recovery never uploads or commits, including when a newer selector is observed.
    pub async fn recover(
        &self,
        connection: &dyn MultiplexedSshConnection,
        stage: &BundleStageRequest,
        request: &PublicationRequest,
    ) -> Result<PublicationReceipt> {
        request.validate_root(stage.install_root())?;
        ensure!(
            request.stage() == stage.stage_id() && request.descriptor() == stage.descriptor(),
            REQUEST_MISMATCH
        );
        recovery::inspect(connection, stage, request).await
    }
}

async fn commit(
    connection: &dyn MultiplexedSshConnection,
    stage: &BundleStageRequest,
    request: &PublicationRequest,
) -> Result<PublicationReceipt> {
    let program = stage.executable(ArtifactRole::Runtime);
    let program = program
        .to_str()
        .ok_or_else(|| anyhow::anyhow!(INVALID_PATH))?;
    let root = stage
        .install_root()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!(INVALID_PATH))?;
    let output = command::run(
        connection,
        program,
        command::hub_arguments([
            INSTALL_BUNDLE_ARGUMENT.into(),
            INSTALL_ROOT_ARGUMENT.into(),
            root.into(),
            REQUEST_CAPNP_ARGUMENT.into(),
            encode_request_argument(request)?,
        ]),
    )
    .await?;
    let receipt = decode_receipt(&output)?;
    receipt.validate(request, stage.install_root())?;
    Ok(receipt)
}
