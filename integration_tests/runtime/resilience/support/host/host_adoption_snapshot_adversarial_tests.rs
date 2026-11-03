use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};
use yolo_process_test_support::ProcessIdentityObservation;

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const MOVED_SESSION: &str = "moved-session";
const MOVED_RUNTIME: &str = "moved-runtime.json";
const CORRUPT_RUNTIME_BYTES: &[u8] = b"not-runtime-json";
const HARD_LINK_RUNTIME: &str = "runtime-alias.json";

#[tokio::test]
async fn authoritative_identity_replacement_during_the_query_fails_immediately() {
    let mutations: &[fn(&mut RuntimeRecord)] = &[
        |record| record.schema_version += 1,
        |record| record.terminal_id = Uuid::new_v4(),
        |record| record.runtime_epoch = Uuid::new_v4(),
        |record| record.worker_id = Uuid::new_v4(),
        |record| record.worker_pid += 1,
        |record| record.worker_start_time += 1,
        |record| record.child_pid += 1,
        |record| record.child_start_time += 1,
        |record| record.boot_id.push_str(REPLACEMENT_SUFFIX),
        |record| record.auth_token.push_str(REPLACEMENT_SUFFIX),
        |record| record.socket_path.push(REPLACEMENT_SUFFIX),
        |record| record.started_at_ms += 1,
        |record| record.command.push_str(REPLACEMENT_SUFFIX),
        |record| record.args.push(REPLACEMENT_SUFFIX.into()),
        |record| record.cwd.push(REPLACEMENT_SUFFIX),
        |record| record.controller_generation = FOREIGN_GENERATION,
        |record| record.controller_generation = SUCCESSOR_GENERATION - 1,
        |record| record.rows = u16::default(),
        |record| record.bootstrap_delivery = yolo_runtime::model::BootstrapDelivery::Pending,
    ];
    for mutate in mutations {
        let fixture = SnapshotFixture::new();
        let mut changed = fixture.changed_dimensions();
        mutate(&mut changed);
        let error = inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
            fixture.publish(&changed);
            Ok(fixture.view())
        })
        .await
        .unwrap_err();
        assert!(error.contains(ADOPTION_RECORD_CHANGED_DETAIL), "{error}");
    }
}

#[tokio::test]
async fn failed_query_cannot_hide_a_runtime_replacement() {
    let fixture = SnapshotFixture::new();
    let mut changed = fixture.record.clone();
    changed.runtime_epoch = Uuid::new_v4();
    let error = inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
        fixture.publish(&changed);
        Err(SnapshotFixture::query_error())
    })
    .await
    .unwrap_err();
    assert!(error.contains(ADOPTION_RECORD_CHANGED_DETAIL));
}

#[tokio::test]
async fn valid_publication_does_not_hide_a_forged_runtime_view() {
    let fixture = SnapshotFixture::new();
    let mut forged = fixture.view();
    forged.runtime.as_mut().unwrap().worker_id = Uuid::new_v4();
    let error = inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
        fixture.publish(&fixture.changed_dimensions());
        Ok(forged)
    })
    .await
    .unwrap_err();
    assert!(error.contains(ADOPTION_VIEW_CHANGED_DETAIL));
}

#[tokio::test]
async fn replacing_the_retained_namespace_cannot_rebind_identical_record_bytes() {
    let fixture = SnapshotFixture::new();
    let session = fixture.paths.session_dir(fixture.record.terminal_id);
    let moved = fixture.directory.path().join(MOVED_SESSION);
    let error = inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
        std::fs::rename(&session, &moved).unwrap();
        std::fs::create_dir(&session).unwrap();
        std::fs::set_permissions(
            &session,
            std::fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
        )
        .unwrap();
        fixture.publish(&fixture.record);
        Ok(fixture.view())
    })
    .await
    .unwrap_err();
    assert!(error.contains(ADOPTION_RECORD_CHANGED_DETAIL));
}

#[tokio::test]
async fn runtime_symlink_replacement_is_not_publication_progress() {
    let fixture = SnapshotFixture::new();
    let runtime = fixture.paths.runtime_record(fixture.record.terminal_id);
    let moved = fixture.directory.path().join(MOVED_RUNTIME);
    let error = inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || async {
        std::fs::rename(&runtime, &moved).unwrap();
        symlink(&moved, &runtime).unwrap();
        Ok(fixture.view())
    })
    .await
    .unwrap_err();
    assert!(error.contains(ADOPTION_RECORD_CHANGED_DETAIL));
}

#[test]
fn final_observation_keeps_the_retained_file_fence() {
    let fixture = SnapshotFixture::new();
    let observed = fixture.expected.observe_current().unwrap();
    let error = evaluate_observed_runtime(
        &fixture.expected,
        &observed,
        Some(&fixture.view()),
        SUCCESSOR_GENERATION,
        || {
            fixture.publish(&fixture.record);
            ProcessIdentityObservation::Matching
        },
    )
    .unwrap_err();
    assert!(error.contains(ADOPTION_RECORD_CHANGED_DETAIL));
}

#[tokio::test]
async fn invalid_retained_runtime_reads_fail_before_the_view_query() {
    for hard_link in [false, true] {
        let fixture = SnapshotFixture::new();
        let path = fixture.paths.runtime_record(fixture.record.terminal_id);
        if hard_link {
            std::fs::hard_link(&path, fixture.directory.path().join(HARD_LINK_RUNTIME)).unwrap();
        } else {
            std::fs::write(&path, CORRUPT_RUNTIME_BYTES).unwrap();
        }
        let queried = std::cell::Cell::new(false);
        let error = inspect_runtime(&fixture.expected, SUCCESSOR_GENERATION, || {
            queried.set(true);
            std::future::ready(Ok(fixture.view()))
        })
        .await
        .unwrap_err();
        assert!(error.contains(ADOPTION_RECORD_CHANGED_DETAIL));
        assert!(!queried.get());
    }
}
