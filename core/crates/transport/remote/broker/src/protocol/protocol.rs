use super::constants::BROKER_PROTOCOL_VERSION;
use crate::remote::{CommandOutput, FileReadResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

include!("protocol_requests.rs");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerRequest {
    ProvisionHub(yolo_remote_provisioning_protocol::BrokerProvisionCall),
    RecoverHubProvision(yolo_remote_provisioning_protocol::BrokerProvisionCall),
    AdvanceHubProvision(yolo_remote_provisioning_protocol::BrokerProvisionCall),
    PrepareHubTransport(yolo_broker_hub_protocol::BrokerHubProfile),
    OpenHubPeer(yolo_broker_hub_protocol::BrokerHubOpenRequest),
    Status,
    Bootstrap,
    WorkingDirectory,
    Command(BrokerCommandRequest),
    FileRead(BrokerFileRequest),
    Shutdown {
        expected_revision: String,
        expected_instance_id: Uuid,
    },
    Retire {
        expected_revision: String,
        expected_instance_id: Uuid,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BrokerRequestEnvelope {
    pub protocol_version: u16,
    pub request_id: Uuid,
    pub request: BrokerRequest,
}

impl BrokerRequestEnvelope {
    pub fn new(request: BrokerRequest) -> Self {
        Self {
            protocol_version: BROKER_PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            request,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerStatus {
    pub protocol_version: u16,
    pub adopted: bool,
    pub instance_id: Uuid,
    pub host_id: String,
    pub connection_id: Option<Uuid>,
    pub connection_generation: u64,
    pub config_revision: String,
    pub capabilities: Vec<String>,
}

impl BrokerStatus {
    pub(super) fn disconnected(
        instance_id: Uuid,
        host_id: String,
        revision: String,
        generation: u64,
    ) -> Self {
        Self {
            protocol_version: BROKER_PROTOCOL_VERSION,
            adopted: true,
            instance_id,
            host_id,
            connection_id: None,
            connection_generation: generation,
            config_revision: revision,
            capabilities: Self::capabilities(),
        }
    }

    pub fn supports(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|value| value == capability)
    }

    pub(super) fn capabilities() -> Vec<String> {
        vec![
            super::constants::CONNECTION_BOOTSTRAP_CAPABILITY.into(),
            super::constants::WORKING_DIRECTORY_CAPABILITY.into(),
            super::retirement::CREDENTIAL_RETIREMENT_CAPABILITY.into(),
        ]
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerResult {
    HubProvision(yolo_remote_provisioning_protocol::ProvisionStatus),
    HubTransport(yolo_broker_hub_protocol::BrokerHubConnectionBinding),
    HubPeer(yolo_broker_hub_protocol::BrokerHubOpened),
    Status(BrokerStatus),
    Bootstrap(BrokerStatus),
    WorkingDirectory {
        connection_id: Uuid,
        path: PathBuf,
    },
    Command {
        connection_id: Uuid,
        output: BrokerCommandOutput,
    },
    FileRead {
        connection_id: Uuid,
        result: BrokerFileResult,
    },
    Shutdown {
        config_revision: String,
        instance_id: Uuid,
    },
    Retire {
        config_revision: String,
        instance_id: Uuid,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BrokerCommandOutput {
    #[serde(with = "serde_bytes")]
    pub stdout: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub stderr: Vec<u8>,
    pub exit_status: Option<u32>,
    pub timed_out: bool,
    pub truncated: bool,
}

impl From<CommandOutput> for BrokerCommandOutput {
    fn from(value: CommandOutput) -> Self {
        Self {
            stdout: value.stdout,
            stderr: value.stderr,
            exit_status: value.exit_status,
            timed_out: value.timed_out,
            truncated: value.truncated,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BrokerFileResult {
    pub resolved_path: PathBuf,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    pub eof: bool,
}

impl From<FileReadResult> for BrokerFileResult {
    fn from(value: FileReadResult) -> Self {
        Self {
            resolved_path: value.resolved_path,
            data: value.data,
            eof: value.eof,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BrokerResponse {
    pub protocol_version: u16,
    pub request_id: Uuid,
    pub result: Option<BrokerResult>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl BrokerResponse {
    pub(super) fn success(request_id: Uuid, result: BrokerResult) -> Self {
        Self {
            protocol_version: BROKER_PROTOCOL_VERSION,
            request_id,
            result: Some(result),
            error_code: None,
            error_message: None,
        }
    }

    pub(super) fn failure(request_id: Uuid, code: &str, message: impl Into<String>) -> Self {
        Self {
            protocol_version: BROKER_PROTOCOL_VERSION,
            request_id,
            result: None,
            error_code: Some(code.into()),
            error_message: Some(message.into()),
        }
    }
}
