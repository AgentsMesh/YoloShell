use super::super::constants::SFTP_LIMITS_EXTENSION_VERSION;
use super::super::memory_fs::{FailurePoint, MemoryFilesystem};
use russh_sftp::protocol::Version;
use russh_sftp::server::StatusReply;

pub(super) async fn initialize(
    filesystem: &MemoryFilesystem,
) -> Result<(Version, bool), StatusReply> {
    super::failures::initialize(filesystem).await?;
    let reject_limits = filesystem.take_failure(FailurePoint::SftpLimitsRejected);
    let mut version = super::super::sftp_wire::server_version(
        filesystem.supports_fsync(),
        filesystem.supports_posix_rename(),
    );
    if reject_limits {
        version.extensions.insert(
            russh_sftp::extensions::LIMITS.into(),
            SFTP_LIMITS_EXTENSION_VERSION.into(),
        );
    }
    Ok((version, reject_limits))
}
