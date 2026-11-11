use yolo_agent_runtime::{NATIVE_CALLBACK_SHELLS, NATIVE_HOOK_EXECUTABLE_ENV, NATIVE_HOOK_HOME_ENV};

#[test]
fn native_hook_environment_contract_stays_stable() {
    assert_eq!(NATIVE_HOOK_EXECUTABLE_ENV, "YOLO_HOOK_EXECUTABLE");
    assert_eq!(NATIVE_HOOK_HOME_ENV, "YOLO_HOOK_HOME");
    assert!(!NATIVE_CALLBACK_SHELLS.is_empty());
    assert!(NATIVE_CALLBACK_SHELLS.iter().all(|shell| shell.starts_with('/')));
}
