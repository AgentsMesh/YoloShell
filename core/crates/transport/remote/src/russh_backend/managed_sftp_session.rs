use super::RemoteError;
use super::channel_transfer_custody::TransferredChannelCustody;
use super::deadline::operation;
use super::deployment_operation::settle;
use russh_sftp::client::SftpSession;
use std::ops::Deref;
use std::time::Duration;

pub(super) struct ManagedSftpSession {
    inner: Option<SftpSession>,
    custody: Option<TransferredChannelCustody>,
    close_timeout: Duration,
}

impl ManagedSftpSession {
    pub(super) fn new(
        inner: SftpSession,
        custody: TransferredChannelCustody,
        close_timeout: Duration,
    ) -> Self {
        Self {
            inner: Some(inner),
            custody: Some(custody),
            close_timeout,
        }
    }

    pub(super) async fn close(mut self) -> Result<(), RemoteError> {
        let requested = operation(self.close_timeout, self.inner.as_ref().unwrap().close()).await;
        self.inner.take();
        let settled = self.custody.take().unwrap().close().await;
        settle(requested, settled)
    }
}

impl Deref for ManagedSftpSession {
    type Target = SftpSession;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl Drop for ManagedSftpSession {
    fn drop(&mut self) {
        self.inner.take();
        self.custody.take();
    }
}
