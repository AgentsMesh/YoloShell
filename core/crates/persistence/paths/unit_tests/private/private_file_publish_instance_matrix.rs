use super::super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const TARGET_NAME: &str = "publish-instance.bin";
const STAGE_PREFIX: &str = ".yolo-publish-instance-";
const STAGE_SUFFIX: &str = ".tmp";
const SAVED_NAME: &str = "saved-publish-instance";
const ORIGINAL_BYTES: &[u8] = b"publish-original";
const STAGED_BYTES: &[u8] = b"publish-staged--";
const REPLACEMENT_BYTES: &[u8] = b"publish-replacement-expanded";
const BEFORE_COMMIT_ERROR: &str = "injected before publish commit";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PublishCase {
    ExchangeSuccess,
    ExchangeInvalid,
    InstallSuccess,
    InstallRollback,
    InstallInvalid,
    BeforeCommitFailure,
    BeforeCommitStageReplacement,
    BeforeCommitTargetReplacement,
}

#[test]
fn one_publish_instance_covers_both_modes_and_every_commit_outcome() {
    for case in [
        PublishCase::ExchangeSuccess,
        PublishCase::ExchangeInvalid,
        PublishCase::InstallSuccess,
        PublishCase::InstallRollback,
        PublishCase::InstallInvalid,
        PublishCase::BeforeCommitFailure,
        PublishCase::BeforeCommitStageReplacement,
        PublishCase::BeforeCommitTargetReplacement,
    ] {
        run_case(case);
    }
}

fn run_case(case: PublishCase) {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    if matches!(
        case,
        PublishCase::ExchangeSuccess
            | PublishCase::ExchangeInvalid
            | PublishCase::BeforeCommitTargetReplacement
    ) {
        write_private(&target, ORIGINAL_BYTES);
    }
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let mut temporary = entry.temporary(STAGE_PREFIX, STAGE_SUFFIX).unwrap();
    temporary.file.write_all(STAGED_BYTES).unwrap();
    temporary.sync_and_inspect(&target, u64::MAX).unwrap();
    let stage = named_path(&target, &temporary.name);
    let saved = root.path().join(SAVED_NAME);
    let result = entry.rename_temporary_with_hooks(
        &temporary,
        u64::MAX,
        || {
            match case {
                PublishCase::BeforeCommitFailure => anyhow::bail!(BEFORE_COMMIT_ERROR),
                PublishCase::BeforeCommitStageReplacement => {
                    fs::rename(&stage, &saved).unwrap();
                    write_private(&stage, REPLACEMENT_BYTES);
                }
                PublishCase::BeforeCommitTargetReplacement => {
                    fs::rename(&target, &saved).unwrap();
                    write_private(&target, REPLACEMENT_BYTES);
                }
                _ => {}
            }
            Ok(())
        },
        || match case {
            PublishCase::ExchangeInvalid => {
                fs::rename(&target, &saved).unwrap();
                write_private(&target, REPLACEMENT_BYTES);
            }
            PublishCase::InstallRollback => fs::write(&target, REPLACEMENT_BYTES).unwrap(),
            PublishCase::InstallInvalid => {
                fs::rename(&target, &saved).unwrap();
                write_private(&target, REPLACEMENT_BYTES);
                write_private(&stage, ORIGINAL_BYTES);
            }
            _ => {}
        },
    );
    let expected_success = matches!(
        case,
        PublishCase::ExchangeSuccess | PublishCase::InstallSuccess
    );
    assert_eq!(result.is_ok(), expected_success, "{case:?}: {result:?}");
}

fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn named_path(target: &Path, name: &std::ffi::CStr) -> PathBuf {
    target.with_file_name(std::ffi::OsStr::from_bytes(name.to_bytes()))
}
