pub use crate::channel_network::{network, network_with_priority};
use uuid::Uuid;
use yolo_hub_client_channels::{CatalogEvent, ControlEvent, ControlRequest};
use yolo_hub_identity::{HubId, RuntimeTarget};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseRequest, TerminalEvent};

pub enum Job {
    Chat(crate::chat::network::Operation),
    Layout(crate::layout_sync::Operation),
    Scoped(ControlRequest),
    Acquire {
        generation: Uuid,
        origin: HubId,
        request: LeaseRequest,
        validity: yolo_hub_client_channels::RequestValidity,
    },
    UserInput(ControlRequest),
    CreationQuery(ControlRequest),
    Finalize {
        generation: Uuid,
        creation_token: Uuid,
        creation: yolo_hub_terminal_protocol::CreateShellResult,
        validity: yolo_hub_client_channels::RequestValidity,
    },
    TerminalReply {
        generation: Uuid,
        request: yolo_hub_terminal_protocol::InputRequest,
        validity: yolo_hub_client_channels::RequestValidity,
    },
    Management(yolo_tui_hub_manager::Operation),
    Refresh {
        request_id: Uuid,
    },
    Call {
        generation: Option<Uuid>,
        origin: HubId,
        command: Command,
    },
    InitialAcquire {
        generation: Uuid,
        origin: HubId,
        request: LeaseRequest,
        validity: yolo_hub_client_channels::RequestValidity,
    },
}

pub enum Update {
    Chat {
        operation: crate::chat::network::Operation,
        result: Result<crate::chat::network::Reading, yolo_tui_chat::QueryError>,
    },
    Layout {
        operation: crate::layout_sync::Operation,
        result: Result<Reply, String>,
    },
    Management {
        operation: yolo_tui_hub_manager::Operation,
        result: Result<
            yolo_hub_management_protocol::ConfigurationSnapshot,
            yolo_hub_management_client::ManagementError,
        >,
    },
    ManagementNotSent {
        token: uuid::Uuid,
    },
    Observation {
        target: RuntimeTarget,
        generation: uuid::Uuid,
        update: ObservationUpdate,
    },
    Catalog(CatalogEvent),
    PlatformEvents(yolo_hub_client_channels::PlatformEventWake),
    Control(ControlEvent),
    NotSent {
        request: ControlRequest,
    },
    Reply {
        command: Command,
        reply: Reply,
    },
    Failed {
        command: Option<Command>,
        message: String,
        connected: bool,
        transient: bool,
    },
}

pub enum ObservationUpdate {
    Attached {
        request: yolo_hub_terminal_protocol::AttachRequest,
        attachment: yolo_hub_terminal_protocol::AttachResult,
    },
    Terminal {
        event: TerminalEvent,
    },
    Closed {
        message: Option<String>,
    },
}
