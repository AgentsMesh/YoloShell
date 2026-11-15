use super::*;

#[tokio::test]
async fn wrong_writer_capability_generation_and_expiry_never_reserve_or_signal() {
    let source = prepared();
    let fixture = &source.fixture;
    let request = interrupt(&source).await;
    let mut wrong = request.clone();
    wrong.writer_capability = WriterCapability::generate();
    assert!(
        fixture
            .state
            .interrupt_native_agent(fixture.controller(), &wrong)
            .await
            .is_err()
    );
    let mut generation = request.clone();
    generation.generation += STEP;
    assert!(
        fixture
            .state
            .interrupt_native_agent(fixture.controller(), &generation)
            .await
            .is_err()
    );
    assert_unreserved(&source, &request).await;
    lock_recover(&fixture.state.lease).expire_for_test();
    let error = fixture
        .state
        .interrupt_native_agent(fixture.controller(), &request)
        .await
        .unwrap_err();
    assert_eq!(error.code, lease_error_code::LEASE_EXPIRED);
    assert_unreserved(&source, &request).await;
}

#[tokio::test]
async fn taken_over_writer_and_old_controller_cannot_issue_a_native_interrupt() {
    let source = prepared();
    let fixture = &source.fixture;
    let old_controller = fixture.controller();
    let request = interrupt(&source).await;
    let mut replacement = fixture.lease_request(REPLACEMENT_WRITER);
    replacement.generation = Some(request.generation);
    fixture
        .state
        .acquire_lease(old_controller, &replacement, true)
        .await
        .unwrap();
    assert!(
        fixture
            .state
            .interrupt_native_agent(old_controller, &request)
            .await
            .is_err()
    );
    assert_unreserved(&source, &request).await;
    fixture
        .state
        .adopt_controller(old_controller.generation.0 + STEP)
        .await
        .unwrap();
    assert!(
        fixture
            .state
            .interrupt_native_agent(old_controller, &request)
            .await
            .is_err()
    );
    assert!(fixture.state.native_control_available());
    assert_unreserved(&source, &request).await;
}

#[tokio::test]
async fn nil_request_wrong_runtime_and_foreign_worker_are_rejected_before_reservation() {
    let source = prepared();
    let fixture = &source.fixture;
    let request = interrupt(&source).await;
    let mut nil = request.clone();
    nil.request_id = Uuid::nil();
    assert!(
        fixture
            .state
            .interrupt_native_agent(fixture.controller(), &nil)
            .await
            .is_err()
    );
    let mut wrong_runtime = request.clone();
    wrong_runtime.runtime_epoch = Uuid::new_v4();
    assert!(
        fixture
            .state
            .interrupt_native_agent(fixture.controller(), &wrong_runtime)
            .await
            .is_err()
    );
    let mut fields = request.witness.fields().clone();
    fields.worker_id = Uuid::new_v4();
    let mut wrong_worker = request.clone();
    wrong_worker.witness = NativeControlWitness::new(fields).unwrap();
    assert!(
        fixture
            .state
            .interrupt_native_agent(fixture.controller(), &wrong_worker)
            .await
            .is_err()
    );
    assert_unreserved(&source, &request).await;
}
