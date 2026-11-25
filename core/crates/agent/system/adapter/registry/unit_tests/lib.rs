use crate::AgentRegistry;
use yolo_agent_adapter_contract::constants as adapter_constants;
use yolo_agent_contract::AgentKind;

#[test]
fn empty_registry_still_carries_the_process_contract_keys() {
    let registry = AgentRegistry::from_shared_adapters(Vec::new()).unwrap();
    let keys = registry.process_environment_keys();
    for key in [
        adapter_constants::HOME_ENV,
        adapter_constants::YOLO_TERMINAL_ID_ENV,
        adapter_constants::YOLO_RUNTIME_EPOCH_ENV,
        yolo_process::PTY_SESSION_CAPABILITY_ENV,
    ] {
        assert!(keys.contains(&key), "missing base key: {key}");
    }
    assert!(registry.adapter(&AgentKind::Codex).is_none());
}
