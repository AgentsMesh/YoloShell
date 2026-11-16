use crate::worker::tests::*;
#[tokio::test]
async fn newer_controller_owns_authority_and_upgrades_lost_reason() {
    let fixture = Fixture::new();
    let first = fixture.controller();
    let second = fixture
        .state
        .adopt_controller(CUSTODY_SUCCESSOR_GENERATION)
        .await
        .unwrap();
    assert!(
        fixture
            .state
            .admit_controller_mutation(first)
            .await
            .is_err()
    );
    let second_permit = fixture
        .state
        .admit_controller_mutation(second)
        .await
        .unwrap();
    fixture
        .state
        .begin_controller_termination(&second_permit, ExitReason::UserTerminate);
    drop(second_permit);
    let second_permit = fixture
        .state
        .admit_controller_mutation(second)
        .await
        .unwrap();
    fixture
        .state
        .begin_controller_termination(&second_permit, ExitReason::Lost);
    drop(second_permit);
    assert_eq!(fixture.state.termination_reason(), Some(ExitReason::Lost));
    assert_eq!(
        fixture.state.termination_authority(),
        Some(TerminationAuthority::Controller(second))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ownership_loss_upgrades_reason_and_authority() {
    let scenario = FinalizerProcessScenario::AuthorityUpgrade;
    if scenario.is_child() {
        ownership_loss_upgrades_reason_and_authority_inner().await;
    } else {
        FinalizerTestProcess::run(scenario);
    }
}

async fn ownership_loss_upgrades_reason_and_authority_inner() {
    let fixture = Fixture::new();
    let controller = fixture.controller();
    let controller_permit = fixture
        .state
        .admit_controller_mutation(controller)
        .await
        .unwrap();
    fixture
        .state
        .begin_controller_termination(&controller_permit, ExitReason::UserTerminate);
    drop(controller_permit);
    let system_permit = fixture.state.admit_system_mutation().await;
    fixture
        .state
        .begin_system_termination(&system_permit, ExitReason::Lost);
    drop(system_permit);
    assert_eq!(fixture.state.termination_reason(), Some(ExitReason::Lost));
    assert_eq!(
        fixture.state.termination_authority(),
        Some(TerminationAuthority::System)
    );
    let controller_permit = fixture
        .state
        .admit_controller_mutation(controller)
        .await
        .unwrap();
    fixture
        .state
        .begin_controller_termination(&controller_permit, ExitReason::Lost);
    drop(controller_permit);
    assert_eq!(
        fixture.state.termination_authority(),
        Some(TerminationAuthority::System)
    );

    let adoption = fixture
        .state
        .adopt_controller(CUSTODY_SUCCESSOR_GENERATION)
        .await
        .unwrap_err();
    assert_eq!(
        adoption.message,
        secondary_custody_promotion::SUCCESSOR_TERMINATION_COMMITTED_DETAIL
    );
    let notified = fixture.state.shutdown.notified();
    let notify_state = Arc::clone(&fixture.state);
    tokio::task::spawn_blocking(move || notify_state.notify_termination_shutdown_blocking())
        .await
        .unwrap();
    tokio::time::timeout(CONTROL_BARRIER_TIMEOUT, notified)
        .await
        .unwrap();
}
