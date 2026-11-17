use super::deployment_fs::{cleanup_temporary, stable_target};
use super::deployment_operation::sftp as sftp_operation;
use super::deployment_verify::verify_file;
use super::*;
use russh_sftp::client::SftpSession;
use std::time::Duration;

use RemoteFilesystemOperation as Operation;

pub(super) async fn activate(
    sftp: &SftpSession,
    connection: &RusshConnection,
    request: &RuntimeActivationRequest,
    limit: Duration,
) -> Result<(), RemoteError> {
    let release = &request.release_path;
    verify_file(
        sftp,
        release,
        request.expected_bytes,
        &request.sha256,
        limit,
    )
    .await?;
    let stable = &request.stable_path;
    let previous = stable_target(sftp, stable, limit).await?;
    let link = &request.temporary_link;
    sftp_operation(
        limit,
        Operation::CreateLink,
        link,
        sftp.symlink(release, link),
    )
    .await?;
    if sftp_operation(limit, Operation::ReadLink, link, sftp.read_link(link)).await?
        != release.as_str()
    {
        let _ = cleanup_temporary(sftp, link, limit).await;
        return Err(RemoteError::RuntimeIntegrity(
            "temporary runtime link has the wrong target".into(),
        ));
    }
    let renamed = if previous.is_some() {
        connection.posix_rename(link, stable).await
    } else {
        sftp_operation(limit, Operation::Rename, stable, sftp.rename(link, stable)).await
    };
    if let Err(error) = renamed {
        let selected = stable_target(sftp, stable, limit).await;
        let _ = cleanup_temporary(sftp, link, limit).await;
        if selected
            .as_ref()
            .is_ok_and(|target| target.as_deref() == Some(release))
        {
            return Ok(());
        }
        return Err(error);
    }
    if stable_target(sftp, stable, limit).await?.as_deref() != Some(release) {
        return Err(RemoteError::RuntimeIntegrity(
            "atomic runtime link did not select the release".into(),
        ));
    }
    Ok(())
}
