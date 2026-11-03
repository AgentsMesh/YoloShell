use super::{authority_test_support::AuthorityFixture, retirement_test_support::*, *};

const CUSTODY_DIGEST_BYTE: u8 = 0x5a;
const FOREIGN_IDENTITY_OFFSET: u64 = 1;
const HOLDER_PID: u32 = 40_003;
const HOLDER_START: u64 = 50_003;
const TEST_READER_HANDLE: u64 = 60_003;
const TEST_WRITER_HANDLE: u64 = 60_004;

#[test]
fn predecessor_source_is_replaced_and_successor_pending_is_idempotent() {
    let state = published_rollover_fixture();
    let successor_pending = pending(&state.replacement);
    let installed = publish_pending(&state).unwrap();
    assert_eq!(installed.0, successor_pending);
    assert_eq!(read_custody(&state), successor_pending);
    let retried = publish_pending(&state).unwrap();
    assert_eq!(retried.0, successor_pending);
}

#[test]
fn preinstalled_successor_pending_is_retried_exactly() {
    let state = published_rollover_fixture();
    let successor_pending = pending(&state.replacement);
    write_custody(&state, &successor_pending);
    let retried = publish_pending(&state).unwrap();
    assert_eq!(retried.0, successor_pending);
    assert_eq!(read_custody(&state), successor_pending);
}

#[test]
fn predecessor_custody_change_before_runtime_publication_fails_closed() {
    let mutations: [fn(&mut AuthorityFixture); 4] = [
        arm_successor,
        remove_custody,
        |state| {
            let mut foreign = read_custody(state);
            foreign.runtime.child_start_time += FOREIGN_IDENTITY_OFFSET;
            write_custody(state, &foreign)
        },
        |state| {
            let source = read_custody(state);
            write_custody(state, &source)
        },
    ];
    for mutate in mutations {
        let mut state = rollover_fixture();
        mutate(&mut state);
        assert!(state.publish().is_err());
        assert_eq!(read_runtime(&state), state.predecessor);
    }
}

#[test]
fn installed_pending_cannot_become_armed_deleted_or_foreign() {
    let mutations: [fn(&mut AuthorityFixture); 3] = [arm_successor, remove_custody, |state| {
        let mut foreign = pending(&state.replacement);
        foreign.runtime.child_start_time += FOREIGN_IDENTITY_OFFSET;
        write_custody(state, &foreign)
    }];
    for mutate in mutations {
        let mut state = published_rollover_fixture();
        write_custody(&state, &pending(&state.replacement));
        mutate(&mut state);
        assert!(publish_pending(&state).is_err());
    }
}

fn publish_pending(
    state: &AuthorityFixture,
) -> anyhow::Result<(
    crate::pty_custody::CustodyDurableRecord,
    crate::paths::identity::FileIdentity,
)> {
    super::super::super::secondary_custody_persistence::publish_pending(
        &state.fixture.namespace,
        &state.fixture.paths,
        &state.replacement,
        &state.checkpoint,
    )
}

#[test]
fn consecutive_rollovers_capture_the_last_installed_pending() {
    let state = published_rollover_fixture();
    let successor_pending = pending(&state.replacement);
    write_custody(&state, &successor_pending);
    let mut next = state.successor.clone();
    next.runtime_epoch = Uuid::new_v4();
    next.worker_id = Uuid::new_v4();
    prepare(&state.fixture, &next, Some(&state.replacement)).unwrap();
    let proof = state.fixture.read().pending_pty_custody_rollover.unwrap();
    let (_, _, identity): (crate::pty_custody::CustodyDurableRecord, _, _) = state
        .fixture
        .namespace
        .lease()
        .read_json_with_versions(&state.fixture.paths.pty_custody(next.terminal_id))
        .unwrap();
    assert_eq!(proof.source_version, identity);
    assert_eq!(
        proof.source_sha256,
        crate::pty_custody::durable_record_digest(&successor_pending).unwrap()
    );
}

fn rollover_fixture() -> AuthorityFixture {
    AuthorityFixture::new_with_predecessor_custody(gone_process(), gone_process(), armed)
}

pub(super) fn published_rollover_fixture() -> AuthorityFixture {
    let mut state = rollover_fixture();
    state.publish().unwrap();
    state
        .checkpoint
        .complete(
            &state.fixture.namespace,
            &state.fixture.paths,
            StartupStage::RuntimeRecord,
        )
        .unwrap();
    state
        .checkpoint
        .enter(
            &state.fixture.namespace,
            &state.fixture.paths,
            StartupStage::SecondaryCustody,
        )
        .unwrap();
    state
}

pub(super) fn pending(
    runtime: &crate::model::RuntimeRecord,
) -> crate::pty_custody::CustodyDurableRecord {
    crate::pty_custody::CustodyDurableRecord::pending(
        crate::pty_custody::CustodyRuntimeIdentity::from_record(runtime),
    )
}

fn armed(runtime: &crate::model::RuntimeRecord) -> crate::pty_custody::CustodyDurableRecord {
    let pending = pending(runtime);
    let request = crate::pty_custody::CustodyRequest::new(
        crate::pty_custody::CustodyLeaseId::new(),
        pending.runtime.clone(),
        crate::pty_custody::CustodyHolderIdentity {
            pid: HOLDER_PID,
            start_time: HOLDER_START,
        },
        runtime.auth_token.clone(),
        runtime.controller_generation,
    );
    let offer = crate::pty_custody::test_support::custody_offer(
        &request,
        crate::pty_custody::CustodyDescriptorSet::macos_process_capability(
            TEST_READER_HANDLE,
            TEST_WRITER_HANDLE,
        ),
        crate::sha256_digest::Sha256Digest::repeated(CUSTODY_DIGEST_BYTE),
    );
    crate::pty_custody::arm_fixture(&pending, &request, &offer).unwrap()
}

fn arm_successor(state: &mut AuthorityFixture) {
    write_custody(state, &armed(&state.replacement));
}

fn remove_custody(state: &mut AuthorityFixture) {
    let path = state
        .fixture
        .paths
        .pty_custody(state.replacement.terminal_id);
    state.fixture.namespace.remove_file(&path).unwrap();
}

pub(super) fn write_custody(
    state: &AuthorityFixture,
    value: &crate::pty_custody::CustodyDurableRecord,
) {
    let path = state
        .fixture
        .paths
        .pty_custody(state.fixture.launch.terminal_id);
    crate::paths::atomic_write_json(&path, value).unwrap();
}

fn read_custody(state: &AuthorityFixture) -> crate::pty_custody::CustodyDurableRecord {
    let path = state
        .fixture
        .paths
        .pty_custody(state.replacement.terminal_id);
    crate::paths::read_json(&path).unwrap()
}

fn read_runtime(state: &AuthorityFixture) -> crate::model::RuntimeRecord {
    let fixture = &state.fixture;
    let path = fixture.paths.runtime_record(state.replacement.terminal_id);
    crate::paths::read_json(&path).unwrap()
}
