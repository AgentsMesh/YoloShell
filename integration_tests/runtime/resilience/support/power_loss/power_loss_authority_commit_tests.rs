use super::*;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use yolo_process_test_support::ProcessIdentity as RuntimeProcessIdentity;

const SESSION_NAME: &str = "power-loss-custody-authority";
const CUSTODY_REPLACEMENT_SUFFIX: &str = ".adversarial-replacement";
const DISPLACED_SESSION_SUFFIX: &str = ".retained";
const REPLACEMENT_SESSION_SUFFIX: &str = ".replacement";
const EXACT_GUARDIAN_COUNT_DETAIL: &str = "session did not expose exactly one Guardian";
const TARGET_NOT_FROZEN_DETAIL: &str = "authority failure allowed a frozen target to change";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn same_value_custody_replacement_fails_before_any_destructive_signal() {
    let mut harness = Harness::start().await;
    let terminal_id = harness.create_cat(SESSION_NAME).await.manifest.terminal_id;
    let paths = harness.paths();
    let record: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    let records = harness.bind_cleanup_records(std::slice::from_ref(&record));
    harness.kill_host_hard();
    let guardians = guardians::capture(
        &paths,
        &records,
        GuardianExpectation::RequiredOnSupportedPlatform,
    )
    .unwrap();
    let [guardian] = guardians.as_slice() else {
        panic!("{EXACT_GUARDIAN_COUNT_DETAIL}: {}", guardians.len());
    };
    let worker = runtime_worker(&records[0]);
    let child = runtime_child(&records[0]);
    let guardian_identity = guardian.direct_target();
    let mut barrier = barrier::PowerLossFreezeBarrier::capture(&paths, &records, guardians)
        .await
        .unwrap();
    barrier.freeze_all(&paths).await.unwrap();
    let custody_path = paths.pty_custody(terminal_id);
    let same_value: serde_json::Value = read_json(&custody_path).unwrap();

    let result = barrier
        .kill_all_after_session_confirm_for_test(
            &paths,
            &records,
            barrier::PowerLossCommitBoundary::RuntimeTree,
            || replace_same_value(&custody_path, &same_value),
        )
        .await;

    assert!(result.is_err());
    assert_frozen(worker);
    assert!(child.matches());
    barrier.ensure_all_frozen().await.unwrap();
    #[cfg(not(target_os = "linux"))]
    assert_frozen(child);
    if let Some(guardian_identity) = guardian_identity {
        assert_frozen(guardian_identity);
    }
}

#[tokio::test]
async fn same_value_session_directory_replacement_fails_before_any_destructive_signal() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let worker = TestProcess::spawn();
    let child = TestProcess::spawn();
    let record = fixture_record(&paths, worker.identity, &child);
    let bound = bind_record(&paths, record.clone());
    let records = std::slice::from_ref(&bound);
    let runtime_worker = runtime_worker(&bound);
    let runtime_child = runtime_child(&bound);
    let guardians = guardians::capture(
        &paths,
        records,
        GuardianExpectation::ExplicitlyAbsentFixture,
    )
    .unwrap();
    let mut barrier = barrier::PowerLossFreezeBarrier::capture(&paths, records, guardians)
        .await
        .unwrap();
    barrier.freeze_all(&paths).await.unwrap();
    let session = paths.session_dir(record.terminal_id);
    let displaced = paths
        .sessions_dir()
        .join(format!("{}{DISPLACED_SESSION_SUFFIX}", record.terminal_id));
    let replacement = paths
        .sessions_dir()
        .join(format!("{}{REPLACEMENT_SESSION_SUFFIX}", record.terminal_id));

    let result = barrier
        .kill_all_after_session_confirm_for_test(
            &paths,
            records,
            barrier::PowerLossCommitBoundary::RuntimeTree,
            || {
                std::fs::rename(&session, &displaced).unwrap();
                paths.ensure_session(record.terminal_id).unwrap();
                yolo_runtime::paths::atomic_write_json(
                    &paths.runtime_record(record.terminal_id),
                    &record,
                )
                .unwrap();
            },
        )
        .await;

    std::fs::rename(&session, replacement).unwrap();
    std::fs::rename(displaced, session).unwrap();
    assert!(result.is_err());
    assert_frozen(runtime_worker);
    assert!(runtime_child.matches());
    barrier.ensure_all_frozen().await.unwrap();
    #[cfg(not(target_os = "linux"))]
    assert_frozen(runtime_child);
}

#[tokio::test]
async fn runtime_replacement_after_direct_confirm_preserves_direct_target() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let worker = TestProcess::spawn();
    let child = TestProcess::spawn();
    let record = fixture_record(&paths, worker.identity, &child);
    let bound = bind_record(&paths, record.clone());
    let records = std::slice::from_ref(&bound);
    let runtime_worker = runtime_worker(&bound);
    let guardians = guardians::capture(
        &paths,
        records,
        GuardianExpectation::ExplicitlyAbsentFixture,
    )
    .unwrap();
    let mut barrier = barrier::PowerLossFreezeBarrier::capture(&paths, records, guardians)
        .await
        .unwrap();
    barrier.freeze_all(&paths).await.unwrap();

    let result = barrier
        .kill_all_after_session_confirm_for_test(
            &paths,
            records,
            barrier::PowerLossCommitBoundary::DirectTarget,
            || {
                yolo_runtime::paths::atomic_write_json(
                    &paths.runtime_record(record.terminal_id),
                    &record,
                )
                .unwrap();
            },
        )
        .await;

    assert!(result.is_err());
    assert_frozen(runtime_worker);
}

fn assert_frozen(identity: RuntimeProcessIdentity) {
    assert!(
        identity.matches(),
        "{TARGET_NOT_FROZEN_DETAIL}: {identity:?}"
    );
    assert!(
        identity.is_stably_stopped().unwrap(),
        "{TARGET_NOT_FROZEN_DETAIL}: {identity:?}"
    );
}

fn replace_same_value(path: &Path, value: &serde_json::Value) {
    let mut replacement_name = path.file_name().unwrap().to_os_string();
    replacement_name.push(CUSTODY_REPLACEMENT_SUFFIX);
    let replacement = path.with_file_name(replacement_name);
    let encoded = serde_json::to_vec_pretty(value).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(yolo_runtime::paths::PRIVATE_FILE_MODE)
        .open(&replacement)
        .unwrap();
    file.write_all(&encoded).unwrap();
    file.sync_all().unwrap();
    std::fs::rename(&replacement, path).unwrap();
    std::fs::File::open(path.parent().unwrap())
        .unwrap()
        .sync_all()
        .unwrap();
}
