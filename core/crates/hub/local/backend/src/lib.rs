pub(crate) mod protocol {
    pub use yolo_protocol_validation::*;
    pub use yolo_wire::*;
}

mod agent_control;
mod agents;
mod attachment;
mod audit;
mod attachment_event;
mod backend;
mod build;
mod connector_admission;
mod connector_catalogue;
mod connector_observation_gate;
mod connector_primary;
mod connectors;
mod conversations;
mod creation_cleanup;
mod creation_query;
mod current_agent;
mod dispatch;
mod errors;
mod files;
mod host_hold;
mod inventory;
mod layouts;
mod lifecycle;
mod locked_observation;
mod native_control;
mod native_control_effect;
mod native_control_receipt;
mod native_diagnostics;
mod native_observation;
mod observation;
mod observation_apply;
mod platform_catalogue;
mod summary;
mod writer;
mod writer_conversion;
mod writer_observation;

#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;

pub use agent_control::PromptExecution;
pub use attachment::Attachment;
pub use backend::LocalBackend;
