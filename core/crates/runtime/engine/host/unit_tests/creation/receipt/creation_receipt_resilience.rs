use crate::host::tests::*;
use super::*;

const RECEIPT_STAGING_RECYCLES: usize = 16;
const CONCURRENT_RECEIPT_PUBLISHERS: usize = 4;

#[test]
fn receipt_staging_reuses_one_bounded_retirement_slot() {
    let (_directory, store, binding, _) = receipt_test_setup();
    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Archived);
    let encoded = serde_json::to_vec(&receipt).unwrap();
    let target = std::ffi::CString::new(binding.creation_token_digest.as_str()).unwrap();
    let mut stable_entries = None;

    for _ in 0..RECEIPT_STAGING_RECYCLES {
        let mut staged =
            acquire_receipt_staging(&store.staging, &store.token_index, &target).unwrap();
        write_receipt_staging(&store.staging, &mut staged, &encoded).unwrap();
        retire_receipt_staging(&store.staging, &mut staged).unwrap();
        drop(staged);
        let entries = std::fs::read_dir(&store.staging.path).unwrap().count();
        assert_eq!(*stable_entries.get_or_insert(entries), entries);
        assert_eq!(receipt_retired_entries(&store.staging), 1);
    }
}

#[test]
fn independent_receipts_publish_concurrently_without_shared_staging_lock() {
    let (_directory, store, binding, _) = receipt_test_setup();
    let receipts = (0..CONCURRENT_RECEIPT_PUBLISHERS)
        .map(|index| {
            let mut binding = binding.clone();
            binding.creation_token_digest = yolo_crypto::sha256_hex(&index.to_be_bytes());
            binding.result.terminal_id = Uuid::new_v4();
            CreationReceipt::from_binding(&binding, CreationCompletion::Finalized)
        })
        .collect::<Vec<_>>();

    std::thread::scope(|scope| {
        let handles = receipts
            .iter()
            .map(|receipt| {
                scope.spawn(|| {
                    publish_receipt_at(
                        &store.staging,
                        &store.token_index,
                        &receipt.creation_token_digest,
                        receipt,
                    )
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.join().unwrap().unwrap();
        }
    });
    for receipt in receipts {
        assert_eq!(
            store
                .lookup_token_digest(&receipt.creation_token_digest)
                .unwrap(),
            Some(receipt)
        );
    }
}
