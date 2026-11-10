//! Versioned local view control. This protocol never controls a shell or agent.
mod constants;
pub use constants::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlDescriptor {
    pub version: u16,
    pub endpoint: Uuid,
    pub token: Uuid,
}

impl ControlDescriptor {
    pub fn new(endpoint: Uuid, token: Uuid) -> Self {
        Self {
            version: CONTROL_PROTOCOL_VERSION,
            endpoint,
            token,
        }
    }
    pub fn is_valid(&self) -> bool {
        self.version == CONTROL_PROTOCOL_VERSION && !self.endpoint.is_nil() && !self.token.is_nil()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlCommand {
    Detach,
    Status,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlRequest {
    pub version: u16,
    pub command: ControlCommand,
    pub token: Uuid,
}

impl ControlRequest {
    pub fn new(token: Uuid) -> Self {
        Self {
            version: CONTROL_PROTOCOL_VERSION,
            command: ControlCommand::Detach,
            token,
        }
    }
    pub fn status(token: Uuid) -> Self {
        Self {
            version: CONTROL_PROTOCOL_VERSION,
            command: ControlCommand::Status,
            token,
        }
    }
    pub fn authorizes(&self, token: Uuid) -> bool {
        self.version == CONTROL_PROTOCOL_VERSION && !self.token.is_nil() && self.token == token
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlResponse {
    pub version: u16,
    pub command: ControlCommand,
    pub accepted: bool,
}

impl ControlResponse {
    pub fn accepted() -> Self {
        Self {
            version: CONTROL_PROTOCOL_VERSION,
            command: ControlCommand::Detach,
            accepted: true,
        }
    }
    pub fn rejected() -> Self {
        Self {
            version: CONTROL_PROTOCOL_VERSION,
            command: ControlCommand::Detach,
            accepted: false,
        }
    }
    pub fn status(accepted: bool) -> Self {
        Self {
            version: CONTROL_PROTOCOL_VERSION,
            command: ControlCommand::Status,
            accepted,
        }
    }
    pub fn confirms_detached(&self) -> bool {
        self.version == CONTROL_PROTOCOL_VERSION
            && self.command == ControlCommand::Detach
            && self.accepted
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
