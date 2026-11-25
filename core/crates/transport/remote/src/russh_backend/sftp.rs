use super::super::constants::SFTP_SUBSYSTEM;
use super::channel_transfer_custody::TransferredChannelCustody;
use super::deadline::{operation, remote_operation};
use super::managed_sftp_session::ManagedSftpSession;
use super::*;
use crate::remote::RemoteCleanupReservation;
use crate::remote::constants::OPERATION_TIMEOUT_MESSAGE;
use russh_sftp::client::SftpSession;
use std::path::Path;
use tokio::io::DuplexStream;
use tokio::task::JoinHandle;

impl RusshConnection {
    pub(super) async fn canonicalize_sftp(
        &self,
        path: &Path,
    ) -> Result<ChannelOutcome<PathBuf>, RemoteError> {
        let path = path_text(path)?;
        let (identity, sftp) = self.open_sftp(ChannelPurpose::FileRead).await?;
        let result = operation(self.operation_timeout, sftp.canonicalize(path)).await;
        let closed = sftp.close().await;
        let resolved = super::deployment_operation::settle(result, closed)?;
        Ok(ChannelOutcome {
            identity,
            value: PathBuf::from(resolved),
        })
    }

    pub(super) async fn read_sftp(
        &self,
        read: &AuthorizedFileRead,
    ) -> Result<ChannelOutcome<FileReadResult>, RemoteError> {
        path_text(read.resolved_path())?;
        let (identity, sftp) = self.open_sftp(ChannelPurpose::FileRead).await?;
        let result =
            super::sftp_read::read_file_contents(&sftp, read, self.operation_timeout).await;
        let closed = sftp.close().await;
        let value = super::deployment_operation::settle(result, closed)?;
        Ok(ChannelOutcome { identity, value })
    }

    pub(super) async fn open_sftp(
        &self,
        purpose: ChannelPurpose,
    ) -> Result<(ChannelIdentity, ManagedSftpSession), RemoteError> {
        let mut channel = self.open_session_channel().await?;
        let approved = remote_operation(
            self.operation_timeout,
            super::channel_request::subsystem(channel.channel(), SFTP_SUBSYSTEM),
        )
        .await?;
        let (session, channel, reservation) = channel.into_parts();
        let (stream, custody) = TransferredChannelCustody::start(
            session,
            channel,
            approved,
            reservation,
            self.operation_timeout,
        );
        let session = initialize_sftp(stream, custody, self.operation_timeout).await?;
        Ok((self.identity(purpose), session))
    }
}

pub(super) async fn open_sftp_session_for_cleanup(
    session: Arc<client::Handle<HostKeyHandler>>,
    limit: Duration,
    reservation: RemoteCleanupReservation,
) -> Result<ManagedSftpSession, RemoteError> {
    let mut channel = session.channel_open_session().await.map_err(ssh_error)?;
    let approved = remote_operation(
        limit,
        super::channel_request::subsystem(&mut channel, SFTP_SUBSYSTEM),
    )
    .await;
    let approved = match approved {
        Ok(value) => value,
        Err(error) => {
            let closed = super::channel_open_cleanup::close_session_channel(
                Arc::clone(&session),
                channel,
                limit,
            )
            .await;
            return super::deployment_operation::settle(Err(error), closed);
        }
    };
    let (stream, custody) =
        TransferredChannelCustody::start(session, channel, approved, reservation, limit);
    initialize_sftp(stream, custody, limit).await
}

async fn initialize_sftp(
    stream: DuplexStream,
    custody: TransferredChannelCustody,
    limit: Duration,
) -> Result<ManagedSftpSession, RemoteError> {
    SftpInitializationCustody::new(stream, custody)
        .wait(limit)
        .await
}

struct SftpInitializationCustody {
    task: Option<JoinHandle<Result<SftpSession, RemoteError>>>,
    channel: Option<TransferredChannelCustody>,
}

impl SftpInitializationCustody {
    fn new(stream: DuplexStream, channel: TransferredChannelCustody) -> Self {
        let task = tokio::spawn(async move { SftpSession::new(stream).await.map_err(ssh_error) });
        Self {
            task: Some(task),
            channel: Some(channel),
        }
    }

    async fn wait(mut self, limit: Duration) -> Result<ManagedSftpSession, RemoteError> {
        match tokio::time::timeout(limit, self.task.as_mut().unwrap()).await {
            Ok(Ok(Ok(session))) => {
                self.task.take();
                Ok(ManagedSftpSession::new(
                    session,
                    self.channel.take().unwrap(),
                    limit,
                ))
            }
            Ok(Ok(Err(primary))) => {
                self.task.take();
                self.channel.take().unwrap().fail(primary)
            }
            Ok(Err(error)) => {
                self.task.take();
                self.channel.take().unwrap().fail(ssh_error(error))
            }
            Err(_) => self.channel.take().unwrap().fail_task(
                RemoteError::transport(OPERATION_TIMEOUT_MESSAGE),
                self.task.take().unwrap(),
            ),
        }
    }
}

impl Drop for SftpInitializationCustody {
    fn drop(&mut self) {
        if let (Some(channel), Some(task)) = (self.channel.take(), self.task.take()) {
            channel.abandon_task(task);
        }
    }
}

pub(super) fn path_text(path: &Path) -> Result<&str, RemoteError> {
    path.to_str()
        .ok_or_else(|| RemoteError::PathDenied(path.to_path_buf()))
}
