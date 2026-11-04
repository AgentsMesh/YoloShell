#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_gate_capability_tests.rs"]
mod capability_tests;
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_gate_disconnected_peer_tests.rs"]
mod disconnected_peer_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_exec_gate_tests.rs"]
mod exec_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/bootstrap/bootstrap_gate_process_helper.rs"]
mod integration_process_helper;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_gate_protocol_adversarial_tests.rs"]
mod protocol_adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/bootstrap/bootstrap_gate_child_entrypoint.rs"]
mod child_entrypoint;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_gate_tests.rs"]
mod tests;
