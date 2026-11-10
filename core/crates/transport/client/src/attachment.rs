use super::deadline::ClientDeadline;
use super::*;
use crate::protocol::{EventEnvelope, ProtocolError, method, validate_attachment_event};

pub struct Attachment {
    stream: Option<yolo_runtime_rpc::RuntimeConnection>,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    pub initial: AttachResult,
}

impl Client {
    pub async fn attach(
        &self,
        terminal_id: Uuid,
        after_seq: u64,
    ) -> Result<Attachment, ClientError> {
        let deadline = self.operation_deadline();
        let mut connection = self.connect_until(deadline).await?;
        let response = connection
            .raw_until(
                method::ATTACH_SESSION,
                &AttachRequest {
                    terminal_id,
                    after_seq,
                },
                deadline,
            )
            .await?;
        let initial: AttachResult = decode_response(response)?;
        initial.validate_for_request(initial.runtime_epoch, after_seq)?;
        Ok(Attachment {
            stream: Some(connection.into_channel()?),
            terminal_id,
            runtime_epoch: initial.runtime_epoch,
            initial,
        })
    }
}

impl Attachment {
    pub async fn next_event(&mut self) -> Result<EventEnvelope, ClientError> {
        let Some(mut stream) = self.stream.take() else {
            return Err(ProtocolError::ConnectionPoisoned.into());
        };
        let event = stream.wait_event().await?;
        validate_attachment_event(&event, self.terminal_id, self.runtime_epoch)?;
        self.stream = Some(stream);
        Ok(event)
    }

    pub async fn detach(mut self) -> Result<(), ClientError> {
        let Some(mut stream) = self.stream.take() else {
            return Err(ProtocolError::ConnectionPoisoned.into());
        };
        stream.close();
        Ok(())
    }

    pub fn runtime_epoch(&self) -> Uuid {
        self.runtime_epoch
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/attachment_epoch_tests.rs"]
mod attachment_epoch_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/attachment_exit_epoch_tests.rs"]
mod attachment_exit_epoch_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/attachment_success_tests.rs"]
mod attachment_success_tests;
