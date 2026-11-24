use super::*;
use crate::host::{CatalogueChange, WatchCatalogueRequest, WatchCatalogueStart};
use crate::protocol::{EventEnvelope, ProtocolError, event, method, validate_event};

/// A Host catalogue change stream. `start_revision` is the Host's revision at
/// subscription time: a caller that previously saw a different revision must
/// read the catalogue once before trusting later hints.
pub struct CatalogueWatch {
    stream: Option<yolo_runtime_rpc::RuntimeConnection>,
    pub start_revision: u64,
}

impl Client {
    pub async fn watch_catalogue(
        &self,
        after_revision: u64,
    ) -> Result<CatalogueWatch, ClientError> {
        let deadline = self.operation_deadline();
        let mut connection = self.connect_until(deadline).await?;
        let response = connection
            .raw_until(
                method::WATCH_CATALOGUE,
                &WatchCatalogueRequest { after_revision },
                deadline,
            )
            .await?;
        let start: WatchCatalogueStart = decode_response(response)?;
        Ok(CatalogueWatch {
            stream: Some(connection.into_channel()?),
            start_revision: start.revision,
        })
    }
}

impl CatalogueWatch {
    pub async fn next_change(&mut self) -> Result<CatalogueChange, ClientError> {
        let Some(mut stream) = self.stream.take() else {
            return Err(ProtocolError::ConnectionPoisoned.into());
        };
        let envelope = stream.wait_event().await?;
        let change = decode_change(&envelope)?;
        self.stream = Some(stream);
        Ok(change)
    }

    pub fn close(mut self) {
        if let Some(mut stream) = self.stream.take() {
            stream.close();
        }
    }
}

fn decode_change(envelope: &EventEnvelope) -> Result<CatalogueChange, ClientError> {
    validate_event(envelope)?;
    if envelope.event != event::CATALOGUE_CHANGED {
        return Err(ProtocolError::EventMismatch.into());
    }
    yolo_wire::from_value(envelope.payload.clone()).map_err(Into::into)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/catalogue_watch_tests.rs"]
mod catalogue_watch_tests;
