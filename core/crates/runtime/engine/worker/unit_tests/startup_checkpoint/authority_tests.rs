use super::authority_test_support::*;
use super::*;
const INJECTED_FAILURE: &str = "injected predecessor publication failure";
const NON_CANONICAL_TERMINAL_DIMENSION: u16 = 0;
const PENDING_INTENT: &[u8] = b"pending-runtime-publication";
const PENDING_INTENT_MAX_BYTES: u64 = PENDING_INTENT.len() as u64;

#[test]
fn retained_predecessor_is_replaced_once_then_allows_normal_updates() {
    let mut state = AuthorityFixture::new();
    assert!(state.checkpoint.predecessor_authority().is_some());
    state.publish().unwrap();
    assert!(state.checkpoint.predecessor_authority().is_none());
    assert_eq!(read_runtime(&state), state.replacement);

    let mut update = state.replacement.clone();
    update.bootstrap_delivery = crate::model::BootstrapDelivery::Delivered;
    state
        .fixture
        .namespace
        .remove_file(
            &state
                .fixture
                .paths
                .worker_launch(state.fixture.launch.terminal_id),
        )
        .unwrap();
    crate::worker::publish_runtime_record(&state.fixture.namespace, &state.fixture.paths, &update)
        .unwrap();
    assert_eq!(read_runtime(&state), update);

    let mut unrelated = update;
    unrelated.worker_id = Uuid::new_v4();
    assert!(
        crate::worker::publish_runtime_record(
            &state.fixture.namespace,
            &state.fixture.paths,
            &unrelated,
        )
        .is_err()
    );
}

#[test]
fn normal_updates_reject_every_runtime_authority_regression() {
    let mut state = AuthorityFixture::new();
    state.publish().unwrap();
    let mut current = state.replacement.clone();
    current.bootstrap_delivery = crate::model::BootstrapDelivery::Delivered;
    crate::worker::publish_runtime_record(&state.fixture.namespace, &state.fixture.paths, &current)
        .unwrap();
    let before = state.runtime_bytes();
    let mutations: [fn(&mut crate::model::RuntimeRecord); 5] = [
        |value| value.worker_id = Uuid::new_v4(),
        |value| value.started_at_ms += STALE_IDENTITY_OFFSET as i64,
        |value| value.controller_generation = current_generation_predecessor(value),
        |value| value.cols = NON_CANONICAL_TERMINAL_DIMENSION,
        |value| value.bootstrap_delivery = crate::model::BootstrapDelivery::Failed,
    ];
    for mutate in mutations {
        let mut candidate = current.clone();
        mutate(&mut candidate);
        assert!(
            crate::worker::publish_runtime_record(
                &state.fixture.namespace,
                &state.fixture.paths,
                &candidate,
            )
            .is_err()
        );
        assert_eq!(state.runtime_bytes(), before);
    }
}

fn current_generation_predecessor(value: &crate::model::RuntimeRecord) -> u64 {
    value.controller_generation.saturating_sub(1)
}

#[test]
fn failed_attempt_preserves_authority_and_exact_predecessor_for_retry() {
    let mut state = AuthorityFixture::new();
    let before = state.runtime_bytes();
    assert!(publish_with(&mut state, || anyhow::bail!(INJECTED_FAILURE)).is_err());
    assert!(state.checkpoint.predecessor_authority().is_some());
    assert_eq!(state.runtime_bytes(), before);

    state.publish().unwrap();
    assert!(state.checkpoint.predecessor_authority().is_none());
    assert_eq!(read_runtime(&state), state.replacement);
}

#[test]
fn checkpoint_identity_and_runtime_digest_tampering_fail_closed() {
    let mut checkpoint_changed = AuthorityFixture::new();
    let evidence = checkpoint_changed.fixture.read();
    crate::paths::atomic_write_json(&checkpoint_changed.fixture.path(), &evidence).unwrap();
    let runtime_before = checkpoint_changed.runtime_bytes();
    assert!(checkpoint_changed.publish().is_err());
    assert_eq!(checkpoint_changed.runtime_bytes(), runtime_before);

    let mut runtime_changed = AuthorityFixture::new();
    let mut predecessor = runtime_changed.predecessor.clone();
    predecessor.started_at_ms += STALE_IDENTITY_OFFSET as i64;
    crate::paths::atomic_write_json(
        &runtime_changed
            .fixture
            .paths
            .runtime_record(predecessor.terminal_id),
        &predecessor,
    )
    .unwrap();
    let changed = runtime_changed.runtime_bytes();
    assert!(runtime_changed.publish().is_err());
    assert_eq!(runtime_changed.runtime_bytes(), changed);
}

#[test]
fn runtime_cas_race_and_pending_intent_preserve_disk_and_authority() {
    let mut raced = AuthorityFixture::new();
    let mut competing = raced.predecessor.clone();
    competing.started_at_ms += STALE_IDENTITY_OFFSET as i64;
    let runtime_path = raced
        .fixture
        .paths
        .runtime_record(raced.fixture.launch.terminal_id);
    assert!(
        publish_with(&mut raced, || {
            crate::paths::atomic_write_json(&runtime_path, &competing)
        })
        .is_err()
    );
    assert_eq!(read_runtime(&raced), competing);
    assert!(raced.checkpoint.predecessor_authority().is_some());

    let mut pending = AuthorityFixture::new();
    let before = pending.runtime_bytes();
    publish_pending_intent(&pending);
    assert!(pending.publish().is_err());
    assert_eq!(pending.runtime_bytes(), before);
    assert!(pending.checkpoint.predecessor_authority().is_some());
}

#[test]
fn pending_intent_at_runtime_cas_boundary_preserves_predecessor() {
    let mut state = AuthorityFixture::new();
    let before = state.runtime_bytes();
    let intent = state
        .fixture
        .paths
        .reboot_migration_intent(state.fixture.launch.terminal_id);
    assert!(
        publish_with(&mut state, || {
            crate::paths::private_file::PrivateFileEntry::open(&intent)?
                .create_bytes_exclusive(PENDING_INTENT, PENDING_INTENT_MAX_BYTES)
                .map(|_| ())
        })
        .is_err()
    );
    assert_eq!(state.runtime_bytes(), before);
    assert!(state.checkpoint.predecessor_authority().is_some());
}

fn publish_with<F>(state: &mut AuthorityFixture, hook: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()>,
{
    crate::worker::runtime_record_publication::publish_initial_with_test_hook(
        &state.fixture.namespace,
        &state.fixture.paths,
        &state.replacement,
        &mut state.checkpoint,
        hook,
    )
}

fn publish_pending_intent(state: &AuthorityFixture) {
    state
        .fixture
        .namespace
        .file(
            &state
                .fixture
                .paths
                .reboot_migration_intent(state.fixture.launch.terminal_id),
        )
        .unwrap()
        .create_bytes_exclusive(PENDING_INTENT, PENDING_INTENT_MAX_BYTES)
        .unwrap();
}

fn read_runtime(state: &AuthorityFixture) -> crate::model::RuntimeRecord {
    crate::paths::read_json(
        &state
            .fixture
            .paths
            .runtime_record(state.fixture.launch.terminal_id),
    )
    .unwrap()
}
