use super::*;
use serde::Serialize;

#[derive(Serialize)]
struct UnconditionalTerminalRef {
    terminal_id: Uuid,
}

#[derive(Serialize)]
struct UnconditionalTerminateRequest {
    terminal_id: Uuid,
    force: bool,
}

const TEST_UPDATED_AT_MS: i64 = 17;
const TEST_MANIFEST_SHA256: &str =
    "b6c4f71b4cfb9a3dad44f196d9939984013a1a9c4883c7ce2cbde8e6382598cc";

#[test]
fn selected_and_unconditional_requests_report_their_fence() {
    let terminal_id = Uuid::new_v4();
    let selected =
        TerminalLifecycleSelection::selected(terminal_id, TEST_UPDATED_AT_MS, TerminalRuntimeStatus::Exited);
    assert!(selected.is_conditional());
    assert_eq!(selected.expected_updated_at_ms, Some(TEST_UPDATED_AT_MS));
    assert_eq!(selected.expected_status, Some(TerminalRuntimeStatus::Exited));
    assert_eq!(selected.expected_manifest_sha256, None);
    assert_eq!(selected.expected_runtime_epoch, None);
    assert!(!selected.require_runtime_absent);

    let unconditional = TerminalLifecycleSelection::unconditional(terminal_id);
    assert!(!unconditional.is_conditional());
    assert_eq!(unconditional.expected_updated_at_ms, None);
    assert_eq!(unconditional.expected_status, None);
    assert_eq!(unconditional.expected_manifest_sha256, None);
    assert_eq!(unconditional.expected_runtime_epoch, None);
    let mut absence = unconditional.clone();
    absence.require_runtime_absent = true;
    assert!(absence.is_conditional());
    let decoded: TerminalLifecycleSelection =
        serde_json::from_value(serde_json::to_value(&absence).unwrap()).unwrap();
    assert_eq!(decoded, absence);

    let epoch = Uuid::new_v4();
    let epoch_fenced = TerminalLifecycleSelection::at_runtime_epoch(terminal_id, epoch);
    assert!(!epoch_fenced.is_conditional());
    assert_eq!(epoch_fenced.expected_runtime_epoch, Some(epoch));

    let digest: TerminalManifestDigest = TEST_MANIFEST_SHA256.parse().unwrap();
    let manifest_fenced = TerminalLifecycleSelection::unconditional(terminal_id)
        .with_expected_manifest_sha256(Some(digest.clone()));
    assert!(manifest_fenced.is_conditional());
    assert_eq!(manifest_fenced.expected_manifest_sha256, Some(digest));

    let exact = selected
        .with_expected_manifest_sha256(manifest_fenced.expected_manifest_sha256)
        .with_expected_runtime_epoch(Some(epoch));
    assert!(exact.is_exact_catalog_selection());
    assert!(
        !TerminalLifecycleSelection::at_runtime_epoch(terminal_id, Uuid::nil()).is_exact_catalog_selection()
    );
}

#[test]
fn product_requests_decode_unconditional_selections() {
    let terminal_id = Uuid::new_v4();
    let archive: TerminalLifecycleSelection =
        yolo_wire::from_value(yolo_wire::to_value(&UnconditionalTerminalRef { terminal_id }).unwrap())
            .unwrap();
    let discard: TerminalDiscardRequest = yolo_wire::from_value(
        yolo_wire::to_value(&UnconditionalTerminateRequest {
            terminal_id,
            force: true,
        })
        .unwrap(),
    )
    .unwrap();
    let terminate: TerminalTerminateRequest = yolo_wire::from_value(
        yolo_wire::to_value(&UnconditionalTerminateRequest {
            terminal_id,
            force: false,
        })
        .unwrap(),
    )
    .unwrap();

    assert_eq!(archive, TerminalLifecycleSelection::unconditional(terminal_id));
    assert_eq!(discard.selection, archive);
    assert!(discard.force);
    assert_eq!(terminate.selection, archive);
    assert!(!terminate.force);
}
