use super::native_test_support::Fixture;
use crate::{AgentKind, AgentRecoveryAvailability, AgentState};

#[test]
fn existing_projection_repair_does_not_reacquire_its_own_target_lock() {
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind);
        fixture.store.save(&fixture.binding).unwrap();
        let guard =
            crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
                &fixture.paths,
                &fixture.namespace,
                fixture.binding.terminal_id,
            )
            .unwrap();
        let repaired = fixture
            .store
            .repair_native_binding_in_namespace_with_commit_fence(
                &fixture.namespace,
                &fixture.context,
                fixture.runtime,
                || guard.require_no_pending_intent(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(repaired.state, AgentState::Unknown);
        assert_eq!(
            repaired.recovery_availability,
            AgentRecoveryAvailability::Available
        );
        assert_eq!(
            fixture
                .store
                .read_raw_in_namespace(&fixture.namespace)
                .unwrap(),
            Some(repaired)
        );
    }
}

#[test]
fn process_observation_reconciliation_preserves_native_checkpoint_without_recursive_lock() {
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind);
        fixture.store.save(&fixture.binding).unwrap();
        let mut observed = fixture.binding.clone();
        observed.state = AgentState::Unknown;
        observed.recovery_availability = AgentRecoveryAvailability::Unavailable;
        let binding = fixture
            .store
            .reconcile_detected_with_commit_fence(
                &fixture.namespace,
                &observed,
                fixture.runtime,
                || Ok(()),
            )
            .unwrap();
        assert_eq!(binding.state, AgentState::Unknown);
        assert_eq!(
            binding.recovery_availability,
            AgentRecoveryAvailability::Available
        );
    }
}
