use crate::host::tests::*;
use super::*;

use crate::protocol::from_value;

#[test]
fn host_wire_requests_cover_defaults_and_explicit_values() {
    let empty_list: ListSessionsRequest =
        from_value(yolo_capnp_value::Value::Map(Vec::new())).unwrap();
    assert!(!empty_list.include_archived);
    assert!(!empty_list.require_complete);
    let list: ListSessionsRequest = from_value(
        to_value(&ListSessionsRequest {
            include_archived: true,
            require_complete: true,
        })
        .unwrap(),
    )
    .unwrap();
    assert!(list.include_archived);
    assert!(list.require_complete);
    let unknown_list = yolo_capnp_value::Value::Map(vec![(
        yolo_capnp_value::Value::from(crate::protocol::field::MISSPELLED_REQUIRE_COMPLETE),
        yolo_capnp_value::Value::Boolean(true),
    )]);
    assert!(from_value::<ListSessionsRequest>(unknown_list).is_err());

    let terminal_id = Uuid::new_v4();
    let default_resume: ResumeRequest = from_value(yolo_capnp_value::Value::Map(vec![(
        yolo_capnp_value::Value::from(crate::protocol::field::TERMINAL_ID),
        yolo_capnp_value::Value::from(terminal_id.to_string()),
    )]))
    .unwrap();
    assert!(!default_resume.recreate);
    assert_eq!(
        default_resume.selection,
        TerminalLifecycleSelection::unconditional(terminal_id)
    );
    let resume: ResumeRequest = from_value(
        to_value(&ResumeRequest {
            selection: TerminalLifecycleSelection::unconditional(terminal_id),
            recreate: true,
        })
        .unwrap(),
    )
    .unwrap();
    assert!(resume.recreate);
}

#[test]
fn host_wire_results_and_hello_shapes_roundtrip() {
    let list: ListSessionsResult = from_value(
        to_value(&ListSessionsResult {
            sessions: Vec::new(),
            complete: true,
        })
        .unwrap(),
    )
    .unwrap();
    assert!(list.sessions.is_empty());
    assert!(list.complete);
    let legacy = from_value::<ListSessionsResult>(yolo_capnp_value::Value::Map(vec![(
        yolo_capnp_value::Value::from(crate::protocol::field::SESSIONS),
        yolo_capnp_value::Value::List(Vec::new()),
    )]));
    assert!(legacy.is_err());

    let result: ReconcileResult = from_value(
        to_value(&ReconcileResult {
            reattached: 1,
            exited: 2,
            unreachable: 3,
            needs_recreate: 4,
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(result.needs_recreate, 4);

    let hello: HelloRequest = from_value(
        to_value(&HelloRequest {
            client_name: Some("desktop".into()),
            protocol_version: crate::model::PROTOCOL_VERSION,
            capabilities: vec!["terminal".into()],
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(hello.client_name.as_deref(), Some("desktop"));
    assert!(from_value::<HelloRequest>(yolo_capnp_value::Value::Map(Vec::new())).is_err());
    for required in [stringify!(protocol_version), stringify!(capabilities)] {
        let mut value = serde_json::to_value(&hello).unwrap();
        value.as_object_mut().unwrap().remove(required);
        assert!(serde_json::from_value::<HelloRequest>(value).is_err());
    }

    let result: HelloResult = from_value(
        to_value(&HelloResult {
            server_name: "yolo-host".into(),
            protocol_version: crate::model::PROTOCOL_VERSION,
            capabilities: vec!["terminal".into()],
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(result.server_name, "yolo-host");
}
