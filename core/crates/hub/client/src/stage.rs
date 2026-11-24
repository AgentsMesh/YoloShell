use crate::{Client, ClientError};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::time::{Duration, Instant, timeout};
use uuid::Uuid;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_jsonrpc_transport::TransportError;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{ATTACHMENT_CHUNK_BYTES, PublicError, StageAttachmentRequest};

impl<S: AsyncRead + AsyncWrite + Unpin> Client<S> {
    /// Stream one local file to the machine running this exact terminal. A successful
    /// EOF returns an absolute path that can be inserted in a prompt or terminal paste.
    /// The timeout renews for each read and RPC as the transfer progresses.
    pub async fn stage_attachment<R: AsyncRead + Unpin>(
        &mut self,
        target: RuntimeTarget,
        name: &str,
        source: &mut R,
        idle_timeout: Duration,
    ) -> Result<String, ClientError> {
        let upload_id = Uuid::new_v4();
        let mut offset = 0;
        let mut buffer = vec![0; ATTACHMENT_CHUNK_BYTES];
        loop {
            let count = timeout(idle_timeout, source.read(&mut buffer))
                .await
                .map_err(|_| ClientError::Transport(TransportError::Deadline))?
                .map_err(ClientError::AttachmentRead)?;
            let request = StageAttachmentRequest {
                target,
                upload_id,
                name: name.to_owned(),
                offset,
                data: buffer[..count].to_vec(),
                eof: count == 0,
            };
            let reply = self
                .call(
                    target.address.hub_id,
                    Command::StageAttachment { request },
                    Instant::now() + idle_timeout,
                )
                .await?;
            let Reply::StagedAttachment { attachment } = reply else {
                return Err(ClientError::InvalidResponse(PublicError::InvalidPayload));
            };
            if let Some(path) = attachment.path {
                return Ok(path);
            }
            offset = attachment.next_offset;
        }
    }
}
