use yolo_hub_routing::{Right, Rights};
use yolo_hub_rpc_descriptor::*;
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Method};

#[test]
fn descriptor_preserves_combined_control_rights_and_local_management() {
    let described = ProtocolDescriptor::current();
    assert_eq!(described.version, HUB_PROTOCOL_VERSION);
    assert_eq!(described.schema_version, DESCRIPTOR_SCHEMA_VERSION);
    assert_eq!(described.protocol, PROTOCOL_NAME);
    let control = described
        .methods
        .iter()
        .find(|item| item.method == Method::AgentControl)
        .unwrap();
    let expected: Rights = [Right::AgentControl, Right::TerminalWrite]
        .into_iter()
        .collect();
    assert_eq!(
        control.access,
        AccessRequirement::Granted { rights: expected }
    );
    let configuration = described
        .methods
        .iter()
        .find(|item| item.method == Method::Configure)
        .unwrap();
    assert_eq!(configuration.access, AccessRequirement::LocalManagement);
    let wire = serde_json::to_value(described).unwrap();
    assert!(
        wire["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["method"] == "attach")
    );
}
