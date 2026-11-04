use super::fixture::Fixture;

const NEXT_EVENT: u64 = 2;
const FENCE_FAILURE: &str = "injected native generation commit fence failure";

#[test]
fn first_event_registration_is_durable_and_ack_loss_replays_exactly() {
    let fixture = Fixture::new();
    let first = fixture.apply(&fixture.start).unwrap();
    let replay = fixture.apply(&fixture.start).unwrap();
    assert_eq!(first, replay);
    assert_eq!(
        fixture.store.load_in_namespace(&fixture.namespace).unwrap(),
        Some(first)
    );
}

#[test]
fn native_record_owns_binding_before_projection_and_projection_can_replay() {
    let fixture = Fixture::new();
    let first = fixture.apply(&fixture.start).unwrap();
    let binding_path = fixture
        .paths
        .agent_binding(fixture.start.scope.terminal_id);
    assert!(!binding_path.exists());
    for _ in 0..NEXT_EVENT {
        fixture
            .store
            .publish_binding_in_namespace(&fixture.namespace, &first, || Ok(()))
            .unwrap();
    }
    let binding: crate::agent::AgentBinding = fixture.namespace.read_json(&binding_path).unwrap();
    assert_eq!(binding, first.recovery_binding(true));
}

#[test]
fn failed_first_commit_leaves_no_registered_invocation() {
    let fixture = Fixture::new();
    assert!(
        fixture
            .store
            .apply_in_namespace(
                &fixture.namespace,
                &fixture.start,
                fixture.worker,
                &fixture.boot_id,
                &fixture.evidence,
                || anyhow::bail!(FENCE_FAILURE)
            )
            .is_err()
    );
    assert!(
        fixture
            .store
            .load_in_namespace(&fixture.namespace)
            .unwrap()
            .is_none()
    );
}

#[test]
fn stale_worker_and_epoch_cannot_mutate_the_original_invocation() {
    let fixture = Fixture::new();
    let first = fixture.apply(&fixture.start).unwrap();
    let request = fixture.state_event(NEXT_EVENT);
    let mut worker = fixture.worker;
    worker.start_time += NEXT_EVENT;
    assert!(
        fixture
            .store
            .apply_in_namespace(
                &fixture.namespace,
                &request,
                worker,
                &fixture.boot_id,
                &fixture.evidence,
                || Ok(())
            )
            .is_err()
    );
    let mut foreign = request;
    foreign.scope.runtime_epoch = uuid::Uuid::new_v4();
    assert!(fixture.apply(&foreign).is_err());
    assert_eq!(
        fixture.store.load_in_namespace(&fixture.namespace).unwrap(),
        Some(first)
    );
}

#[test]
fn explicit_revocation_is_durable_and_rejects_callback_replay() {
    let fixture = Fixture::new();
    let first = fixture.apply(&fixture.start).unwrap();
    let revoked = fixture
        .store
        .revoke_in_namespace(&fixture.namespace, &first, || Ok(()))
        .unwrap();
    assert!(revoked.revoked);
    assert!(fixture.apply(&fixture.start).is_err());
    assert!(!revoked.verify_resume_evidence().unwrap());
}
