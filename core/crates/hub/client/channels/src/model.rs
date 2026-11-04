use crate::ControlError;
use std::path::PathBuf;
use uuid::Uuid;
use yolo_hub_catalog_client::{Catalog, CatalogError};
use yolo_hub_client::HubEndpoint;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Hello;
use yolo_hub_rpc_protocol::{Command, Reply};

#[derive(Clone, Debug)]
pub struct Endpoint {
    pub entry: HubEndpoint,
    pub hub: HubId,
}

impl Endpoint {
    pub fn local(socket: impl Into<PathBuf>, hub: HubId) -> Self {
        Self {
            entry: HubEndpoint::local(socket),
            hub,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ControlRequest {
    pub validity: Option<crate::RequestValidity>,
    pub generation: Option<Uuid>,
    pub destination: HubId,
    pub command: Command,
}

#[derive(Debug)]
pub enum ControlEvent {
    Ready {
        generation: Uuid,
        hello: Hello,
    },
    Unreachable {
        generation: Option<Uuid>,
        message: String,
    },
    Finished {
        request: ControlRequest,
        result: Result<Reply, ControlError>,
        /// Current fast control channel health, independent of the operation stream.
        connected: bool,
    },
}

#[derive(Debug)]
pub struct CatalogEvent {
    pub request_id: Uuid,
    pub result: Result<Catalog, CatalogError>,
}
