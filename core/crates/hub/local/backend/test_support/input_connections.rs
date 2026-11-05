use super::{cleanup_rpc::ProtocolFixture, fixtures};
use tokio::task::JoinHandle;
use uuid::Uuid;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    InputFields, InputRequest, LeaseGrant, LeaseGrantFields, PublicError, SecretCapability,
};
use yolo_runtime::{
    protocol::{RequestEnvelope, ResponseEnvelope, delivery, method},
    worker::{InputRequest as WorkerInput, InputResult},
};

pub const FIRST_BYTES: &[u8] = b"first input";
pub const NEXT_BYTES: &[u8] = b"next input";
pub const REFUSED_INPUT: &str = "input intentionally rejected by protocol peer";

pub fn input(target: RuntimeTarget, data: &[u8]) -> InputRequest {
    let raw = fixtures::grant();
    let capability =
        SecretCapability::from_uuid(Uuid::from_slice(raw.writer_capability.as_bytes()).unwrap())
            .unwrap();
    let grant = LeaseGrant::new(LeaseGrantFields {
        target,
        principal: fixtures::principal(),
        generation: raw.generation,
        capability,
        expires_at_ms: raw.expires_at_ms,
        ttl_ms: raw.ttl_ms,
        renew_after_ms: raw.renew_after_ms,
    })
    .unwrap();
    InputRequest::new(InputFields {
        request_id: Uuid::new_v4(),
        grant,
        data: data.to_vec(),
    })
    .unwrap()
}

pub fn next_input(previous: &InputRequest) -> InputRequest {
    let mut fields = previous.fields().clone();
    fields.request_id = Uuid::new_v4();
    fields.data = NEXT_BYTES.to_vec();
    InputRequest::new(fields).unwrap()
}

pub fn pending(
    fixture: &ProtocolFixture,
    request: &InputRequest,
) -> JoinHandle<Result<Reply, PublicError>> {
    let backend = fixture.backend.clone();
    let command = Command::Input {
        request: request.clone(),
    };
    tokio::spawn(async move { backend.execute(&command).await })
}

pub fn delivered(incoming: &RequestEnvelope, expected: &InputRequest) -> ResponseEnvelope {
    assert_eq!(incoming.method, method::TERMINAL_INPUT);
    let actual: WorkerInput = incoming.decode_payload().unwrap();
    let fields = expected.fields();
    let grant = fields.grant.fields();
    assert_eq!(
        actual.terminal_id,
        grant.target.address.terminal_id.as_uuid()
    );
    assert_eq!(actual.runtime_epoch, grant.target.runtime_epoch.as_uuid());
    assert_eq!(actual.input_id, fields.request_id);
    assert_eq!(actual.data, fields.data);
    assert_eq!(actual.generation, grant.generation);
    assert_eq!(
        actual.writer_capability.as_bytes(),
        grant.capability.as_uuid().as_bytes()
    );
    ResponseEnvelope::success(
        incoming,
        &InputResult {
            input_id: fields.request_id,
            duplicate: false,
            delivery: delivery::DELIVERED.into(),
        },
    )
    .unwrap()
}

pub fn assert_delivered(reply: Result<Reply, PublicError>) {
    assert!(matches!(reply, Ok(Reply::Input { duplicate: false })));
}
