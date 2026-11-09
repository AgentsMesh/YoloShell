use super::super::*;
use std::fs::{self, OpenOptions};
use std::io::{Seek, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const TARGET_NAME: &str = "retained-instance.bin";
const ORIGINAL_BYTES: &[u8] = b"retained-original";
const STAGED_BYTES: &[u8] = b"retained-staged";
const OCCUPANT_BYTES: &[u8] = b"retained-occupant-expanded";
const OVERWRITE_BYTES: &[u8] = b"z";
const FENCE_ERROR: &str = "injected retained fence failure";
const FENCE_CALL_OVERFLOW: &str = "retained fence call count overflow";
const INSTALL_HOOK_ERROR: &str = "injected retained install hook failure";
const PREPARE_ERROR: &str = "injected retained prepare failure";
const RENAME_HOOK_ERROR: &str = "injected retained rename hook failure";
const SAVED_NAME: &str = "saved-retained-target";
const FIRST_BYTE_OFFSET: u64 = 0;
const FENCE_CALL_INCREMENT: usize = 1;
const INITIAL_FENCE_CALL_COUNT: usize = 0;
const SECOND_BYTE_OFFSET: u64 = 1;
const SECOND_FENCE_CALL: usize = 2;
const OVERFLOW_WRITE_BYTE: u8 = b'x';

#[derive(Clone, Copy, Debug)]
enum StageCase {
    Empty,
    BeyondLimit,
    Multiple,
    Overflow,
    PrepareFailure,
    Valid,
}

#[derive(Clone, Copy, Debug)]
enum InstallCase {
    Success,
    FenceFailure,
    InstallHookFailure,
    InstallReplacement,
    IsolationReplacement,
    RenameFailure,
    RenameHookFailure,
    SecondFenceFailure,
    StageMutation,
}

#[test]
fn one_retained_stage_instance_covers_new_reused_and_rejected_writes() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    write_private(&target, ORIGINAL_BYTES);
    let entry = PrivateFileEntry::ensure(&target).unwrap();

    assert!(stage(&entry, StageCase::Empty).is_err());
    assert!(stage(&entry, StageCase::BeyondLimit).is_err());
    assert!(stage(&entry, StageCase::PrepareFailure).is_err());
    assert!(stage(&entry, StageCase::Overflow).is_err());
    drop(stage(&entry, StageCase::Multiple).unwrap().0);
    drop(stage(&entry, StageCase::Valid).unwrap().0);
    drop(stage(&entry, StageCase::Valid).unwrap().0);
}

#[test]
fn one_retained_install_instance_covers_commit_abort_and_rename_collision() {
    for case in [
        InstallCase::Success,
        InstallCase::FenceFailure,
        InstallCase::InstallHookFailure,
        InstallCase::InstallReplacement,
        InstallCase::IsolationReplacement,
        InstallCase::RenameFailure,
        InstallCase::RenameHookFailure,
        InstallCase::SecondFenceFailure,
        InstallCase::StageMutation,
    ] {
        run_install(case);
    }
}

#[test]
fn rollback_rejects_a_stage_that_never_owned_an_isolated_target() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    write_private(&target, ORIGINAL_BYTES);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let expected = entry.open_retained(u64::MAX).unwrap().identity;
    let mut temporary = stage(&entry, StageCase::Valid).unwrap().0;

    assert!(
        entry
            .rollback_retained_install(&mut temporary, expected, expected, u64::MAX)
            .is_err()
    );
}

fn stage(entry: &PrivateFileEntry, case: StageCase) -> Result<(RetainedPrivateFile, ())> {
    let max_bytes = match case {
        StageCase::BeyondLimit => u64::try_from(ORIGINAL_BYTES.len()).unwrap(),
        StageCase::Empty
        | StageCase::Multiple
        | StageCase::Overflow
        | StageCase::PrepareFailure
        | StageCase::Valid => u64::MAX,
    };
    entry.stage_retained(max_bytes, move |_, _| {
        if matches!(case, StageCase::PrepareFailure) {
            anyhow::bail!(PREPARE_ERROR);
        }
        let writes = match case {
            StageCase::Empty => Vec::new(),
            StageCase::BeyondLimit => vec![RetainedWrite {
                bytes: vec![OVERFLOW_WRITE_BYTE],
                offset: max_bytes,
            }],
            StageCase::Overflow => vec![RetainedWrite {
                bytes: vec![OVERFLOW_WRITE_BYTE],
                offset: u64::MAX,
            }],
            StageCase::Multiple => vec![
                RetainedWrite {
                    bytes: STAGED_BYTES.to_vec(),
                    offset: FIRST_BYTE_OFFSET,
                },
                RetainedWrite {
                    bytes: OVERWRITE_BYTES.to_vec(),
                    offset: SECOND_BYTE_OFFSET,
                },
            ],
            StageCase::PrepareFailure => unreachable!(),
            StageCase::Valid => vec![RetainedWrite {
                bytes: STAGED_BYTES.to_vec(),
                offset: FIRST_BYTE_OFFSET,
            }],
        };
        Ok((writes, ()))
    })
}

include!("private_file_retained_install_matrix.rs");
