use std::path::PathBuf;
use uuid::Uuid;
use yolo_terminal_model::{
    BootstrapDelivery, TerminalLifecycleSelection, RecoveryState, TerminalManifest, TerminalManifestDigest,
    TerminalRuntimeStatus, WriterCapability, normalize_terminal_columns, normalize_terminal_rows,
};

const TERMINAL: Uuid = Uuid::from_u128(1);
const EPOCH: Uuid = Uuid::from_u128(2);
const MIN_COLUMNS: u16 = 2;
const WIDE_COLUMNS: u16 = 120;

#[test]
fn terminal_dimensions_and_bootstrap_states_are_canonical() {
    assert_eq!(normalize_terminal_columns(0), MIN_COLUMNS);
    assert_eq!(normalize_terminal_rows(0), MIN_COLUMNS);
    assert_eq!(normalize_terminal_columns(WIDE_COLUMNS), WIDE_COLUMNS);
    assert!(BootstrapDelivery::Delivered.is_ready());
    assert!(BootstrapDelivery::Survivor.is_serviceable());
    assert!(!BootstrapDelivery::Pending.is_serviceable());
}

#[test]
fn manifest_digest_is_stable_and_detects_mutation() {
    let manifest = manifest();
    let digest = TerminalManifestDigest::capture(&manifest).unwrap();
    assert_eq!(TerminalManifestDigest::capture(&manifest).unwrap(), digest);
    let mut changed = manifest;
    changed.name.push_str(" changed");
    assert_ne!(TerminalManifestDigest::capture(&changed).unwrap(), digest);
}

#[test]
fn product_selection_records_explicit_fences() {
    let selected = TerminalLifecycleSelection::selected(TERMINAL, 10, TerminalRuntimeStatus::Running);
    assert!(selected.is_conditional());
    let exact = selected
        .with_expected_manifest_sha256(Some(TerminalManifestDigest::capture(&manifest()).unwrap()))
        .with_expected_runtime_epoch(Some(EPOCH));
    assert!(exact.is_exact_catalog_selection());
    assert!(!TerminalLifecycleSelection::unconditional(TERMINAL).is_conditional());
}

#[test]
fn sensitive_writer_capability_round_trips_without_debugging_secrets() {
    let capability = WriterCapability::generate();
    let encoded = serde_json::to_vec(&capability).unwrap();
    assert_eq!(
        serde_json::from_slice::<WriterCapability>(&encoded).unwrap(),
        capability
    );
    let debug = format!("{capability:?}");
    assert!(debug.contains("REDACTED") || !debug.contains("WriterCapability("));
}

fn manifest() -> TerminalManifest {
    TerminalManifest {
        schema_version: 1,
        terminal_id: TERMINAL,
        name: "session".into(),
        command: "/bin/sh".into(),
        args: Vec::new(),
        cwd: PathBuf::from("/workspace"),
        cols: WIDE_COLUMNS,
        rows: 32,
        created_at_ms: 1,
        updated_at_ms: 2,
        archived: false,
        recovery_state: RecoveryState::Created,
    }
}
