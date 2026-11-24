use super::constants::{REQUIRED_RUNTIME_CAPABILITIES, RUNTIME_CLIENT_NAME};
use super::deadline::RemoteDeadline;
use super::protocol::validate_runtime;
use super::rpc::RemoteRpc;
use super::transport::validate_channel;
use super::*;
use crate::host::{HelloRequest, HelloResult};
use crate::model::HostStatus;
use crate::protocol::{EventEnvelope, method};
use crate::worker::{AttachRequest, AttachResult};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use uuid::Uuid;

pub struct RemoteHost {
    connection: Arc<dyn MultiplexedSshConnection>,
    runtime: RemoteRuntimeSpec,
    control: Option<RemoteRpc>,
    status: HostStatus,
    state: RemoteState,
}

pub struct RemoteAttachment {
    rpc: RemoteRpc,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    pub initial: AttachResult,
}

include!("client/connection.rs");
include!("client/operations.rs");
include!("client/attachment.rs");
include!("client/handshake.rs");
