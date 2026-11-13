use super::*;

const NATIVE_METHODS: &[&str] = &[
    method::OBSERVE_NATIVE_AGENT,
    method::GET_NATIVE_CONTROL_RECEIPT,
    method::INTERRUPT_NATIVE_AGENT,
];
const NATIVE_SOCKET: &str = "native-control-worker.sock";
const OPAQUE_PAYLOAD_FIELD: &str = "worker-owned-payload";
const OPAQUE_PAYLOAD_VALUE: &str = "forward-this-without-reinterpretation";
const NO_NATIVE_CONNECTION_TIMEOUT: Duration = Duration::from_millis(100);

// Host validates routing scope; Worker owns each method's complete payload decoder.
fn opaque_native_payload(terminal_id: Uuid, epoch: Uuid) -> yolo_capnp_value::Value {
    let mut payload = to_value(&crate::worker::RuntimeRef::new(terminal_id, epoch)).unwrap();
    if let yolo_capnp_value::Value::Map(fields) = &mut payload {
        fields.push((OPAQUE_PAYLOAD_FIELD.into(), OPAQUE_PAYLOAD_VALUE.into()));
    } else {
        unreachable!();
    }
    payload
}

#[path = "native_control_dispatch_tests.rs"]
mod dispatch;
#[path = "native_control_drain_tests.rs"]
mod drain;
