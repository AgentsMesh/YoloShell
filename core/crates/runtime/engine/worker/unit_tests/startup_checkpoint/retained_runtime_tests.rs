use super::retirement_test_support::*;
use super::*;
use crate::model::{BootstrapDelivery, RuntimeRecord};
use crate::worker::startup::startup_checkpoint::constants::PREDECESSOR_RUNTIME_FIELD;

#[test]
fn retired_successor_authorizes_only_the_exact_retained_runtime() {
    let fixture = CheckpointFixture::new();
    let predecessor = activated_runtime(&fixture);
    let successor = successor_for(&fixture);
    assert_eq!(
        prepare(&fixture, &successor, Some(&predecessor)).unwrap(),
        WorkerStartupCheckpointPreparation::Prepared
    );
    assert!(fixture.read().predecessor_runtime_sha256.is_some());
    assert!(
        worker_startup_checkpoint_authorizes_retained_runtime(
            &fixture.paths,
            &successor,
            &predecessor,
        )
        .unwrap()
    );

    let mutations: &[fn(&mut RuntimeRecord)] = &[
        |value| value.terminal_id = Uuid::new_v4(),
        |value| value.runtime_epoch = Uuid::new_v4(),
        |value| value.worker_id = Uuid::new_v4(),
        |value| value.worker_pid = TEST_CHILD_PID_PRIMARY,
        |value| value.worker_start_time += STALE_IDENTITY_OFFSET,
        |value| value.child_pid = TEST_CHILD_PID_FALLBACK,
        |value| value.child_start_time += STALE_IDENTITY_OFFSET,
        |value| value.boot_id = TEST_OTHER_BOOT_ID.into(),
        |value| value.socket_path = TEST_CWD.into(),
        |value| value.auth_token = TEST_ENVIRONMENT_VALUE.into(),
        |value| value.controller_generation += STALE_IDENTITY_OFFSET,
        |value| value.started_at_ms += i64::try_from(STALE_IDENTITY_OFFSET).unwrap(),
        |value| value.command = TEST_CWD.into(),
        |value| value.args.clear(),
        |value| value.cwd = TEST_COMMAND.into(),
        |value| value.cols = value.rows,
        |value| value.rows = value.cols,
        |value| value.bootstrap_delivery = BootstrapDelivery::Failed,
    ];
    for mutate in mutations {
        let mut changed = predecessor.clone();
        mutate(&mut changed);
        assert!(
            !worker_startup_checkpoint_authorizes_retained_runtime(
                &fixture.paths,
                &successor,
                &changed,
            )
            .unwrap()
        );
        assert!(prepare(&fixture, &successor, Some(&changed)).is_err());
    }
    let mut malformed = predecessor;
    malformed.schema_version += UNSUPPORTED_SCHEMA_VERSION_OFFSET;
    assert!(
        worker_startup_checkpoint_authorizes_retained_runtime(
            &fixture.paths,
            &successor,
            &malformed,
        )
        .is_err()
    );
}

#[test]
fn retained_runtime_digest_is_bounded_and_does_not_disclose_runtime_fields() {
    let fixture = CheckpointFixture::new();
    let predecessor = activated_runtime(&fixture);
    let successor = successor_for(&fixture);
    prepare(&fixture, &successor, Some(&predecessor)).unwrap();
    let bytes = std::fs::read(fixture.path()).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value.get(PREDECESSOR_RUNTIME_FIELD).is_some());
    assert!(
        bytes.len()
            <= usize::try_from(crate::paths::constants::WORKER_STARTUP_CHECKPOINT_MAX_BYTES)
                .unwrap()
    );
    for secret in [
        predecessor.auth_token.as_bytes(),
        predecessor.command.as_bytes(),
        predecessor.cwd.as_os_str().as_encoded_bytes(),
    ] {
        assert!(!bytes.windows(secret.len()).any(|window| window == secret));
    }
}

#[test]
fn retired_successor_without_a_digest_cannot_authorize_a_retained_runtime() {
    let fixture = CheckpointFixture::new();
    let predecessor = activated_runtime(&fixture);
    let successor = successor_for(&fixture);
    prepare(&fixture, &successor, Some(&predecessor)).unwrap();
    let mut incomplete = fixture.read();
    incomplete.predecessor_runtime_sha256 = None;
    crate::paths::atomic_write_json(&fixture.path(), &incomplete).unwrap();

    assert!(
        !worker_startup_checkpoint_authorizes_retained_runtime(
            &fixture.paths,
            &successor,
            &predecessor,
        )
        .unwrap()
    );
    assert!(prepare(&fixture, &successor, Some(&predecessor)).is_err());
}
