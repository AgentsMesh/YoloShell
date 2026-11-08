use crate::host::tests::*;
use super::*;

const MOVED_RECEIPT_STORE_DIRECTORY: &str = "moved-creation-receipts";

#[test]
fn receipt_store_rejects_symlinked_ancestor_even_when_it_reaches_the_same_inode() {
    use std::os::unix::fs::symlink;
    let (directory, store, binding, _) = receipt_test_setup();
    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Finalized);
    let public = directory.path().join(CREATION_RECEIPT_STORE_DIRECTORY);
    let moved = directory.path().join(MOVED_RECEIPT_STORE_DIRECTORY);
    std::fs::rename(&public, &moved).unwrap();
    symlink(&moved, &public).unwrap();

    assert!(store.publish(&receipt).is_err());
    assert!(
        !moved
            .join(CREATION_RECEIPT_TOKEN_DIRECTORY)
            .join(&binding.creation_token_digest)
            .exists()
    );
    assert!(CreationReceiptStore::open(directory.path()).is_err());

    std::fs::remove_file(&public).unwrap();
    std::fs::rename(&moved, &public).unwrap();
    store.publish(&receipt).unwrap();
    assert_eq!(
        store
            .lookup_token_digest(&binding.creation_token_digest)
            .unwrap(),
        Some(receipt)
    );
}
