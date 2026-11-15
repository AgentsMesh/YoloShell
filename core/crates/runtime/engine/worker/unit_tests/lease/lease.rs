use crate::worker::tests::*;
const EXITED_LEASE_OWNER: &str = "owner";

#[tokio::test]
async fn controller_identity_status_and_fencing_are_enforced() {
    let fixture = Fixture::new();
    let state = &fixture.state;
    assert!(state.validate_session(fixture.terminal_id()).is_ok());
    assert!(state.validate_session(Uuid::new_v4()).is_err());
    assert!(
        state
            .validate_controller(AuthenticatedController::new(1))
            .is_ok()
    );
    assert!(
        state
            .validate_controller(AuthenticatedController::new(2))
            .is_err()
    );
    state.adopt_controller(2).await.unwrap();
    assert_eq!(state.controller_generation.load(Ordering::Acquire), 2);
    assert!(
        state
            .validate_controller(AuthenticatedController::new(1))
            .is_err()
    );
    assert!(state.adopt_controller(1).await.is_err());
    assert!(state.paths.runtime_record(fixture.terminal_id()).exists());
    assert_eq!(state.status().status, TerminalRuntimeStatus::Running);
    *state.exit.lock().unwrap() = Some(ExitRecord {
        runtime_epoch: state.runtime_epoch(),
        code: Some(0),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: now_ms(),
        last_output_sequence: 0,
    });
    assert_eq!(state.status().status, TerminalRuntimeStatus::Exited);
}

#[tokio::test]
async fn writer_lease_conflict_renew_takeover_expiry_and_release() {
    let fixture = Fixture::new();
    let state = &fixture.state;
    let first_request = fixture.lease_request("first");
    let controller = fixture.controller();
    let first = state
        .acquire_lease(controller, &first_request, false)
        .await
        .unwrap();
    assert!(
        state
            .admit_writer_mutation(
                controller,
                first.generation,
                &first.writer_capability,
            )
            .await
            .is_ok()
    );
    assert!(
        state
            .acquire_lease(controller, &fixture.lease_request("second"), false)
            .await
            .is_err()
    );
    let same = state
        .acquire_lease(controller, &first_request, false)
        .await
        .unwrap();
    assert_eq!(same.generation, first.generation);

    let mut renewal = first_request.clone();
    renewal.generation = Some(first.generation);
    assert_eq!(
        state
            .renew_lease(controller, &renewal)
            .await
            .unwrap()
            .generation,
        first.generation
    );
    renewal.generation = Some(first.generation + 10);
    assert!(state.renew_lease(controller, &renewal).await.is_err());

    let mut second_request = fixture.lease_request("second");
    assert!(
        state
            .acquire_lease(controller, &second_request, true)
            .await
            .is_err()
    );
    second_request.generation = Some(first.generation);
    let second = state
        .acquire_lease(controller, &second_request, true)
        .await
        .unwrap();
    assert!(second.generation > first.generation);
    assert!(
        state
            .admit_writer_mutation(
                controller,
                first.generation,
                &first.writer_capability,
            )
            .await
            .is_err()
    );
    let mut wrong_release = fixture.lease_request("first");
    wrong_release.generation = Some(second.generation);
    assert!(
        state
            .release_lease(controller, &wrong_release)
            .await
            .is_err()
    );
    let mut release = second_request;
    release.generation = Some(second.generation);
    state.release_lease(controller, &release).await.unwrap();
    assert!(state.release_lease(controller, &release).await.is_err());

    let expiring = state
        .acquire_lease(controller, &first_request, false)
        .await
        .unwrap();
    state.lease.lock().unwrap().expire_for_test();
    assert_eq!(
        state
            .admit_writer_mutation(
                controller,
                expiring.generation,
                &expiring.writer_capability,
            )
            .await
            .err()
            .unwrap()
            .code,
        lease_error_code::LEASE_EXPIRED
    );
    let replacement = state
        .acquire_lease(controller, &fixture.lease_request("third"), false)
        .await
        .unwrap();
    assert!(replacement.generation > expiring.generation);
}

#[tokio::test]
async fn lease_mutations_reject_exited_sessions_with_typed_error() {
    let fixture = Fixture::new();
    let request = fixture.lease_request(EXITED_LEASE_OWNER);
    let grant = fixture
        .state
        .acquire_lease(fixture.controller(), &request, false)
        .await
        .unwrap();
    let mut release = request.clone();
    release.generation = Some(grant.generation);
    fixture.state.child_waited.store(true, Ordering::Release);
    let errors = [
        fixture
            .state
            .acquire_lease(fixture.controller(), &request, false)
            .await
            .unwrap_err(),
        fixture
            .state
            .get_writer_lease(fixture.controller())
            .await
            .unwrap_err(),
        fixture
            .state
            .release_lease(fixture.controller(), &release)
            .await
            .unwrap_err(),
    ];
    assert!(
        errors
            .iter()
            .all(|error| error.code == lease_error_code::SESSION_NOT_RUNNING)
    );
}
