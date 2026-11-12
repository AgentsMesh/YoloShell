use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_agent_contract::AgentBinding;

/// One binding or status change published by a Worker. Carries no terminal
/// output: an observer of agent state never receives screen or journal bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentEvent {
    pub event_type: String,
    pub binding: AgentBinding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchAgentEventsRequest {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
}

/// A Host catalogue change: `terminal_id` is absent when the whole catalogue
/// may have changed and the subscriber must resynchronise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogueChange {
    pub revision: u64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "yolo_crypto::serde_uuid::option"
    )]
    pub terminal_id: Option<Uuid>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchCatalogueRequest {
    #[serde(default)]
    pub after_revision: u64,
}

/// The revision from which the event stream continues. A subscriber whose
/// previous revision differs performs one full catalogue read first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchCatalogueStart {
    pub revision: u64,
}
