use super::{
    RusshConnection,
    channel_transfer_custody::TransferredChannelCustody,
    deadline::{operation, remote_operation},
    managed_raw_sftp_session::ManagedRawSftpSession,
};
use crate::remote::{
    RemoteError,
    constants::{FSYNC_EXTENSION, FSYNC_EXTENSION_VERSION, FSYNC_REQUIRED_MESSAGE, SFTP_SUBSYSTEM},
};
use russh_sftp::client::RawSftpSession;

pub(super) const SCOPE: &str = "runtime bundle staging";
pub(super) const CANCELLED: &str = "runtime bundle staging was cancelled";
pub(super) const INVALID_METADATA: &str = "runtime bundle path or metadata changed";
pub(super) const READBACK_MISMATCH: &str =
    "runtime bundle readback differs from its authorized bytes";
pub(super) const PERMISSION_MASK: u32 = 0o7777;
pub(super) const WRITE_ACCESS_MASK: u32 = 0o022;
pub(super) const STICKY_MODE: u32 = 0o1000;
pub(super) const ROOT_OWNER: u32 = 0;
pub(super) const SENTINEL_BYTES: usize = 1;

impl RusshConnection {
    pub(super) async fn open_bundle_sftp(
        &self,
        require_fsync: bool,
    ) -> Result<ManagedRawSftpSession, RemoteError> {
        let mut channel = self.open_session_channel().await?;
        let approved = remote_operation(
            self.operation_timeout,
            super::channel_request::subsystem(channel.channel(), SFTP_SUBSYSTEM),
        )
        .await?;
        let (session, channel, reservation) = channel.into_parts();
        let (stream, custody) = TransferredChannelCustody::start_bundle(
            session,
            channel,
            approved,
            reservation,
            self.operation_timeout,
        );
        let raw = ManagedRawSftpSession::new(RawSftpSession::new(stream), custody);
        let version = match operation(self.operation_timeout, raw.init()).await {
            Ok(version) => version,
            Err(error) => return raw.fail(error),
        };
        if require_fsync
            && !version
                .extensions
                .get(FSYNC_EXTENSION)
                .is_some_and(|version| version == FSYNC_EXTENSION_VERSION)
        {
            let result = Err(RemoteError::RuntimeDeployment(
                FSYNC_REQUIRED_MESSAGE.into(),
            ));
            return super::deployment_operation::settle(result, raw.close(SCOPE).await);
        }
        Ok(raw)
    }
}

pub(super) fn invalid() -> RemoteError {
    RemoteError::RuntimeIntegrity(INVALID_METADATA.into())
}
