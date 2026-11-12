use crate::host::tests::*;
const CLEANUP_WAVE4_UNRELATED_NAME: &str = "unrelated-receipt-entry";
const CLEANUP_WAVE4_SAVED_NAME: &str = "saved-cleanup-receipt";
const CLEANUP_WAVE4_HOOK_ERROR: &str = "cleanup receipt hook failure";
const CLEANUP_WAVE4_ORIGINAL: &[u8] = b"original stale receipt";
const CLEANUP_WAVE4_REPLACEMENT: &[u8] = b"replacement stale receipt";
const CLEANUP_WAVE4_SLOT_BYTES: &[u8] = b"occupied retirement slot";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CleanupWave4Fault {
    Empty,
    Unrelated,
    BoundExceeded,
    Normal,
    HookFailure,
    SlotOccupied,
    SlotAppears,
    SourceReplacement,
}

#[test]
fn cleanup_receipt_faults_converge_through_one_hook_instance() {
    for fault in [
        CleanupWave4Fault::Empty,
        CleanupWave4Fault::Unrelated,
        CleanupWave4Fault::BoundExceeded,
        CleanupWave4Fault::Normal,
        CleanupWave4Fault::HookFailure,
        CleanupWave4Fault::SlotOccupied,
        CleanupWave4Fault::SlotAppears,
        CleanupWave4Fault::SourceReplacement,
    ] {
        let (_directory, store, _, _) = receipt_test_setup();
        let active = cleanup_wave4_active_path(&store.staging);
        let slot = cleanup_wave4_slot_path(&store.staging, &active);
        let saved = store.staging.path.join(CLEANUP_WAVE4_SAVED_NAME);
        match fault {
            CleanupWave4Fault::Empty => {}
            CleanupWave4Fault::Unrelated => {
                let unrelated = store.staging.path.join(CLEANUP_WAVE4_UNRELATED_NAME);
                std::fs::write(&unrelated, CLEANUP_WAVE4_ORIGINAL).unwrap();
                set_private_file_mode(&unrelated);
            }
            CleanupWave4Fault::BoundExceeded => {
                for index in 0..=MAX_CREATION_RECEIPT_STAGING_ENTRIES {
                    let unrelated = store
                        .staging
                        .path
                        .join(format!("{CLEANUP_WAVE4_UNRELATED_NAME}-{index}"));
                    std::fs::write(&unrelated, CLEANUP_WAVE4_ORIGINAL).unwrap();
                    set_private_file_mode(&unrelated);
                }
            }
            _ => {
                std::fs::write(&active, CLEANUP_WAVE4_ORIGINAL).unwrap();
                set_private_file_mode(&active);
            }
        }
        if fault == CleanupWave4Fault::SlotOccupied {
            std::fs::write(&slot, CLEANUP_WAVE4_SLOT_BYTES).unwrap();
            set_private_file_mode(&slot);
        }

        let result = cleanup_stale_receipts_with(&store.staging, |candidate| match fault {
            CleanupWave4Fault::HookFailure => anyhow::bail!(CLEANUP_WAVE4_HOOK_ERROR),
            CleanupWave4Fault::SlotAppears => {
                std::fs::write(&slot, CLEANUP_WAVE4_SLOT_BYTES)?;
                set_private_file_mode(&slot);
                Ok(())
            }
            CleanupWave4Fault::SourceReplacement => {
                std::fs::rename(candidate, &saved)?;
                std::fs::write(candidate, CLEANUP_WAVE4_REPLACEMENT)?;
                set_private_file_mode(candidate);
                Ok(())
            }
            _ => Ok(()),
        });

        match fault {
            CleanupWave4Fault::Empty => assert!(result.is_ok()),
            CleanupWave4Fault::Unrelated => {
                assert!(result.is_ok());
                assert!(
                    store
                        .staging
                        .path
                        .join(CLEANUP_WAVE4_UNRELATED_NAME)
                        .exists()
                );
            }
            CleanupWave4Fault::BoundExceeded => assert!(result.is_err()),
            CleanupWave4Fault::Normal => {
                assert!(result.is_ok());
                assert!(!active.exists());
                assert_eq!(std::fs::read(slot).unwrap(), Vec::<u8>::new());
            }
            CleanupWave4Fault::HookFailure => {
                assert!(result.is_err());
                assert_eq!(std::fs::read(active).unwrap(), CLEANUP_WAVE4_ORIGINAL);
            }
            CleanupWave4Fault::SlotOccupied | CleanupWave4Fault::SlotAppears => {
                assert!(result.is_err());
                assert_eq!(std::fs::read(active).unwrap(), CLEANUP_WAVE4_ORIGINAL);
                assert_eq!(std::fs::read(slot).unwrap(), CLEANUP_WAVE4_SLOT_BYTES);
            }
            CleanupWave4Fault::SourceReplacement => {
                assert!(result.is_err());
                assert_eq!(std::fs::read(active).unwrap(), CLEANUP_WAVE4_REPLACEMENT);
                assert_eq!(std::fs::read(saved).unwrap(), CLEANUP_WAVE4_ORIGINAL);
                assert!(!slot.exists());
            }
        }
    }
}

fn cleanup_wave4_active_path(staging: &ReceiptDirectory) -> PathBuf {
    staging.path.join(format!(
        "{CREATION_RECEIPT_TEMP_PREFIX}{}{CREATION_RECEIPT_TEMP_SUFFIX}",
        Uuid::new_v4()
    ))
}

fn cleanup_wave4_slot_path(staging: &ReceiptDirectory, active: &Path) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::CString::new(active.file_name().unwrap().as_bytes()).unwrap();
    receipt_entry_path(staging, &receipt_slot_name(&name).unwrap())
}
