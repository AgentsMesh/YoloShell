use super::*;
use yolo_runtime::paths::socket_namespace::NamespaceRelease;

const HARNESS_CLEANUP_SETTLEMENT_DETAIL: &str =
    "runtime harness cleanup must settle detached Guardian custody";
const CORRUPT_RUNTIME_CONTENT: &[u8] = b"corrupt-runtime-record";
const EXPECTED_TEST_PANIC: &str = "exercise panic-safe Harness cleanup";
const UNKNOWN_PANIC_DETAIL: &str = "non-string Harness cleanup panic";
const RUNTIME_RECORD_READ_CONTEXT: &str = "read runtime cleanup record";
const ROOT_CAUSE_OWNERSHIP_DETAIL: &str = "corrupt runtime root cause must be reported once";
const CORRUPT_FIXTURE_SOCKET_RETIRED_DETAIL: &str =
    "corrupt fixture must release its retained socket namespace after process settlement";
const EXPECTED_ROOT_CAUSE_COUNT: usize = 1;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn harness_cleanup_survives_concurrent_corruption_panics_and_stopped_children() {
    let corrupt_panic = tokio::spawn(assert_harness_cleanup(
        RuntimeRecordDamage::Corrupt,
        true,
        true,
    ));
    let deleted = tokio::spawn(assert_harness_cleanup(
        RuntimeRecordDamage::Deleted,
        true,
        false,
    ));
    corrupt_panic.await.unwrap();
    deleted.await.unwrap();
    assert_harness_cleanup(RuntimeRecordDamage::Corrupt, true, false).await;
}

#[derive(Clone, Copy)]
enum RuntimeRecordDamage {
    Deleted,
    Corrupt,
}

async fn assert_harness_cleanup(
    damage: RuntimeRecordDamage,
    ignore_term: bool,
    panic_during_scope: bool,
) {
    let harness = Harness::start().await;
    let session = harness.create_cat("cleanup-adversarial").await;
    let terminal_id = session.manifest.terminal_id;
    let record: RuntimeRecord = read_json(&harness.paths().runtime_record(terminal_id)).unwrap();
    let target = runtime_ref_from_record(&record);
    let worker = ProcessIdentity {
        pid: record.worker_pid,
        start_time: record.worker_start_time,
    };
    let child = ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    };
    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    let (guardian, guardian_custody) =
        active_guardian(&harness, terminal_id, record.controller_generation).await;
    let mut cleanup = OrphanCleanup::processes([worker, child]);
    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    cleanup.track(guardian);
    if ignore_term {
        prepare_term_ignoring_child(&harness, target).await;
    }
    let runtime_path = harness.paths().runtime_record(terminal_id);
    let cleanup_failure_expected = matches!(damage, RuntimeRecordDamage::Corrupt);
    // Expected corruption preserves the durable home and its diagnostic. Retain
    // registered endpoint authority now so the fixture can settle its transport later.
    let socket_namespace = cleanup_failure_expected
        .then(|| harness.paths().retain_socket_namespace().unwrap());
    match damage {
        RuntimeRecordDamage::Deleted => std::fs::remove_file(&runtime_path).unwrap(),
        RuntimeRecordDamage::Corrupt => {
            std::fs::write(&runtime_path, CORRUPT_RUNTIME_CONTENT).unwrap();
        }
    }
    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if panic_during_scope {
            let _harness = harness;
            panic!("{EXPECTED_TEST_PANIC}");
        }
        drop(harness);
    }));
    assert_eq!(
        unwind.is_err(),
        panic_during_scope || cleanup_failure_expected
    );
    if cleanup_failure_expected && !panic_during_scope {
        let detail = panic_detail(unwind.as_ref().unwrap_err().as_ref());
        assert!(detail.contains(&runtime_path.display().to_string()));
        assert_eq!(
            detail.matches(RUNTIME_RECORD_READ_CONTEXT).count(),
            EXPECTED_ROOT_CAUSE_COUNT,
            "{ROOT_CAUSE_OWNERSHIP_DETAIL}: {detail}"
        );
    }
    let settled = tokio::time::timeout(
        GUARDIAN_NAMESPACE_LOSS_SETTLEMENT_TIMEOUT,
        wait_for_orphan_settlement(
            worker,
            child,
            #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
            guardian,
            #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
            guardian_custody,
        ),
    )
    .await;
    assert!(
        settled.is_ok(),
        "{HARNESS_CLEANUP_SETTLEMENT_DETAIL}: worker={:?}, child={:?}, guardian={:?}",
        worker.observe(),
        child.observe(),
        guardian_diagnostic(
            #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
            guardian.observe(),
        ),
    );
    drop(cleanup);
    if let Some(namespace) = socket_namespace {
        let directory = namespace.directory().unwrap();
        assert_eq!(
            namespace.release().unwrap(),
            NamespaceRelease::Removed,
            "{CORRUPT_FIXTURE_SOCKET_RETIRED_DETAIL}"
        );
        assert_eq!(
            std::fs::symlink_metadata(directory.path()).unwrap_err().kind(),
            std::io::ErrorKind::NotFound,
            "{CORRUPT_FIXTURE_SOCKET_RETIRED_DETAIL}"
        );
        assert_eq!(std::fs::read(&runtime_path).unwrap(), CORRUPT_RUNTIME_CONTENT);
    }
}

fn panic_detail(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(detail) = payload.downcast_ref::<String>() {
        return detail.clone();
    }
    payload
        .downcast_ref::<&str>()
        .copied()
        .unwrap_or(UNKNOWN_PANIC_DETAIL)
        .to_owned()
}
