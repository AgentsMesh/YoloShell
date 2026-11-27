use crate::host::tests::*;
use crate::host::creation_receipt_staging_support_tests::{self as staging_hooks, Stage};

const STAGING_WAVE6_TARGET: &str = "staging-wave6-target";
const STAGING_WAVE6_SAVED_NAME: &str = "staging-wave6-saved";
const STAGING_WAVE6_REPLACEMENT: &[u8] = b"staging wave6 replacement";

#[derive(Clone, Copy, Debug)]
enum StagingWave6Case {
    CreateCollision,
    OpenReplacement,
    SlotMoveCollision,
}

#[test]
fn receipt_staging_rejects_each_name_race_without_overwriting() {
    for case in [
        StagingWave6Case::CreateCollision,
        StagingWave6Case::OpenReplacement,
        StagingWave6Case::SlotMoveCollision,
    ] {
        assert_staging_race(case);
    }
}

fn assert_staging_race(case: StagingWave6Case) {
    let (_directory, store, _, _) = receipt_test_setup();
    let target = std::ffi::CString::new(STAGING_WAVE6_TARGET).unwrap();
    let active_name = receipt_temporary_name(&store.token_index, &target).unwrap();
    let active = receipt_entry_path(&store.staging, &active_name);
    let saved = store.staging.path.join(STAGING_WAVE6_SAVED_NAME);
    match case {
        StagingWave6Case::CreateCollision => {}
        StagingWave6Case::OpenReplacement => {
            drop(acquire_receipt_staging(&store.staging, &store.token_index, &target).unwrap());
        }
        StagingWave6Case::SlotMoveCollision => {
            let mut staged =
                acquire_receipt_staging(&store.staging, &store.token_index, &target).unwrap();
            retire_receipt_staging(&store.staging, &mut staged).unwrap();
            drop(staged);
        }
    }
    let selected = match case {
        StagingWave6Case::CreateCollision => Stage::BeforeCreate,
        StagingWave6Case::OpenReplacement => Stage::AfterReadOpen,
        StagingWave6Case::SlotMoveCollision => Stage::BeforeSlotMove,
    };
    let _guard = staging_hooks::install(move |stage, path| {
        if stage != selected {
            return Ok(());
        }
        if matches!(case, StagingWave6Case::OpenReplacement) {
            std::fs::rename(path, &saved)?;
        }
        write_private_bytes(path, STAGING_WAVE6_REPLACEMENT);
        Ok(())
    });

    let result = acquire_receipt_staging(&store.staging, &store.token_index, &target);

    assert!(result.is_err(), "race unexpectedly succeeded: {case:?}");
    assert_eq!(std::fs::read(active).unwrap(), STAGING_WAVE6_REPLACEMENT);
}
