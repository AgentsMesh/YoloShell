use crate::{ClientError, HubEntryStream};
use std::{future::Future, pin::Pin};
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_identity::HubId;

/// An authenticated remote entry adapter supplied by the client executable.
/// Authentication and connection reuse stay outside public RPC and rendering.
/// Opening a stream cannot send or retry a resource mutation.
pub trait EntryConnector: Send + Sync + std::fmt::Debug {
    fn validate(&self) -> Result<(), ClientError>;
    fn client_id(&self) -> Uuid;
    fn open(
        &self,
        expected: HubId,
        deadline: Instant,
    ) -> Pin<Box<dyn Future<Output = Result<HubEntryStream, ClientError>> + Send + '_>>;
}
