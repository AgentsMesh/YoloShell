use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};
use yolo_runtime::paths::atomic_write_json;

#[path = "../test_support/dimensions.rs"]
mod fixture;
use fixture::Fixture;

const READ_STEP: usize = 1;
const FIRST_READ: usize = 1;
const COMPLETED_READS: usize = 2;
const PERMISSION_DENIED_MODE: u32 = 0o000;
const CORRUPT: &[u8] = b"not-runtime-json";
const REPLACEMENT_AUTH: &str = "replaced-authentication";
const CHANGED_DIRECTORY: &str = "replaced-directory";
const SAVED_RUNTIME: &str = "saved-runtime.json";
const SAVED_SESSION: &str = "saved-session";
const SHORT_BUDGET: std::time::Duration = std::time::Duration::from_millis(10);

#[tokio::test]
async fn version_conflict_is_retried_only_until_the_same_runtime_has_the_requested_size() {
    let mut fixture = Fixture::new();
    let mut calls = usize::default();
    fixture
        .subject
        .wait_with(
            fixture.size,
            Instant::now() + super::super::RELAY_TIMEOUT,
            |entry| {
                calls += READ_STEP;
                if calls == FIRST_READ {
                    let error = Fixture::conflicted_read(entry, &fixture.resized).unwrap_err();
                    assert!(error.downcast_ref::<ReadVersionChanged>().is_some());
                    return Err(error);
                }
                read_entry_guarded(entry.reopen()?, || Ok(()))
            },
        )
        .await
        .unwrap();
    assert_eq!(calls, COMPLETED_READS);
    assert_eq!(fixture.subject.expected, fixture.resized);
}

#[tokio::test]
async fn a_version_conflict_cannot_hide_replaced_runtime_authority() {
    let mutations: &[fn(&mut RuntimeRecord)] = &[
        |record| record.runtime_epoch = uuid::Uuid::new_v4(),
        |record| record.worker_id = uuid::Uuid::new_v4(),
        |record| record.auth_token = REPLACEMENT_AUTH.into(),
        |record| record.cwd.push(CHANGED_DIRECTORY),
        |record| record.controller_generation = u64::default(),
    ];
    for mutate in mutations {
        let mut fixture = Fixture::new();
        mutate(&mut fixture.resized);
        let mut calls = usize::default();
        let result = fixture
            .subject
            .wait_with(
                fixture.size,
                Instant::now() + super::super::RELAY_TIMEOUT,
                |entry| {
                    calls += READ_STEP;
                    if calls == FIRST_READ {
                        return Fixture::conflicted_read(entry, &fixture.resized);
                    }
                    read_entry_guarded(entry.reopen()?, || Ok(()))
                },
            )
            .await;
        assert!(result.is_err());
        assert_eq!(calls, COMPLETED_READS);
    }
}

#[tokio::test]
async fn missing_corrupt_unsafe_or_replaced_namespace_never_becomes_pending() {
    for fault in [
        Fault::Missing,
        Fault::Corrupt,
        Fault::Permissions,
        Fault::Symlink,
        Fault::Namespace,
    ] {
        let mut fixture = Fixture::new();
        let path = &fixture.path;
        match fault {
            Fault::Missing => std::fs::remove_file(path).unwrap(),
            Fault::Corrupt => std::fs::write(path, CORRUPT).unwrap(),
            Fault::Permissions => std::fs::set_permissions(
                path,
                std::fs::Permissions::from_mode(PERMISSION_DENIED_MODE),
            )
            .unwrap(),
            Fault::Symlink => {
                let saved = path.with_file_name(SAVED_RUNTIME);
                std::fs::rename(path, &saved).unwrap();
                symlink(saved, path).unwrap();
            }
            Fault::Namespace => {
                let session = path.parent().unwrap();
                let saved = session.with_file_name(SAVED_SESSION);
                std::fs::rename(session, saved).unwrap();
                atomic_write_json(path, &fixture.resized).unwrap();
            }
        }
        let mut calls = usize::default();
        let error = fixture
            .subject
            .wait_with(
                fixture.size,
                Instant::now() + super::super::RELAY_TIMEOUT,
                |entry| {
                    calls += READ_STEP;
                    read_entry_guarded(entry.reopen()?, || Ok(()))
                },
            )
            .await
            .unwrap_err();
        assert_eq!(calls, FIRST_READ);
        assert!(error.downcast_ref::<ReadVersionChanged>().is_none());
    }
}

#[tokio::test]
async fn worker_or_child_identity_loss_is_rejected_before_reading_dimensions() {
    for worker in [false, true] {
        let mut fixture = Fixture::new();
        if worker {
            fixture.subject.expected.worker_start_time += READ_STEP as u64;
        } else {
            fixture.subject.expected.child_start_time += READ_STEP as u64;
        }
        let result = fixture
            .subject
            .wait_with(
                fixture.size,
                Instant::now() + super::super::RELAY_TIMEOUT,
                |_| panic!("an exited or replaced process cannot authorize another read"),
            )
            .await;
        assert_eq!(result.unwrap_err().to_string(), PROCESS_CHANGED);
    }
}

#[tokio::test]
async fn exhausted_deadline_does_not_read_or_accept_a_late_ready_result() {
    let mut fixture = Fixture::new();
    let result = fixture
        .subject
        .wait_with(fixture.size, Instant::now(), |_| panic!())
        .await;
    assert_eq!(result.unwrap_err().to_string(), DIMENSIONS_TIMEOUT);
    let deadline = Instant::now() + SHORT_BUDGET;
    let result = fixture
        .subject
        .wait_with(fixture.size, deadline, |_| {
            std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
            Ok(fixture.resized.clone())
        })
        .await;
    assert_eq!(result.unwrap_err().to_string(), DIMENSIONS_TIMEOUT);
}

enum Fault {
    Missing,
    Corrupt,
    Permissions,
    Symlink,
    Namespace,
}
