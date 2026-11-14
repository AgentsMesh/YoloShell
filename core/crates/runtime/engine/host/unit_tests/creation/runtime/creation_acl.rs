use crate::host::tests::*;
use super::*;

#[cfg(target_os = "macos")]
#[test]
fn creation_ledger_rejects_a_grant_acl() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    write_creation_ledger(&path, &CreationLedger::default()).unwrap();
    crate::acl_fixture::add_read_grant(&path);

    assert!(CreationLedger::open(&path).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn creation_receipt_rejects_a_grant_acl() {
    let (_directory, store, binding, _) = receipt_test_setup();
    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Archived);
    publish_receipt_at(
        &store.staging,
        &store.token_index,
        &binding.creation_token_digest,
        &receipt,
    )
    .unwrap();
    let path = store.token_index.path.join(&binding.creation_token_digest);
    crate::acl_fixture::add_read_grant(&path);

    assert!(
        store
            .lookup_token_digest(&binding.creation_token_digest)
            .is_err()
    );
}
