use crate::host::tests::*;
use super::*;

#[test]
fn conflicting_receipt_indexes_fail_closed_before_generation_bump() {
    let (directory, store, binding, ledger_path) = receipt_test_setup();
    write_active_binding(&ledger_path, &binding);
    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Finalized);
    publish_receipt_at(
        &store.staging,
        &store.token_index,
        &binding.creation_token_digest,
        &receipt,
    )
    .unwrap();
    let mut conflicting = receipt;
    conflicting.result.creation_capability = Uuid::new_v4();
    publish_receipt_at(
        &store.staging,
        &store.session_index,
        &binding.result.terminal_id.hyphenated().to_string(),
        &conflicting,
    )
    .unwrap();
    drop(store);
    assert!(Host::open(HostConfig::new(directory.path())).is_err());
}

#[test]
fn receipt_store_rejects_symlink_hardlink_mode_and_identity_corruption() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    for defect in ["symlink", "hardlink", "mode", "identity"] {
        let (directory, store, binding, _) = receipt_test_setup();
        let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Archived);
        let path = store.token_index.path.join(&binding.creation_token_digest);
        match defect {
            "symlink" => {
                let outside = directory.path().join("outside-receipt");
                std::fs::write(&outside, serde_json::to_vec(&receipt).unwrap()).unwrap();
                std::fs::set_permissions(
                    &outside,
                    std::fs::Permissions::from_mode(CREATION_RECEIPT_FILE_MODE),
                )
                .unwrap();
                symlink(&outside, &path).unwrap();
            }
            _ => {
                publish_receipt_at(
                    &store.staging,
                    &store.token_index,
                    &binding.creation_token_digest,
                    &receipt,
                )
                .unwrap();
                corrupt_receipt_fixture(defect, &directory, &path, &receipt);
            }
        }
        assert!(
            store
                .lookup_token_digest(&binding.creation_token_digest)
                .is_err()
        );
    }
}

fn corrupt_receipt_fixture(
    defect: &str,
    directory: &tempfile::TempDir,
    path: &Path,
    receipt: &CreationReceipt,
) {
    use std::os::unix::fs::PermissionsExt;
    match defect {
        "hardlink" => {
            std::fs::hard_link(path, directory.path().join("receipt-link")).unwrap();
        }
        "mode" => std::fs::set_permissions(
            path,
            std::fs::Permissions::from_mode(CREATION_RECEIPT_TEMP_MODE),
        )
        .unwrap(),
        "identity" => {
            let mut wrong = receipt.clone();
            wrong.creation_token_digest = yolo_crypto::sha256_hex(b"wrong-token");
            std::fs::set_permissions(
                path,
                std::fs::Permissions::from_mode(CREATION_RECEIPT_TEMP_MODE),
            )
            .unwrap();
            std::fs::write(path, serde_json::to_vec(&wrong).unwrap()).unwrap();
            std::fs::set_permissions(
                path,
                std::fs::Permissions::from_mode(CREATION_RECEIPT_FILE_MODE),
            )
            .unwrap();
        }
        _ => unreachable!(),
    }
}

#[test]
fn receipt_store_removes_only_strict_private_stale_temps() {
    let (directory, store, _, _) = receipt_test_setup();
    let stale = store.staging.path.join(format!(
        "{CREATION_RECEIPT_TEMP_PREFIX}{}{CREATION_RECEIPT_TEMP_SUFFIX}",
        Uuid::new_v4()
    ));
    std::fs::write(&stale, b"stale").unwrap();
    set_private_file_mode(&stale);
    let ignored = store.staging.path.join(".receipt.not-a-uuid.tmp");
    std::fs::write(&ignored, b"ignored").unwrap();
    set_private_file_mode(&ignored);
    drop(store);
    CreationReceiptStore::open(directory.path()).unwrap();
    assert!(!stale.exists());
    assert!(ignored.exists());
}

#[test]
fn receipt_staging_scan_is_bounded_by_all_directory_entries() {
    let (directory, store, _, _) = receipt_test_setup();
    let staging = store.staging.path.clone();
    drop(store);
    for index in 0..=MAX_CREATION_RECEIPT_STAGING_ENTRIES {
        std::fs::write(
            staging.join(format!(".receipt.not-a-uuid-{index}.tmp")),
            b"unrelated",
        )
        .unwrap();
    }

    assert!(CreationReceiptStore::open(directory.path()).is_err());
    assert_eq!(
        std::fs::read_dir(staging).unwrap().count(),
        MAX_CREATION_RECEIPT_STAGING_ENTRIES + 1
    );
}
