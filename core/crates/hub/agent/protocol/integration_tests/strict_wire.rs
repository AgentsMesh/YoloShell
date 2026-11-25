use serde_json::Value;
use yolo_hub_agent_protocol::*;

use crate::constants::*;
use crate::fixtures::*;

#[test]
fn snapshots_reject_unknown_fields_at_every_mutable_protocol_boundary() {
    let snapshot = waiting(InteractionKind::Approval);
    let original = serde_json::to_value(snapshot).unwrap();
    for path in [
        "",
        "/cursor",
        "/cursor/target",
        "/cursor/target/address",
        "/terminal",
        "/terminal/address",
        "/provider",
        "/lifecycle",
        "/capabilities",
        "/capabilities/interrupt",
        "/interaction",
        "/interaction/reference",
        "/interaction/choices/0",
    ] {
        let mut wire = original.clone();
        wire.pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(INJECTED_FIELD.into(), Value::Bool(true));
        assert!(
            serde_json::from_value::<AgentSnapshot>(wire).is_err(),
            "{path}"
        );
    }
}

#[test]
fn control_wire_rejects_unknown_operations_fields_and_wrong_schema() {
    let snapshot = waiting(InteractionKind::Approval);
    let request = AgentControlRequest::new(response_fields(&snapshot)).unwrap();
    let original = serde_json::to_value(request).unwrap();
    for path in [
        "",
        "/target",
        "/expected_cursor",
        "/action",
        "/action/interaction",
        "/action/response",
    ] {
        let mut wire = original.clone();
        wire.pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(INJECTED_FIELD.into(), Value::Bool(true));
        assert!(
            serde_json::from_value::<AgentControlRequest>(wire).is_err(),
            "{path}"
        );
    }
    let mut unknown = original.clone();
    unknown["action"]["operation"] = UNKNOWN_OPERATION.into();
    assert!(serde_json::from_value::<AgentControlRequest>(unknown).is_err());
    let mut wrong_schema = original;
    wrong_schema[SCHEMA_VERSION_FIELD] = u16::MAX.into();
    assert!(serde_json::from_value::<AgentControlRequest>(wrong_schema).is_err());
}

#[test]
fn receipts_do_not_accept_synthetic_success_fields() {
    let snapshot = AgentSnapshot::new(fields()).unwrap();
    let request = request_fields(&snapshot);
    let receipt = CommandReceipt::accepted(request.request_id, request.target).unwrap();
    let original = serde_json::to_value(receipt).unwrap();
    for path in ["", "/outcome"] {
        let mut wire = original.clone();
        wire.pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(INJECTED_FIELD.into(), Value::Bool(true));
        assert!(serde_json::from_value::<CommandReceipt>(wire).is_err());
    }
}
