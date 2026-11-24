use super::super::constants::{
    FILE_PERMISSION_MASK, RUNTIME_DIRECTORY_MODE, RUNTIME_LINK_PREFIX, RUNTIME_RELEASE_PREFIX,
    RUNTIME_UPLOAD_PREFIX,
};
use super::deployment_operation::{optional, sftp as sftp_operation};
use super::*;
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::FileAttributes;

use RemoteFilesystemOperation as Operation;

pub(super) async fn ensure_install_directory(
    sftp: &SftpSession,
    parent: &str,
    limit: std::time::Duration,
) -> Result<(), RemoteError> {
    match optional(
        limit,
        Operation::InspectMetadata,
        parent,
        sftp.metadata(parent),
    )
    .await?
    {
        Some(metadata) if safe_directory(&metadata) => return Ok(()),
        Some(_) => {
            return Err(RemoteError::RuntimeIntegrity(
                "deployment parent is not a private directory".into(),
            ));
        }
        None => {}
    }
    if let Err(error) = sftp_operation(
        limit,
        Operation::CreateDirectory,
        parent,
        sftp.create_dir(parent),
    )
    .await
    {
        match sftp_operation(
            limit,
            Operation::InspectMetadata,
            parent,
            sftp.metadata(parent),
        )
        .await
        {
            Ok(metadata) if safe_directory(&metadata) => return Ok(()),
            _ => return Err(error),
        }
    }
    sftp_operation(
        limit,
        Operation::SetPermissions,
        parent,
        sftp.set_metadata(
            parent,
            FileAttributes {
                permissions: Some(RUNTIME_DIRECTORY_MODE),
                ..FileAttributes::empty()
            },
        ),
    )
    .await?;
    let metadata = sftp_operation(
        limit,
        Operation::InspectMetadata,
        parent,
        sftp.metadata(parent),
    )
    .await?;
    if !safe_directory(&metadata) {
        return Err(RemoteError::RuntimeIntegrity(
            "created deployment directory has unsafe permissions".into(),
        ));
    }
    Ok(())
}

pub(super) async fn stable_target(
    sftp: &SftpSession,
    stable: &str,
    limit: std::time::Duration,
) -> Result<Option<String>, RemoteError> {
    let metadata = match optional(
        limit,
        Operation::InspectLink,
        stable,
        sftp.symlink_metadata(stable),
    )
    .await?
    {
        Some(metadata) => metadata,
        None => return Ok(None),
    };
    if !metadata.is_symlink() {
        return Err(RemoteError::RuntimeIntegrity(
            "stable runtime path is not a symbolic link".into(),
        ));
    }
    let target = sftp_operation(limit, Operation::ReadLink, stable, sftp.read_link(stable)).await?;
    let stable = Path::new(stable);
    let target_path = Path::new(&target);
    let managed = super::super::file::scoped_absolute(target_path)
        && target_path.parent() == stable.parent()
        && target_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(RUNTIME_RELEASE_PREFIX));
    if managed {
        Ok(Some(target))
    } else {
        Err(RemoteError::RuntimeIntegrity(
            "stable runtime link points outside managed releases".into(),
        ))
    }
}

pub(super) async fn exists(
    sftp: &SftpSession,
    path: &str,
    limit: std::time::Duration,
) -> Result<bool, RemoteError> {
    sftp_operation(
        limit,
        Operation::CheckExistence,
        path,
        sftp.try_exists(path),
    )
    .await
}

pub(super) async fn cleanup_temporary(
    sftp: &SftpSession,
    path: &str,
    limit: std::time::Duration,
) -> Result<(), RemoteError> {
    let Some(name) = Path::new(path).file_name().and_then(|value| value.to_str()) else {
        return Ok(());
    };
    if name.starts_with(RUNTIME_UPLOAD_PREFIX) || name.starts_with(RUNTIME_LINK_PREFIX) {
        optional(limit, Operation::Remove, path, sftp.remove_file(path))
            .await
            .map(|_| ())?;
    }
    Ok(())
}

fn safe_directory(metadata: &FileAttributes) -> bool {
    metadata.is_dir()
        && metadata.permissions.unwrap_or_default() & FILE_PERMISSION_MASK == RUNTIME_DIRECTORY_MODE
}
