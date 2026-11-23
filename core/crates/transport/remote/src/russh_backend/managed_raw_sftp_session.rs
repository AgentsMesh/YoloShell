use super::RemoteError;
use super::channel_transfer_custody::TransferredChannelCustody;
use super::deployment_operation::{close_raw_session, settle};
use russh_sftp::client::RawSftpSession;
use std::ops::Deref;

pub(super) struct ManagedRawSftpSession {
    inner: Option<RawSftpSession>,
    custody: Option<TransferredChannelCustody>,
}

impl ManagedRawSftpSession {
    pub(super) fn new(inner: RawSftpSession, custody: TransferredChannelCustody) -> Self {
        Self {
            inner: Some(inner),
            custody: Some(custody),
        }
    }

    pub(super) fn fail<T>(mut self, primary: RemoteError) -> Result<T, RemoteError> {
        self.inner.take();
        self.custody.take().unwrap().fail(primary)
    }

    pub(super) async fn close(mut self, scope: &str) -> Result<(), RemoteError> {
        let requested = close_raw_session(self.inner.as_ref().unwrap(), scope);
        self.inner.take();
        let settled = self.custody.take().unwrap().close().await;
        settle(requested, settled)
    }
}

impl Deref for ManagedRawSftpSession {
    type Target = RawSftpSession;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl Drop for ManagedRawSftpSession {
    fn drop(&mut self) {
        self.inner.take();
        self.custody.take();
    }
}
