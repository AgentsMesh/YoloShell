use crate::host::tests::*;
use super::*;

pub(super) fn receipt_active_path(
    staging: &ReceiptDirectory,
    index: &ReceiptDirectory,
    name: &str,
) -> PathBuf {
    let target = std::ffi::CString::new(name).unwrap();
    receipt_entry_path(staging, &receipt_temporary_name(index, &target).unwrap())
}

#[test]
fn receipt_publish_source_swap_rolls_back_without_unlinking_replacement() {
    let (_directory, store, binding, _) = receipt_test_setup();
    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Finalized);
    let name = binding.creation_token_digest.as_str();
    let active = receipt_active_path(&store.staging, &store.token_index, name);
    let saved = store.staging.path.join("saved-receipt-staging");
    let replacement = b"receipt source replacement";
    let target = store.token_index.path.join(name);

    let result = publish_receipt_at_with_test_hook(
        &store.staging,
        &store.token_index,
        name,
        &receipt,
        |stage| {
            if stage == ReceiptPublishStage::BeforePublish {
                std::fs::rename(&active, &saved)?;
                std::fs::write(&active, replacement)?;
                set_private_file_mode(&active);
                set_receipt_path_mode(&active, CREATION_RECEIPT_FILE_MODE);
            }
            Ok(())
        },
    );

    assert!(result.is_err());
    assert!(!target.exists());
    assert_eq!(std::fs::read(active).unwrap(), replacement);
    assert_eq!(
        std::fs::read(saved).unwrap(),
        serde_json::to_vec(&receipt).unwrap()
    );
}

#[test]
fn receipt_publish_target_appearance_preserves_the_existing_inode() {
    let (_directory, store, binding, _) = receipt_test_setup();
    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Finalized);
    let mut conflicting = receipt.clone();
    conflicting.completion = CreationCompletion::Archived;
    let name = binding.creation_token_digest.as_str();
    let target = store.token_index.path.join(name);
    let bytes = serde_json::to_vec(&conflicting).unwrap();

    let result = publish_receipt_at_with_test_hook(
        &store.staging,
        &store.token_index,
        name,
        &receipt,
        |stage| {
            if stage == ReceiptPublishStage::BeforePublish {
                std::fs::write(&target, &bytes)?;
                set_receipt_path_mode(&target, CREATION_RECEIPT_FILE_MODE);
            }
            Ok(())
        },
    );

    assert!(result.is_err());
    assert_eq!(std::fs::read(target).unwrap(), bytes);
    assert!(!receipt_active_path(&store.staging, &store.token_index, name).exists());
}

#[test]
fn receipt_cleanup_source_swap_restores_replacement_without_truncating_it() {
    let (_directory, store, _, _) = receipt_test_setup();
    let active = store.staging.path.join(format!(
        "{CREATION_RECEIPT_TEMP_PREFIX}{}{CREATION_RECEIPT_TEMP_SUFFIX}",
        Uuid::new_v4()
    ));
    let saved = store.staging.path.join("saved-stale-receipt");
    let original = b"trusted stale receipt";
    let replacement = b"stale receipt replacement";
    std::fs::write(&active, original).unwrap();
    set_private_file_mode(&active);
    let hooked = std::cell::Cell::new(false);

    let result = cleanup_stale_receipts_with(&store.staging, |candidate| {
        hooked.set(true);
        std::fs::rename(candidate, &saved)?;
        std::fs::write(candidate, replacement)?;
        set_private_file_mode(candidate);
        Ok(())
    });

    assert!(hooked.get());
    assert!(result.is_err());
    assert_eq!(std::fs::read(active).unwrap(), replacement);
    assert_eq!(std::fs::read(saved).unwrap(), original);
    assert_eq!(receipt_retired_entries(&store.staging), 0);
}

pub(super) fn set_receipt_path_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

pub(super) fn receipt_retired_entries(staging: &ReceiptDirectory) -> usize {
    std::fs::read_dir(&staging.path)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(CREATION_RECEIPT_SLOT_PREFIX)
        })
        .count()
}
