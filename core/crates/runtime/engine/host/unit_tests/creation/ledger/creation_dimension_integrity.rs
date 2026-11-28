use super::*;
use crate::host::tests::*;

const INTEGRITY_NAME: &str = "dimension-integrity";
const NONCANONICAL_DIMENSION: u16 = 1;

#[tokio::test]
async fn noncanonical_durable_dimensions_are_rejected_without_repairing_the_catalog() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let request = CreateTerminal {
        creation_token: Uuid::new_v4(),
        name: Some(INTEGRITY_NAME.into()),
        cwd: directory.path().to_path_buf(),
        cols: Some(NONCANONICAL_DIMENSION),
        rows: Some(NONCANONICAL_DIMENSION),
        initial_input: None,
        context: None,
    };
    let fingerprint = CreationSpec::fingerprint(&request).unwrap();
    let spec = CreationSpec::resolve(&request, fingerprint).unwrap();
    let ledger_path = host.paths.home().join(CREATION_LEDGER_FILE_NAME);
    let binding = lock_recover(&host.creation_capabilities)
        .reserve(
            &ledger_path,
            request.creation_token,
            spec,
            &host.creation_receipts,
        )
        .unwrap();
    let normalized = host.prepare_bound_session(&binding).await.unwrap();
    let mut malformed = normalized.clone();
    malformed.cols = NONCANONICAL_DIMENSION;
    malformed.rows = NONCANONICAL_DIMENSION;
    let path = host.paths.manifest(binding.result.terminal_id);
    atomic_write_json(&path, &malformed).unwrap();
    let before = std::fs::read(&path).unwrap();

    lock_recover(&host.catalog)
        .delete(binding.result.terminal_id)
        .unwrap();
    assert!(host.existing_bound_manifest(&binding).await.is_err());
    assert!(
        lock_recover(&host.catalog)
            .get(binding.result.terminal_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    atomic_write_json(&path, &normalized).unwrap();

    lock_recover(&host.catalog)
        .delete(binding.result.terminal_id)
        .unwrap();
    lock_recover(&host.catalog).insert(&normalized).unwrap();
    assert!(
        host.existing_bound_manifest(&binding)
            .await
            .unwrap()
            .is_some()
    );
    std::fs::remove_file(host.paths.manifest(binding.result.terminal_id)).unwrap();
    host.existing_bound_manifest(&binding)
        .await
        .unwrap()
        .unwrap();
    let repaired: TerminalManifest =
        read_json(&host.paths.manifest(binding.result.terminal_id)).unwrap();
    assert_eq!(repaired.cols, crate::model::MIN_TERMINAL_COLUMNS);
    assert_eq!(repaired.rows, crate::model::MIN_TERMINAL_ROWS);
}
