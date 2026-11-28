use crate::host::tests::*;
use super::*;

const CORRUPT_RECEIPT_BYTES: &[u8] = b"not-creation-receipt-json";
const INVALID_RECEIPT_DIGEST: &str = "invalid-digest";
const INVALID_RECEIPT_INDEX_NAME: &str = "invalid\0name";
const WRONG_RECEIPT_INDEX_TOKEN: &[u8] = b"wrong-index-token";
const MISMATCHED_RECEIPT_TOKEN: &[u8] = b"mismatched-receipt-token";
const RECEIPT_SCHEMA_OFFSET: u16 = 1;

#[derive(Clone, Copy, Debug)]
enum ReceiptCorruption {
    Schema,
    Digest,
    Session,
    Capability,
    Encoding,
}

#[test]
fn receipt_lookup_distinguishes_missing_and_each_partial_index() {
    for token_index in [true, false] {
        let (_directory, store, binding, _) = receipt_test_setup();
        assert!(store.lookup_binding(&binding).unwrap().is_none());
        assert!(matches!(
            store
                .lookup_identity(&binding.creation_token_digest, binding.result)
                .unwrap(),
            CreationReceiptLookup::Missing
        ));
        let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Archived);
        let (index, name) = if token_index {
            (&store.token_index, binding.creation_token_digest.clone())
        } else {
            (
                &store.session_index,
                binding.result.terminal_id.hyphenated().to_string(),
            )
        };
        publish_receipt_at(&store.staging, index, &name, &receipt).unwrap();
        assert_eq!(
            store.lookup_binding(&binding).unwrap(),
            Some(receipt.clone())
        );
        assert_eq!(
            store
                .lookup_identity(&binding.creation_token_digest, binding.result)
                .unwrap(),
            CreationReceiptLookup::Matched(receipt)
        );
    }
}

#[test]
fn receipt_lookup_rejects_invalid_queries_and_index_mismatches() {
    let (_directory, store, binding, _) = receipt_test_setup();
    assert!(store.lookup_token_digest(INVALID_RECEIPT_DIGEST).is_err());
    assert!(store.lookup_session(Uuid::nil()).is_err());
    assert!(read_receipt_at(&store.token_index, INVALID_RECEIPT_INDEX_NAME).is_err());

    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Finalized);
    let wrong_digest = yolo_crypto::sha256_hex(WRONG_RECEIPT_INDEX_TOKEN);
    publish_receipt_at(&store.staging, &store.token_index, &wrong_digest, &receipt).unwrap();
    assert!(store.lookup_token_digest(&wrong_digest).is_err());

    let wrong_session = Uuid::new_v4();
    publish_receipt_at(
        &store.staging,
        &store.session_index,
        &wrong_session.hyphenated().to_string(),
        &receipt,
    )
    .unwrap();
    assert!(store.lookup_session(wrong_session).is_err());
}

#[test]
fn receipt_lookup_rejects_a_receipt_for_another_binding() {
    let (_directory, store, binding, _) = receipt_test_setup();
    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Finalized);
    store.publish(&receipt).unwrap();
    let mut mismatched = binding.clone();
    mismatched.result.creation_capability = Uuid::new_v4();
    assert!(store.lookup_binding(&mismatched).is_err());
    assert_eq!(
        store
            .lookup_identity(&binding.creation_token_digest, mismatched.result)
            .unwrap(),
        CreationReceiptLookup::Mismatched
    );
    let other_digest = yolo_crypto::sha256_hex(MISMATCHED_RECEIPT_TOKEN);
    assert_eq!(
        store
            .lookup_identity(&other_digest, binding.result)
            .unwrap(),
        CreationReceiptLookup::Mismatched
    );

    let other = CreateTerminalResult {
        terminal_id: Uuid::new_v4(),
        creation_capability: binding.result.creation_capability,
    };
    assert_eq!(
        store
            .lookup_identity(&binding.creation_token_digest, other)
            .unwrap(),
        CreationReceiptLookup::Mismatched
    );
}

#[test]
fn receipt_read_rejects_every_persisted_record_corruption() {
    for corruption in [
        ReceiptCorruption::Schema,
        ReceiptCorruption::Digest,
        ReceiptCorruption::Session,
        ReceiptCorruption::Capability,
        ReceiptCorruption::Encoding,
    ] {
        let (_directory, store, binding, _) = receipt_test_setup();
        let path = store.token_index.path.join(&binding.creation_token_digest);
        let bytes = if matches!(corruption, ReceiptCorruption::Encoding) {
            CORRUPT_RECEIPT_BYTES.to_vec()
        } else {
            let mut receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Archived);
            match corruption {
                ReceiptCorruption::Schema => receipt.schema_version += RECEIPT_SCHEMA_OFFSET,
                ReceiptCorruption::Digest => receipt.creation_token_digest.make_ascii_uppercase(),
                ReceiptCorruption::Session => receipt.result.terminal_id = Uuid::nil(),
                ReceiptCorruption::Capability => receipt.result.creation_capability = Uuid::nil(),
                ReceiptCorruption::Encoding => unreachable!(),
            }
            serde_json::to_vec(&receipt).unwrap()
        };
        std::fs::write(&path, bytes).unwrap();
        set_receipt_path_mode(&path, CREATION_RECEIPT_FILE_MODE);
        assert!(
            store
                .lookup_token_digest(&binding.creation_token_digest)
                .is_err(),
            "accepted corrupt receipt {corruption:?}"
        );
    }
}
