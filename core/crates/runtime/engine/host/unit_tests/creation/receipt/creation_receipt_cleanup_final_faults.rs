use crate::host::tests::*;
const CLEANUP_FINAL_ACTIVE_BYTES: &[u8] = b"cleanup final active receipt";
const CLEANUP_FINAL_HOOK_FAILURE: &str = "injected cleanup final hook failure";
const CLEANUP_FINAL_ROLLBACK_DETAIL: &str = "receipt staging retirement was rolled back";
const CLEANUP_FINAL_MOVE_DETAIL: &str = "move creation receipt exclusively";
const CLEANUP_FINAL_OPEN_DETAIL: &str = "open receipt staging stream";
const CLEANUP_FINAL_ENUMERATE_DETAIL: &str = "enumerate receipt staging";
const CLEANUP_FINAL_READ_DETAIL: &str = "read receipt staging";
const CLEANUP_FINAL_REGULAR_FILE: &str = "cleanup-final-regular-file";

#[test]
fn retirement_hook_failures_rollback_every_post_move_stage() {
    for selected in [
        ReceiptRetirementStage::AfterMove,
        ReceiptRetirementStage::AfterDirectorySync,
        ReceiptRetirementStage::AfterTruncate,
    ] {
        let (_directory, store, active, _slot, mut staged) = cleanup_final_fixture();
        let error = retire_receipt_staging_with(&store.staging, &mut staged, &mut |stage, _| {
            if stage == selected {
                anyhow::bail!(CLEANUP_FINAL_HOOK_FAILURE);
            }
            Ok(())
        })
        .unwrap_err();

        assert!(error.to_string().contains(CLEANUP_FINAL_ROLLBACK_DETAIL));
        assert!(active.is_file());
    }
}

#[test]
fn retirement_reports_a_source_removed_at_the_move_boundary() {
    let (_directory, store, active, slot, mut staged) = cleanup_final_fixture();
    let remove = active.clone();
    let error = retire_receipt_staging_with(&store.staging, &mut staged, &mut |stage, _| {
        if stage == ReceiptRetirementStage::BeforeMove {
            std::fs::remove_file(&remove)?;
        }
        Ok(())
    })
    .unwrap_err();

    assert!(error.to_string().contains(CLEANUP_FINAL_MOVE_DETAIL));
    assert!(!active.exists());
    assert!(!slot.exists());
}

#[test]
fn receipt_stream_rejects_a_regular_file_descriptor() {
    use std::os::fd::AsRawFd;

    let directory = tempdir().unwrap();
    let path = directory.path().join(CLEANUP_FINAL_REGULAR_FILE);
    write_private_bytes(&path, CLEANUP_FINAL_ACTIVE_BYTES);
    let file = File::open(path).unwrap();

    let error = ReceiptDirectoryStream::open(file.as_raw_fd())
        .err()
        .unwrap();

    assert!(error.to_string().contains(CLEANUP_FINAL_OPEN_DETAIL));
}

#[test]
fn receipt_stream_surfaces_directory_stream_creation_failure() {
    let (_directory, store, _, _) = receipt_test_setup();
    let _guard = install_receipt_stream_fault(ReceiptStreamFault::Open);

    let error = ReceiptDirectoryStream::open(store.staging.file.as_raw_fd())
        .err()
        .unwrap();

    assert!(error.to_string().contains(CLEANUP_FINAL_ENUMERATE_DETAIL));
}

#[test]
fn receipt_stream_surfaces_directory_read_failure() {
    let (_directory, store, _, _) = receipt_test_setup();
    let mut stream = ReceiptDirectoryStream::open(store.staging.file.as_raw_fd()).unwrap();
    let _guard = install_receipt_stream_fault(ReceiptStreamFault::Read);

    let error = stream.next_name().err().unwrap();

    assert!(error.to_string().contains(CLEANUP_FINAL_READ_DETAIL));
}

fn cleanup_final_fixture() -> (
    tempfile::TempDir,
    CreationReceiptStore,
    PathBuf,
    PathBuf,
    ReceiptStagingFile,
) {
    use std::os::unix::ffi::OsStrExt;

    let (directory, store, _, _) = receipt_test_setup();
    let active = cleanup_wave4_active_path(&store.staging);
    write_private_bytes(&active, CLEANUP_FINAL_ACTIVE_BYTES);
    let name = std::ffi::CString::new(active.file_name().unwrap().as_bytes()).unwrap();
    let slot = receipt_slot_name(&name).unwrap();
    let slot_path = receipt_entry_path(&store.staging, &slot);
    let (file, lock, identity) = open_receipt_staging(&store.staging, &name)
        .unwrap()
        .unwrap();
    let staged = ReceiptStagingFile {
        file,
        _lock: lock,
        identity,
        name,
        slot,
    };
    (directory, store, active, slot_path, staged)
}
