use crate::local_unix::{LocalUnixStream, rights};
use crate::protocol::{RequestEnvelope, read_frame, to_value, write_frame};
use anyhow::{Context, Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::os::fd::{BorrowedFd, OwnedFd};
use std::path::Path;
use uuid::Uuid;

const PEER_UID_CONTEXT: &str = "read PTY custody peer credentials";
const PEER_UID_MISMATCH_DETAIL: &str = "PTY custody peer belongs to another user";
const PEER_PID_CONTEXT: &str = "PTY custody peer process identity is unavailable";
const OPENING_ENVELOPE_DETAIL: &str = "invalid PTY custody opening envelope";
#[cfg(any(test, feature = "yolo-test-support"))]
const TRUNCATED_TEST_FRAME_DETAIL: &str = "PTY custody test frame has no payload";

pub struct CustodyUnixStream {
    inner: LocalUnixStream,
}

impl CustodyUnixStream {
    pub async fn connect(path: &Path) -> Result<Self> {
        let stream = LocalUnixStream::connect(path).await?;
        Self::admit(stream)
    }

    pub fn from_accepted(stream: tokio::net::UnixStream) -> Result<Self> {
        Self::admit(LocalUnixStream::from(stream))
    }

    pub fn from_local(stream: LocalUnixStream) -> Result<Self> {
        Self::admit(stream)
    }

    pub async fn write_opening(&mut self, request: &super::CustodyRequest) -> Result<()> {
        self.write_method_opening(super::METHOD, request).await
    }

    pub async fn write_method_opening<T: Serialize>(
        &mut self,
        method: &str,
        payload: &T,
    ) -> Result<()> {
        let opening = RequestEnvelope::new(Uuid::new_v4().to_string(), method, to_value(payload)?);
        self.write(&opening).await
    }

    pub fn decode_opening(opening: &RequestEnvelope) -> Result<super::CustodyRequest> {
        ensure!(
            opening.version == crate::model::PROTOCOL_VERSION
                && opening.kind == crate::protocol::REQUEST_KIND
                && opening.method == super::METHOD,
            OPENING_ENVELOPE_DETAIL
        );
        opening.decode_payload().map_err(Into::into)
    }

    pub async fn read<T: DeserializeOwned>(&mut self) -> Result<T> {
        read_frame(&mut self.inner).await.map_err(Into::into)
    }

    pub async fn write<T: Serialize>(&mut self, value: &T) -> Result<()> {
        write_frame(&mut self.inner, value)
            .await
            .map_err(Into::into)
    }

    #[cfg(any(test, feature = "yolo-test-support"))]
    pub(crate) async fn write_truncated_frame<T: Serialize>(&mut self, value: &T) -> Result<()> {
        let frame = crate::protocol::encode(value)?;
        let (_, prefix) = frame.split_last().context(TRUNCATED_TEST_FRAME_DETAIL)?;
        tokio::io::AsyncWriteExt::write_all(&mut self.inner, prefix)
            .await
            .map_err(Into::into)
    }

    pub async fn send_rights(&self, descriptors: &[BorrowedFd<'_>]) -> Result<()> {
        rights::send_exact(&self.inner, super::constants::RIGHTS_MARKER, descriptors)
            .await
            .map_err(Into::into)
    }

    pub async fn receive_rights(&self, count: usize) -> Result<Vec<OwnedFd>> {
        rights::receive_exact(&self.inner, super::constants::RIGHTS_MARKER, count)
            .await
            .map_err(Into::into)
    }

    pub fn peer_pid(&self) -> Result<u32> {
        let credentials = self.inner.peer_cred().context(PEER_UID_CONTEXT)?;
        let pid = credentials.pid().context(PEER_PID_CONTEXT)?;
        u32::try_from(pid).context(PEER_PID_CONTEXT)
    }

    fn admit(inner: LocalUnixStream) -> Result<Self> {
        let credentials = inner.peer_cred().context(PEER_UID_CONTEXT)?;
        ensure!(
            credentials.uid() == unsafe { libc::geteuid() },
            PEER_UID_MISMATCH_DETAIL
        );
        Ok(Self { inner })
    }
}
