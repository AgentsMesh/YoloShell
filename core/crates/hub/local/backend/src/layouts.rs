use crate::LocalBackend;
use std::sync::Arc;
use tokio::sync::Mutex;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;
use yolo_paths::directory::PrivateDirectory;
use yolo_presentation_protocol::{LayoutAddress, LayoutSummary, LayoutUpdate, ProtocolError};
use yolo_presentation_store::{LayoutStore, StoreError};

const LAYOUT_DIRECTORY: &str = "presentation";

impl LocalBackend {
    pub fn with_layouts(mut self, root: &PrivateDirectory) -> Result<Self, PublicError> {
        if self.layouts.is_none() {
            let directory = root
                .ensure_descendant(&root.path().join(LAYOUT_DIRECTORY))
                .map_err(|_| PublicError::Internal)?;
            let store = LayoutStore::open(directory, self.hub).map_err(storage_error)?;
            self.layouts = Some(Arc::new(Mutex::new(store)));
        }
        Ok(self)
    }
    pub(crate) async fn list_layouts(&self) -> Result<Reply, PublicError> {
        let store = self
            .layouts
            .as_ref()
            .ok_or(PublicError::Unsupported)?
            .lock()
            .await;
        Ok(Reply::Layouts {
            documents: store
                .list()
                .map_err(storage_error)?
                .iter()
                .map(LayoutSummary::try_from)
                .collect::<Result<_, _>>()
                .map_err(|_| PublicError::Internal)?,
        })
    }
    pub(crate) async fn get_layout(&self, address: LayoutAddress) -> Result<Reply, PublicError> {
        let store = self
            .layouts
            .as_ref()
            .ok_or(PublicError::Unsupported)?
            .lock()
            .await;
        Ok(Reply::Layout {
            document: store.get(address).map_err(storage_error)?,
        })
    }
    pub(crate) async fn update_layout(&self, change: LayoutUpdate) -> Result<Reply, PublicError> {
        let mut store = self
            .layouts
            .as_ref()
            .ok_or(PublicError::Unsupported)?
            .lock()
            .await;
        Ok(Reply::Layout {
            document: Some(store.update(change).map_err(storage_error)?),
        })
    }
}
fn storage_error(error: StoreError) -> PublicError {
    match error {
        StoreError::Protocol(ProtocolError::RevisionConflict) => PublicError::Conflict,
        StoreError::Protocol(ProtocolError::AddressMismatch) => PublicError::OriginMismatch,
        StoreError::Protocol(_) => PublicError::InvalidRequest,
        StoreError::Storage(_) => PublicError::Internal,
    }
}
