use super::invocation_set_tests::parked_pair;
use super::support::{Fixture, binding};
use crate::agent::claude::generation::GenerationApplyDisposition;
use crate::agent::{AgentBindingStore, AgentKind, DetectionContext, ObservedProcess};
use std::collections::{BTreeMap, VecDeque};

const SHELL_PID: u32 = 7;
const FOREGROUND_GROUP: i32 = 11;

#[test]
fn foreground_flip_before_marker_prevents_stale_monitor_publication() {
    let fixture = Fixture::new();
    let pair = parked_pair(&fixture);
    let store = AgentBindingStore::new(fixture.binding_path());
    let context = context(&fixture);
    let runtime = fixture.runtime();
    let second_binding = binding(&pair.second);
    store
        .save_authoritative_in_namespace(&fixture.lease, &second_binding, &pair.second, runtime)
        .unwrap();
    let first_process = process(&pair.first);
    let second_process = process(&pair.second);
    let mut proofs = VecDeque::from([
        vec![first_process.clone()],
        vec![first_process.clone()],
        vec![second_process],
    ]);
    assert!(
        store
            .reactivate_claude_foreground_in_namespace(
                &fixture.lease,
                &context,
                runtime,
                &[first_process],
                || Ok(proofs.pop_front().unwrap()),
            )
            .is_err()
    );
    let (second, disposition, reactivated) = fixture
        .store
        .transition_foreground_in_namespace(
            &fixture.lease,
            pair.second_authentication,
            &pair.second,
            pair.second.last_event_sequence,
            &pair.second_event,
            pair.second.transcript_identity,
        )
        .unwrap();
    assert_eq!(disposition, GenerationApplyDisposition::Replay);
    assert!(reactivated);
    let pending = fixture
        .store
        .begin_binding_publication_in_namespace(&fixture.lease, pair.second_authentication, &second)
        .unwrap();
    let second_binding = binding(&pending);
    store
        .save_authoritative_in_namespace(&fixture.lease, &second_binding, &pending, runtime)
        .unwrap();
    let committed = fixture
        .store
        .commit_binding_publication_in_namespace(
            &fixture.lease,
            pair.second_authentication,
            &pending,
        )
        .unwrap();
    let loaded = store
        .load_authoritative_in_namespace(&fixture.lease, &context, runtime)
        .unwrap()
        .unwrap();
    assert_eq!(loaded.provider_session_id, committed.provider_session_id);
    assert_eq!(loaded.process_pid, Some(committed.process.pid));
}

fn context(fixture: &Fixture) -> DetectionContext {
    DetectionContext::new(
        fixture.begin.terminal_id,
        fixture.begin.cwd.clone(),
        [(AgentKind::ClaudeCode, fixture.begin.data_home.clone())],
    )
}

fn process(
    authority: &crate::agent::claude::generation::ClaudeGenerationAuthority,
) -> ObservedProcess {
    ObservedProcess {
        pid: authority.process.pid,
        start_time: authority.process.start_time,
        parent_pid: SHELL_PID,
        process_group: FOREGROUND_GROUP,
        foreground_group: FOREGROUND_GROUP,
        executable: crate::agent::claude::constants::EXECUTABLE.to_owned(),
        argv: vec![crate::agent::claude::constants::EXECUTABLE.to_owned()],
        environment: Some(BTreeMap::new()),
        cwd: Some(authority.cwd.clone()),
        evidence_paths: None,
    }
}
