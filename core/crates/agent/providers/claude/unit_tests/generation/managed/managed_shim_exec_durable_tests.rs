use super::super::SessionStartSource;
use super::managed_shim_exec_publication_test_support::Case;
use super::observed_unmanaged_publication_test_support::{ABSOLUTE_NODE, CLAUDE_ENTRYPOINT};
use crate::agent::{AgentRecoveryAvailability, AgentState};

const IDENTITY_INCREMENT: i64 = 1;

#[test]
fn durable_publication_promotes_exact_shim_predecessors_to_canonical_bindings() {
    let cases = [
        Case::native(AgentState::Running),
        Case::native(AgentState::WaitingForInput),
        Case::new(
            AgentState::Running,
            ABSOLUTE_NODE,
            vec![ABSOLUTE_NODE.to_owned(), CLAUDE_ENTRYPOINT.to_owned()],
        ),
        Case::resumed(AgentState::Running, SessionStartSource::Resume),
        Case::resumed(AgentState::Running, SessionStartSource::Fork),
    ];
    for case in cases {
        let (committed, binding) = case.publish(&case.authority).unwrap();
        assert_eq!(
            binding.executable,
            crate::agent::claude::constants::EXECUTABLE
        );
        assert_eq!(binding.state, case.authority.state);
        assert_eq!(
            binding.recovery_availability,
            AgentRecoveryAvailability::Available
        );
        assert_eq!(committed.binding_revision, Some(committed.revision));
        assert!(committed.matches_binding(&binding));
        assert_eq!(case.binding_at(), binding);
        assert_committed_publication(&case, &committed);
    }
}

#[test]
fn replay_after_full_commit_is_idempotent() {
    let case = Case::native(AgentState::Running);
    let first = case.publish(&case.authority).unwrap();
    let second = case.publish_with_force(&first.0, true).unwrap();
    assert_eq!(second, first);
    assert_eq!(case.binding_at(), first.1);
    assert_committed_publication(&case, &second.0);
}

#[test]
fn retry_after_binding_write_converges_without_reusing_special_admission() {
    let case = Case::native(AgentState::Running);
    let target = case.authority.binding(Some(&case.predecessor)).unwrap();
    case.write_binding(&target);

    let (committed, binding) = case.publish_with_force(&case.authority, true).unwrap();

    assert_eq!(binding, target);
    assert_eq!(committed.binding_revision, Some(committed.revision));
    assert!(committed.matches_binding(&binding));
    assert_committed_publication(&case, &committed);
}

#[test]
fn force_replay_before_binding_write_can_consume_the_special_admission() {
    let case = Case::native(AgentState::Running);
    let (committed, binding) = case.publish_with_force(&case.authority, true).unwrap();
    assert_eq!(committed.binding_revision, Some(committed.revision));
    assert!(committed.matches_binding(&binding));
    assert_committed_publication(&case, &committed);
}

#[test]
fn expired_retry_after_binding_write_uses_the_ordinary_canonical_path() {
    let case = Case::expired_handshake(AgentState::Running);
    let target = case.authority.binding(Some(&case.predecessor)).unwrap();
    case.write_binding(&target);

    let (committed, binding) = case.publish_with_force(&case.authority, true).unwrap();

    assert_eq!(binding, target);
    assert_eq!(committed.binding_revision, Some(committed.revision));
    assert_committed_publication(&case, &committed);
}

#[test]
fn published_authority_cannot_consume_a_replanted_shim_predecessor() {
    let case = Case::native(AgentState::Running);
    let committed = case.publish(&case.authority).unwrap().0;
    case.write_binding(&case.predecessor);
    let authority_before = case
        .fixture
        .store
        .load_in_namespace(&case.fixture.lease)
        .unwrap();

    assert!(case.publish(&committed).is_err());
    assert_eq!(case.binding_at(), case.predecessor);
    assert_eq!(
        case.fixture
            .store
            .load_in_namespace(&case.fixture.lease)
            .unwrap(),
        authority_before
    );
}

#[test]
fn predecessor_replacement_after_capture_fails_without_durable_mutation() {
    let case = Case::native(AgentState::Running);
    let admission = case.admission(&case.authority, true).unwrap();
    let mut replacement = case.predecessor.clone();
    replacement.updated_at_ms += IDENTITY_INCREMENT;
    case.write_binding(&replacement);
    let authority_before = case
        .fixture
        .store
        .load_in_namespace(&case.fixture.lease)
        .unwrap();

    assert!(
        case.fixture
            .store
            .publish_hook_binding_state_in_namespace(
                &case.fixture.lease,
                &case.fixture.binding_path(),
                case.fixture.authentication(),
                &case.authority,
                false,
                &admission,
                |_| Ok(()),
            )
            .is_err()
    );
    assert_eq!(case.binding_at(), replacement);
    assert_eq!(
        case.fixture
            .store
            .load_in_namespace(&case.fixture.lease)
            .unwrap(),
        authority_before
    );
}

fn assert_committed_publication(case: &Case, authority: &super::super::ClaudeGenerationAuthority) {
    let set = case
        .fixture
        .store
        .load_set(&case.fixture.lease)
        .unwrap()
        .unwrap();
    let publication = set.binding_publication.unwrap();
    assert_eq!(publication.invocation_id, authority.invocation_id);
    assert_eq!(
        publication.publication_generation,
        set.publication_generation
    );
    assert_eq!(publication.authority_revision, authority.revision);
    assert!(set.pending_binding_transition.is_none());
}
