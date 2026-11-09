use super::super::constants::{
    MAX_REMOTE_PATH_BYTES, POSIX_RENAME_EXTENSION, POSIX_RENAME_SESSION_SCOPE,
    POSIX_RENAME_UNEXPECTED_PACKET_MESSAGE, POSIX_RENAME_VERSION, SFTP_PATH_LENGTH_BYTES,
    SFTP_SUBSYSTEM,
};
use super::deadline::{operation, remote_operation};
use super::deployment_operation::{
    protocol, settle, sftp as sftp_operation, status as status_error,
};
use super::managed_raw_sftp_session::ManagedRawSftpSession;
use super::*;
use russh_sftp::client::RawSftpSession;
use russh_sftp::protocol::{Packet, StatusCode};

use RemoteFilesystemOperation as Operation;

impl RusshConnection {
    pub(super) async fn posix_rename(&self, old: &str, new: &str) -> Result<(), RemoteError> {
        let encoded_paths = path_pair(old, new)?;
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
        if version
            .extensions
            .get(POSIX_RENAME_EXTENSION)
            .map(String::as_str)
            != Some(POSIX_RENAME_VERSION)
        {
            let primary = Err(RemoteError::RuntimeDeployment(
                "server lacks atomic POSIX rename support".into(),
            ));
            let closed = raw.close(POSIX_RENAME_SESSION_SCOPE).await;
            return settle(primary, closed);
        }
        let result = sftp_operation(
            self.operation_timeout,
            Operation::AtomicRename,
            new,
            raw.extended(POSIX_RENAME_EXTENSION, encoded_paths),
        )
        .await;
        let result = match result {
            Ok(Packet::Status(status)) if status.status_code == StatusCode::Ok => Ok(()),
            Ok(Packet::Status(status)) => Err(status_error(Operation::AtomicRename, new, status)),
            Ok(_) => Err(protocol(
                Operation::AtomicRename,
                new,
                POSIX_RENAME_UNEXPECTED_PACKET_MESSAGE,
            )),
            Err(error) => Err(error),
        };
        let closed = raw.close(POSIX_RENAME_SESSION_SCOPE).await;
        settle(result, closed)
    }
}

fn path_pair(old: &str, new: &str) -> Result<Vec<u8>, RemoteError> {
    let mut encoded = Vec::new();
    encode_path(&mut encoded, old)?;
    encode_path(&mut encoded, new)?;
    Ok(encoded)
}

fn encode_path(target: &mut Vec<u8>, value: &str) -> Result<(), RemoteError> {
    if value.len() > MAX_REMOTE_PATH_BYTES {
        return Err(RemoteError::InvalidRuntime(
            "deployment path is too long".into(),
        ));
    }
    let length = value.len() as u32;
    target.reserve(SFTP_PATH_LENGTH_BYTES + value.len());
    target.extend_from_slice(&length.to_be_bytes());
    target.extend_from_slice(value.as_bytes());
    Ok(())
}
