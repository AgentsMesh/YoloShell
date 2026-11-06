use super::{
    RusshConnection, bundle_directory as directory, bundle_file, bundle_raw::*,
    deployment_operation::settle,
};
use crate::remote::{
    BundleExecutableLocation, BundleStageRequest, ChannelOutcome, ChannelPurpose, RemoteError,
};
use yolo_distribution_bundle::{ArtifactRole, BIN_DIRECTORY, RELEASES_DIRECTORY};
use yolo_distribution_publication_protocol::{EXECUTABLE_MODE, MAX_INSTALL_PATH_BYTES};

impl RusshConnection {
    pub(super) async fn verify_bundle_executable_sftp(
        &self,
        request: &BundleStageRequest,
        location: BundleExecutableLocation,
        role: ArtifactRole,
    ) -> Result<ChannelOutcome<()>, RemoteError> {
        let uid = self.bundle_owner().await?;
        let raw = self.open_bundle_sftp(false).await?;
        let identity = self.identity(ChannelPurpose::FileRead);
        let result = async {
            let owner =
                directory::root(&raw, request.install_root(), false, self.operation_timeout)
                    .await?
                    .ok_or_else(invalid)?;
            if owner.uid() != uid {
                return Err(invalid());
            }
            let directory = match location {
                BundleExecutableLocation::Staged => request.stage_directory(),
                BundleExecutableLocation::Published => {
                    owner
                        .verify(
                            &raw,
                            directory::text(&request.install_root().join(RELEASES_DIRECTORY))?,
                            self.operation_timeout,
                        )
                        .await?;
                    request
                        .descriptor()
                        .release_directory(request.install_root())
                }
            };
            let bin = directory.join(BIN_DIRECTORY);
            let executable = directory.join(role.relative_path());
            if directory::text(&executable)?.len() > MAX_INSTALL_PATH_BYTES {
                return Err(invalid());
            }
            owner
                .verify(&raw, directory::text(&directory)?, self.operation_timeout)
                .await?;
            owner
                .verify(&raw, directory::text(&bin)?, self.operation_timeout)
                .await?;
            bundle_file::verify(
                &raw,
                directory::text(&executable)?,
                request.payload(role),
                EXECUTABLE_MODE,
                uid,
                self.operation_timeout,
            )
            .await?;
            owner
                .verify(
                    &raw,
                    directory::text(request.install_root())?,
                    self.operation_timeout,
                )
                .await?;
            owner
                .verify(&raw, directory::text(&directory)?, self.operation_timeout)
                .await?;
            owner
                .verify(&raw, directory::text(&bin)?, self.operation_timeout)
                .await
        }
        .await;
        settle(result, raw.close(SCOPE).await)?;
        Ok(ChannelOutcome {
            identity,
            value: (),
        })
    }
}
