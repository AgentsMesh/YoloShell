use crate::host::tests::*;
use super::*;

pub(super) fn write_active_binding(path: &Path, binding: &CreationBinding) {
    write_creation_ledger(
        path,
        &CreationLedger {
            bindings: vec![binding.clone()],
            ..CreationLedger::default()
        },
    )
    .unwrap();
}

pub(super) fn assert_receipt_indexes_empty(store: &CreationReceiptStore) {
    for index in [&store.token_index, &store.session_index] {
        assert!(std::fs::read_dir(&index.path).unwrap().next().is_none());
    }
}

#[test]
fn receipt_publish_builds_private_immutable_o1_indexes() {
    use std::os::unix::fs::PermissionsExt;
    let (directory, store, binding, _) = receipt_test_setup();
    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Finalized);
    store.publish(&receipt).unwrap();
    assert_eq!(
        store
            .lookup_token_digest(&binding.creation_token_digest)
            .unwrap(),
        Some(receipt.clone())
    );
    assert_eq!(
        store.lookup_session(binding.result.terminal_id).unwrap(),
        Some(receipt)
    );
    for path in [
        directory
            .path()
            .join(CREATION_RECEIPT_STORE_DIRECTORY)
            .join(CREATION_RECEIPT_TOKEN_DIRECTORY)
            .join(&binding.creation_token_digest),
        directory
            .path()
            .join(CREATION_RECEIPT_STORE_DIRECTORY)
            .join(CREATION_RECEIPT_SESSION_DIRECTORY)
            .join(binding.result.terminal_id.hyphenated().to_string()),
    ] {
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & FILE_MODE_MASK,
            CREATION_RECEIPT_FILE_MODE
        );
    }
}

#[test]
fn startup_repairs_each_receipt_publish_boundary_before_ledger_removal() {
    for published in ["token", "session", "both"] {
        let (directory, store, binding, ledger_path) = receipt_test_setup();
        write_active_binding(&ledger_path, &binding);
        let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Archived);
        if published != "session" {
            publish_receipt_at(
                &store.staging,
                &store.token_index,
                &binding.creation_token_digest,
                &receipt,
            )
            .unwrap();
        }
        if published != "token" {
            publish_receipt_at(
                &store.staging,
                &store.session_index,
                &binding.result.terminal_id.hyphenated().to_string(),
                &receipt,
            )
            .unwrap();
        }
        drop(store);
        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        assert!(
            lock_recover(&host.creation_capabilities)
                .bindings
                .is_empty()
        );
        assert_eq!(
            host.creation_receipts
                .lookup_binding(&binding)
                .unwrap()
                .unwrap(),
            receipt
        );
    }
}

#[test]
fn finalize_ack_loss_is_idempotent_and_frees_active_capacity() {
    let (_directory, store, mut binding, ledger_path) = receipt_test_setup();
    let token = Uuid::new_v4();
    binding.creation_token_digest = creation_token_digest(token);
    write_active_binding(&ledger_path, &binding);
    let request = FinalizeCreationRequest {
        creation_token: token,
        terminal_id: binding.result.terminal_id,
        creation_capability: binding.result.creation_capability,
    };
    let mut ledger = CreationLedger::open(&ledger_path).unwrap();
    ledger.finalize(&ledger_path, request, &store).unwrap();
    ledger.finalize(&ledger_path, request, &store).unwrap();
    assert!(ledger.bindings.is_empty());
    assert!(
        store
            .lookup_token_digest(&creation_token_digest(token))
            .unwrap()
            .is_some()
    );
    let next = ledger
        .reserve(
            &ledger_path,
            Uuid::new_v4(),
            ledger_test_spec(binding.spec.cwd.as_path()),
            &store,
        )
        .unwrap();
    assert_ne!(next.result.terminal_id, binding.result.terminal_id);
}
