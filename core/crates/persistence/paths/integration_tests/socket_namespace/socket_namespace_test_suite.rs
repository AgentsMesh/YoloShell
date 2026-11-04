use super::*;
use crate::maintenance::{MaintenanceScanner, ScanChunk};
#[path = "socket_namespace_stage_unlink_recovery_tests.rs"]
mod stage_unlink_recovery;

const TEST_SCANNER_CHUNKS_PER_ENTRY: usize = 4;
const TEST_SCANNER_CHUNK_LIMIT: usize =
    MAX_PRIVATE_DIRECTORY_ENTRIES * TEST_SCANNER_CHUNKS_PER_ENTRY;
const TEST_SCANNER_PASS_EXHAUSTED: &str =
    "socket namespace scanner did not complete a bounded pass";

fn run_scanner_pass(namespace: &SocketNamespace) {
    let mut scanner = namespace.scanner().unwrap();
    finish_scanner_pass(&mut scanner);
}

fn finish_scanner_pass(scanner: &mut SocketNamespaceScanner) {
    for _ in 0..TEST_SCANNER_CHUNK_LIMIT {
        if scanner.scan_chunk().unwrap() == ScanChunk::PassComplete {
            return;
        }
    }
    panic!("{TEST_SCANNER_PASS_EXHAUSTED}");
}

#[path = "socket_namespace_connector_tests.rs"]
mod connector_tests;
#[path = "socket_namespace_empty_remove_fault_tests.rs"]
mod empty_remove_fault_tests;
#[path = "socket_namespace_hub_tests.rs"]
mod hub_tests;
#[path = "socket_namespace_orphan_certificate_tests.rs"]
mod orphan_certificate_tests;
#[path = "socket_namespace_orphan_cleanup_tests.rs"]
mod orphan_cleanup_tests;
#[path = "socket_namespace_path_tests.rs"]
mod path_tests;
#[path = "../socket_stage/socket_stage_probe_adversarial_tests.rs"]
mod probe_adversarial_tests;
#[path = "../socket_stage/socket_stage_probe_cleanup_matrix_tests.rs"]
mod probe_cleanup_matrix_tests;
#[path = "../socket_stage/socket_stage_probe_fault_matrix_tests.rs"]
mod probe_fault_matrix_tests;
#[path = "../socket_stage/socket_stage_probe_fence_matrix.rs"]
mod probe_fence_matrix;
#[path = "../socket_stage/socket_stage_probe_proof_matrix.rs"]
mod probe_proof_matrix;
#[path = "../socket_stage/socket_stage_probe_recovery_tests.rs"]
mod probe_recovery_tests;
#[path = "socket_namespace_public_client_tests.rs"]
mod public_client_tests;
#[path = "socket_namespace_root_override_tests.rs"]
mod root_override_tests;
#[path = "socket_namespace_stage_scavenge_tests.rs"]
mod stage_scavenge_tests;
#[path = "../../test_support/integration/socket_namespace_test_support.rs"]
mod test_support;
#[path = "socket_namespace_tests.rs"]
mod tests;
