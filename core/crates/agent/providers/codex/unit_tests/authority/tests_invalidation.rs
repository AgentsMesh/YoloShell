use super::*;

#[test]
fn active_authority_is_invalidated_with_its_exact_provider_identity() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();

    let unavailable = fixture.invalidate(fixture.runtime).unwrap().unwrap();

    assert_eq!(unavailable.status, CodexAuthorityStatus::Unavailable);
    assert_eq!(unavailable.binding.state, AgentState::Unknown);
    assert_eq!(
        unavailable.binding.recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
    assert_eq!(unavailable.binding.process_pid, Some(PROVIDER_PID));
    assert_eq!(unavailable.binding.process_start_time, Some(PROVIDER_START));
}

#[test]
fn confirmed_exit_is_not_revoked_by_worker_finalization() {
    let fixture = Fixture::new();
    let mut exited = fixture.verified_authority(Generation::INITIAL.0);
    exited.binding.state = AgentState::Exited;
    exited.binding.process_pid = None;
    exited.binding.process_start_time = None;
    fixture.publish(&exited).unwrap();

    let retained = fixture.invalidate(fixture.runtime).unwrap().unwrap();

    assert_eq!(retained, exited);
    assert_eq!(
        retained.binding.recovery_availability,
        AgentRecoveryAvailability::Available
    );
}

#[test]
fn foreign_runtime_is_ignored_and_terminal_invalidation_is_idempotent() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();
    let foreign = AuthorityRuntimeIdentity {
        runtime_epoch: Uuid::new_v4(),
        worker: fixture.runtime.worker,
    };
    assert!(fixture.invalidate(foreign).unwrap().is_none());
    fixture.invalidate(fixture.runtime).unwrap();
    let repaired = fixture.invalidate(fixture.runtime).unwrap().unwrap();
    assert_eq!(repaired.status, CodexAuthorityStatus::Unavailable);
    assert_eq!(
        fixture
            .namespace
            .read_json::<AgentBinding>(&fixture.binding_path)
            .unwrap(),
        repaired.binding
    );
}

#[test]
fn stale_invalidation_cannot_overwrite_a_successor_authority_or_binding() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();
    let authority_path = fixture
        .binding_path
        .parent()
        .unwrap()
        .join(AUTHORITY_FILE_NAME);
    let (loaded, identity) = fixture
        .namespace
        .read_json_with_identity::<CodexBindingAuthority>(&authority_path)
        .unwrap();
    let successor = fixture.authority(Generation::INITIAL.0 + 1, CodexAuthorityStatus::Active);
    fixture.publish(&successor).unwrap();

    assert!(
        store
            .invalidate_loaded_in_namespace_with_commit_fence(
                &fixture.namespace,
                &fixture.binding_path,
                loaded,
                identity,
                fixture.runtime,
                || Ok(()),
            )
            .is_err()
    );
    assert_eq!(
        store.load_in_namespace(&fixture.namespace).unwrap(),
        Some(successor.clone())
    );
    assert_eq!(
        fixture
            .namespace
            .read_json::<AgentBinding>(&fixture.binding_path)
            .unwrap(),
        successor.binding
    );
}
