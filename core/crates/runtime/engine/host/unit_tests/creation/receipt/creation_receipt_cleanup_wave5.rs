use crate::host::tests::*;
const CLEANUP_WAVE5_ACTIVE_BYTES: &[u8] = b"active receipt retained during rollback";
const CLEANUP_WAVE5_SLOT_BYTES: &[u8] = b"retirement slot retained during rollback";
const CLEANUP_WAVE5_CAUSE: &str = "injected retirement verification failure";
const CLEANUP_WAVE5_RETAINED_DETAIL: &str = "receipt staging replacement was retained in its slot";

#[test]
fn occupied_active_name_retains_both_receipt_rollback_candidates() {
    use std::os::unix::ffi::OsStrExt;

    let (_directory, store, _, _) = receipt_test_setup();
    let active = cleanup_wave4_active_path(&store.staging);
    std::fs::write(&active, CLEANUP_WAVE5_ACTIVE_BYTES).unwrap();
    set_private_file_mode(&active);
    let name = std::ffi::CString::new(active.file_name().unwrap().as_bytes()).unwrap();
    let slot = receipt_slot_name(&name).unwrap();
    let slot_path = receipt_entry_path(&store.staging, &slot);
    let (file, lock, identity) = open_receipt_staging(&store.staging, &name)
        .unwrap()
        .unwrap();
    std::fs::write(&slot_path, CLEANUP_WAVE5_SLOT_BYTES).unwrap();
    set_private_file_mode(&slot_path);
    let staged = ReceiptStagingFile {
        file,
        _lock: lock,
        identity,
        name,
        slot,
    };

    let error = rollback_receipt_retirement(
        &store.staging,
        &staged,
        anyhow::anyhow!(CLEANUP_WAVE5_CAUSE),
    )
    .unwrap_err();

    assert!(error.to_string().contains(CLEANUP_WAVE5_RETAINED_DETAIL));
    assert_eq!(std::fs::read(active).unwrap(), CLEANUP_WAVE5_ACTIVE_BYTES);
    assert_eq!(std::fs::read(slot_path).unwrap(), CLEANUP_WAVE5_SLOT_BYTES);
}
