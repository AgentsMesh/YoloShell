use crate::host::tests::*;
const RETIREMENT_WAVE6_ACTIVE_BYTES: &[u8] = b"retirement matrix active receipt";
const RETIREMENT_WAVE6_REPLACEMENT_BYTES: &[u8] = b"retirement matrix replacement";
const RETIREMENT_WAVE6_SAVED_NAME: &str = "retirement-matrix-saved";
const RETIREMENT_WAVE6_FAILURE: &str = "injected retirement matrix failure";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RetirementWave6Case {
    Normal,
    AlreadyMissing,
    BeforeMoveFailure,
    AfterMoveReplacement,
    AfterMoveOccupied,
    AfterMoveMissing,
    AfterSyncReplacement,
    AfterTruncateReplacement,
}

#[test]
fn one_retirement_instance_covers_commit_and_rollback_boundaries() {
    for case in [
        RetirementWave6Case::Normal,
        RetirementWave6Case::AlreadyMissing,
        RetirementWave6Case::BeforeMoveFailure,
        RetirementWave6Case::AfterMoveReplacement,
        RetirementWave6Case::AfterMoveOccupied,
        RetirementWave6Case::AfterMoveMissing,
        RetirementWave6Case::AfterSyncReplacement,
        RetirementWave6Case::AfterTruncateReplacement,
    ] {
        assert_retirement_case(case);
    }
}

fn assert_retirement_case(case: RetirementWave6Case) {
    use std::os::unix::ffi::OsStrExt;
    let (_directory, store, _, _) = receipt_test_setup();
    let active = cleanup_wave4_active_path(&store.staging);
    write_private_bytes(&active, RETIREMENT_WAVE6_ACTIVE_BYTES);
    let name = std::ffi::CString::new(active.file_name().unwrap().as_bytes()).unwrap();
    let slot = receipt_slot_name(&name).unwrap();
    let slot_path = receipt_entry_path(&store.staging, &slot);
    let saved = store.staging.path.join(RETIREMENT_WAVE6_SAVED_NAME);
    let (file, lock, identity) = open_receipt_staging(&store.staging, &name)
        .unwrap()
        .unwrap();
    let mut staged = ReceiptStagingFile {
        file,
        _lock: lock,
        identity,
        name,
        slot,
    };
    if case == RetirementWave6Case::AlreadyMissing {
        std::fs::remove_file(&active).unwrap();
    }

    let result = retire_receipt_staging_with(&store.staging, &mut staged, &mut |stage, _| {
        let selected = match case {
            RetirementWave6Case::AfterMoveReplacement
            | RetirementWave6Case::AfterMoveOccupied
            | RetirementWave6Case::AfterMoveMissing => ReceiptRetirementStage::AfterMove,
            RetirementWave6Case::AfterSyncReplacement => ReceiptRetirementStage::AfterDirectorySync,
            RetirementWave6Case::AfterTruncateReplacement => ReceiptRetirementStage::AfterTruncate,
            RetirementWave6Case::Normal
            | RetirementWave6Case::AlreadyMissing
            | RetirementWave6Case::BeforeMoveFailure => return retirement_before(case, stage),
        };
        if stage != selected {
            return Ok(());
        }
        std::fs::rename(&slot_path, &saved)?;
        if case != RetirementWave6Case::AfterMoveMissing {
            write_private_bytes(&slot_path, RETIREMENT_WAVE6_REPLACEMENT_BYTES);
        }
        if case == RetirementWave6Case::AfterMoveOccupied {
            write_private_bytes(&active, RETIREMENT_WAVE6_REPLACEMENT_BYTES);
        }
        Ok(())
    });

    match case {
        RetirementWave6Case::Normal | RetirementWave6Case::AlreadyMissing => {
            assert!(
                result.is_ok(),
                "unexpected failure for {case:?}: {result:?}"
            )
        }
        _ => assert!(result.is_err(), "fault unexpectedly succeeded: {case:?}"),
    }
}

fn retirement_before(case: RetirementWave6Case, stage: ReceiptRetirementStage) -> Result<()> {
    if case == RetirementWave6Case::BeforeMoveFailure && stage == ReceiptRetirementStage::BeforeMove
    {
        anyhow::bail!(RETIREMENT_WAVE6_FAILURE);
    }
    Ok(())
}
