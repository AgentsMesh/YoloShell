use super::{
    RusshConnection, bundle_directory as directory, bundle_file, bundle_raw::*,
    bundle_task::StageTask,
};
use crate::remote::{
    BundleStageRequest, BundleStageResult, ChannelOutcome, ChannelPurpose, RemoteError,
    cleanup_registry::StageRetention,
};
use russh_sftp::client::RawSftpSession;
use std::{sync::Arc, time::Duration};
use yolo_distribution_bundle::{ArtifactRole, BIN_DIRECTORY, MANIFEST_FILE_NAME};
use yolo_distribution_publication_protocol::{EXECUTABLE_MODE, MANIFEST_MODE};

impl RusshConnection {
    pub(super) async fn stage_bundle_sftp(
        &self,
        request: &BundleStageRequest,
    ) -> Result<ChannelOutcome<BundleStageResult>, RemoteError> {
        let retention = self
            .cleanup
            .reserve_stage(self.info.connection_id, request)?;
        let reservation = self.cleanup.reserve()?;
        let uid = self.bundle_owner().await?;
        let raw = self.open_bundle_sftp(true).await?;
        let identity = self.identity(ChannelPurpose::RuntimeDeployment);
        let value = StageTask::start(
            raw,
            Arc::clone(&self.session),
            request.clone(),
            uid,
            self.operation_timeout,
            reservation,
            retention,
        )
        .finish()
        .await?;
        Ok(ChannelOutcome { identity, value })
    }
}

pub(super) async fn stage(
    raw: &RawSftpSession,
    request: &BundleStageRequest,
    uid: u32,
    retention: &StageRetention,
    limit: Duration,
) -> Result<BundleStageResult, RemoteError> {
    let owner = directory::root(raw, request.install_root(), true, limit)
        .await?
        .ok_or_else(invalid)?;
    if owner.uid() != uid {
        return Err(invalid());
    }
    let stage = request.stage_directory();
    let root = directory::text(request.install_root())?;
    owner.verify(raw, root, limit).await?;
    // From the first attempted mutation this name may exist, but SFTP cannot prove its inode.
    retention.attempted();
    directory::create(raw, &stage, owner, limit).await?;
    let bin = stage.join(BIN_DIRECTORY);
    directory::create(raw, &bin, owner, limit).await?;
    for role in ArtifactRole::ALL {
        owner.verify(raw, root, limit).await?;
        owner.verify(raw, directory::text(&stage)?, limit).await?;
        owner.verify(raw, directory::text(&bin)?, limit).await?;
        let path = request.executable(role);
        bundle_file::upload(
            raw,
            directory::text(&path)?,
            request.payload(role),
            EXECUTABLE_MODE,
            uid,
            limit,
        )
        .await?;
    }
    let manifest = stage.join(MANIFEST_FILE_NAME);
    bundle_file::upload(
        raw,
        directory::text(&manifest)?,
        request.manifest(),
        MANIFEST_MODE,
        uid,
        limit,
    )
    .await?;
    owner.verify(raw, root, limit).await?;
    owner.verify(raw, directory::text(&stage)?, limit).await?;
    owner.verify(raw, directory::text(&bin)?, limit).await?;
    for role in ArtifactRole::ALL {
        let path = request.executable(role);
        bundle_file::verify(
            raw,
            directory::text(&path)?,
            request.payload(role),
            EXECUTABLE_MODE,
            uid,
            limit,
        )
        .await?;
    }
    bundle_file::verify(
        raw,
        directory::text(&manifest)?,
        request.manifest(),
        MANIFEST_MODE,
        uid,
        limit,
    )
    .await?;
    Ok(BundleStageResult::from_request(request))
}
