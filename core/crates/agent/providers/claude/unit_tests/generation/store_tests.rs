use super::super::SessionStartSource;
use super::support::*;
use crate::agent::{AgentBindingStore, AgentKind, AgentState, DetectionContext};
use std::fs;

const DOWNSTREAM_CLAUDE_EXECUTABLE: &str = "/resolved/downstream/claude";

#[test]
fn begin_is_durable_nonrecoverable_and_fences_previous_authentication() {
    let mut fixture = Fixture::new();
    assert!(
        fixture
            .store
            .load_in_namespace(&fixture.lease)
            .unwrap()
            .is_none()
    );
    let first = fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    assert_eq!(first.generation, 1);
    assert_eq!(first.revision, 1);
    assert_eq!(first.state, AgentState::Unknown);
    assert!(first.transcript_path.is_none());
    assert_path_exists(fixture.store.path());
    let stale_authentication = fixture.authentication();

    fixture.begin.runtime_epoch = uuid::Uuid::new_v4();
    fixture.begin.invocation_id = uuid::Uuid::new_v4();
    fixture.begin.hook_token = uuid::Uuid::new_v4();
    fixture.begin.provider_session_id = session_id();
    let second = fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    assert_eq!(second.generation, 1);
    assert_eq!(second.revision, 1);
    assert_eq!(
        fixture
            .store
            .publication_generation_in_namespace(&fixture.lease)
            .unwrap(),
        2
    );
    assert!(
        fixture
            .store
            .apply_in_namespace(
                &fixture.lease,
                stale_authentication,
                &fixture.start(&second.provider_session_id, SessionStartSource::Fresh),
            )
            .is_err()
    );
}

#[test]
fn stores_in_separate_terminal_namespaces_do_not_cross_contaminate_same_cwd() {
    let left = Fixture::new();
    let mut right = Fixture::new();
    right.begin.cwd = left.begin.cwd.clone();
    left.store
        .begin_in_namespace(&left.lease, &left.begin)
        .unwrap();
    right
        .store
        .begin_in_namespace(&right.lease, &right.begin)
        .unwrap();
    let left_id = left.begin.provider_session_id.clone();
    let right_id = right.begin.provider_session_id.clone();
    let left_record = left
        .store
        .apply_in_namespace(
            &left.lease,
            left.authentication(),
            &left.start(&left_id, SessionStartSource::Fresh),
        )
        .unwrap()
        .0;
    let right_record = right
        .store
        .apply_in_namespace(
            &right.lease,
            right.authentication(),
            &right.start(&right_id, SessionStartSource::Fresh),
        )
        .unwrap()
        .0;
    assert_ne!(left_record.terminal_id, right_record.terminal_id);
    assert_ne!(left_record.provider_session_id, right_record.provider_session_id);
    assert_eq!(left_record.cwd, right_record.cwd);
}

#[test]
fn binding_recovery_requires_matching_runtime_and_process_authority() {
    let fixture = Fixture::new();
    let session_id = fixture.begin.provider_session_id.clone();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let authority = fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &fixture.start(&session_id, SessionStartSource::Fresh),
        )
        .unwrap()
        .0;
    let transcript = authority.transcript_path.as_ref().unwrap();
    fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    fs::write(
        transcript,
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{}\",\"cwd\":{}}}\n",
            authority.provider_session_id,
            serde_json::to_string(&authority.cwd).unwrap()
        ),
    )
    .unwrap();
    let binding = binding(&authority);
    let store = AgentBindingStore::new(fixture.binding_path());
    store
        .save_authoritative_in_namespace(&fixture.lease, &binding, &authority, fixture.runtime())
        .unwrap();
    let context = DetectionContext::new(
        authority.terminal_id,
        authority.cwd.clone(),
        [(AgentKind::ClaudeCode, authority.data_home.clone())],
    );
    let recovered = store
        .load_authoritative_in_namespace(&fixture.lease, &context, fixture.runtime())
        .unwrap()
        .unwrap();
    assert_eq!(recovered.state, AgentState::Running);

    let mut incomplete_scan = binding.clone();
    incomplete_scan.state = AgentState::Unknown;
    let downgraded_unknown = store
        .reconcile_detected_in_namespace(&fixture.lease, &incomplete_scan, fixture.runtime())
        .unwrap();
    assert_eq!(downgraded_unknown.state, AgentState::Unknown);
    assert_eq!(
        downgraded_unknown.provider_session_id,
        binding.provider_session_id
    );
    assert_eq!(downgraded_unknown.evidence_path, binding.evidence_path);

    incomplete_scan.state = AgentState::WaitingForInput;
    let downgraded_mismatch = store
        .reconcile_detected_in_namespace(&fixture.lease, &incomplete_scan, fixture.runtime())
        .unwrap();
    assert_eq!(downgraded_mismatch.state, AgentState::Unknown);
    assert_eq!(
        downgraded_mismatch.provider_session_id,
        binding.provider_session_id
    );

    incomplete_scan.executable = DOWNSTREAM_CLAUDE_EXECUTABLE.to_owned();
    let downgraded_downstream = store
        .reconcile_detected_in_namespace(&fixture.lease, &incomplete_scan, fixture.runtime())
        .unwrap();
    assert_eq!(downgraded_downstream.state, AgentState::Unknown);

    incomplete_scan.process_start_time = Some(PROCESS_START + 1);
    let rejected_scan = store
        .reconcile_detected_in_namespace(&fixture.lease, &incomplete_scan, fixture.runtime())
        .unwrap();
    assert_eq!(rejected_scan.state, AgentState::Unknown);

    let mut stale_runtime = fixture.runtime();
    stale_runtime.worker.start_time += 1;
    let _worker_alive = crate::process::install_test_process_identity_relation(
        WORKER_PID,
        WORKER_START,
        crate::process::ProcessIdentityRelation::Matching,
    );
    let _agent_alive = crate::process::install_test_process_identity_relation(
        PROCESS_PID,
        PROCESS_START,
        crate::process::ProcessIdentityRelation::Matching,
    );
    let rejected = store
        .load_authoritative_in_namespace(&fixture.lease, &context, stale_runtime)
        .unwrap()
        .unwrap();
    assert_eq!(rejected.state, AgentState::Unknown);
}
