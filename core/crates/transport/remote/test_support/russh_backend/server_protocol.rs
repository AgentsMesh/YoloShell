use super::constants::{
    MEMORY_BOOT_ID, MEMORY_CONTROLLER_GENERATION, MEMORY_HOME, MEMORY_HOST_PID,
    MEMORY_NEXT_SEQUENCE, MEMORY_RUNTIME_EPOCH_VALUE,
};
use crate::host::HelloResult;
use crate::model::HostStatus;
use crate::protocol::{RequestEnvelope, ResponseEnvelope, method};
use crate::remote::constants::{REQUIRED_RUNTIME_CAPABILITIES, RUNTIME_SERVER_NAME};
use crate::worker::AttachResult;

pub struct RuntimeService;
impl yolo_capnp_rpc::Service for RuntimeService {
    fn call(&self, _: yolo_capnp_rpc::Method, value: yolo_capnp_rpc::Value) -> yolo_capnp_rpc::Future<'_, yolo_capnp_rpc::Reply> {
        Box::pin(async move {
            let request: RequestEnvelope = yolo_capnp_codec::from_value(value)?;
            Ok(yolo_capnp_rpc::Reply::value(yolo_capnp_codec::to_value(&response_for(&request))?))
        })
    }
}

pub fn response_for(request: &RequestEnvelope) -> ResponseEnvelope {
    if request.method == method::HELLO {
        return ResponseEnvelope::success(
            request,
            &HelloResult {
                server_name: RUNTIME_SERVER_NAME.into(),
                protocol_version: crate::model::PROTOCOL_VERSION,
                capabilities: capabilities(),
            },
        )
        .unwrap();
    }
    if request.method == method::HOST_STATUS {
        return ResponseEnvelope::success(request, &status()).unwrap();
    }
    if request.method == method::ATTACH_SESSION {
        return ResponseEnvelope::success(
            request,
            &AttachResult {
                runtime_epoch: uuid::Uuid::from_u128(MEMORY_RUNTIME_EPOCH_VALUE),
                snapshot: None,
                chunks: vec![],
                next_sequence: MEMORY_NEXT_SEQUENCE,
                resynchronized: false,
            },
        )
        .unwrap();
    }
    ResponseEnvelope::empty(request)
}

fn capabilities() -> Vec<String> {
    REQUIRED_RUNTIME_CAPABILITIES
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn status() -> HostStatus {
    HostStatus {
        protocol_version: crate::model::PROTOCOL_VERSION,
        boot_id: MEMORY_BOOT_ID.into(),
        host_pid: MEMORY_HOST_PID,
        controller_generation: MEMORY_CONTROLLER_GENERATION,
        home: MEMORY_HOME.into(),
        capabilities: capabilities(),
    }
}
