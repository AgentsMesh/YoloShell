use super::super::constants::{
    FSYNC_EXTENSION, FSYNC_EXTENSION_VERSION, SFTP_CAPABILITY_SESSION_SCOPE, SFTP_SUBSYSTEM,
};
use super::deadline::{operation, remote_operation};
use super::deployment_operation::settle;
use super::managed_raw_sftp_session::ManagedRawSftpSession;
use super::*;
use russh_sftp::client::RawSftpSession;

impl RusshConnection {
    pub(super) async fn require_file_fsync(&self) -> Result<(), RemoteError> {
        let mut channel = self.open_session_channel().await?;
        let approved = remote_operation(
            self.operation_timeout,
            super::channel_request::subsystem(channel.channel(), SFTP_SUBSYSTEM),
        )
        .await;
        let approved = match approved {
            Ok(value) => value,
            Err(primary) => {
                return Err(primary);
            }
        };
        let (session, channel, reservation) = channel.into_parts();
        let (stream, custody) = super::channel_transfer_custody::TransferredChannelCustody::start(
            session,
            channel,
            approved,
            reservation,
            self.operation_timeout,
        );
        let raw = ManagedRawSftpSession::new(RawSftpSession::new(stream), custody);
        let initialized = operation(self.operation_timeout, raw.init()).await;
        let version = match initialized {
            Ok(version) => version,
            Err(primary) => {
                return raw.fail(primary);
            }
        };
        let supported = version
            .extensions
            .get(FSYNC_EXTENSION)
            .is_some_and(|version| version == FSYNC_EXTENSION_VERSION);
        let result = if supported {
            Ok(())
        } else {
            Err(RemoteError::RuntimeDeployment(
                super::super::constants::FSYNC_REQUIRED_MESSAGE.into(),
            ))
        };
        let closed = raw.close(SFTP_CAPABILITY_SESSION_SCOPE).await;
        settle(result, closed)
    }
}
