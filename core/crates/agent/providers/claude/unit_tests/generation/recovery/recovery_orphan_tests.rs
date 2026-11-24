use super::recovery_orphan_test_support::ParkedRecoveryFixture;

#[test]
fn dead_parked_pending_queue_is_folded_acknowledged_and_retired() {
    let fixture = ParkedRecoveryFixture::new();
    let _guards = fixture.gone_guards();
    fixture.fill_outbox_capacity();
    let publication = fixture
        .base
        .store
        .publication_generation_in_namespace(&fixture.base.lease)
        .unwrap();

    let loaded = fixture.load().unwrap().unwrap();
    assert_eq!(loaded, fixture.active_binding());
    assert_eq!(
        fixture
            .base
            .store
            .load_in_namespace(&fixture.base.lease)
            .unwrap(),
        Some(fixture.pair.second.clone())
    );
    assert!(
        fixture
            .base
            .store
            .load_invocation_in_namespace(&fixture.base.lease, fixture.pair.first.invocation_id,)
            .unwrap()
            .is_none()
    );
    assert!(
        crate::claude_hooks::outbox::pending_recovery(&fixture.outbox, fixture.credentials,)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fixture
            .base
            .store
            .publication_generation_in_namespace(&fixture.base.lease)
            .unwrap(),
        publication
    );
    fixture.reuse_outbox_capacity();
}

#[test]
fn crash_after_parked_ack_retires_on_reentry_without_touching_active_publication() {
    let fixture = ParkedRecoveryFixture::new();
    let _guards = fixture.gone_guards();
    fixture.acknowledge_without_retire();
    let publication = fixture
        .base
        .store
        .publication_generation_in_namespace(&fixture.base.lease)
        .unwrap();

    let loaded = fixture.load().unwrap().unwrap();
    assert_eq!(loaded, fixture.active_binding());
    assert_eq!(
        fixture
            .base
            .store
            .load_in_namespace(&fixture.base.lease)
            .unwrap(),
        Some(fixture.pair.second.clone())
    );
    assert!(
        fixture
            .base
            .store
            .load_invocation_in_namespace(&fixture.base.lease, fixture.pair.first.invocation_id,)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fixture
            .base
            .store
            .publication_generation_in_namespace(&fixture.base.lease)
            .unwrap(),
        publication
    );
}

#[test]
fn unavailable_parked_evidence_does_not_invalidate_the_active_binding() {
    let fixture = ParkedRecoveryFixture::new();
    let _guards = fixture.gone_guards();
    let parked_transcript = fixture.pair.first.transcript_path.as_ref().unwrap();
    std::fs::write(parked_transcript, []).unwrap();

    assert_eq!(fixture.load().unwrap(), Some(fixture.active_binding()));
    assert!(
        fixture
            .base
            .store
            .load_invocation_in_namespace(&fixture.base.lease, fixture.pair.first.invocation_id,)
            .unwrap()
            .is_some()
    );
    assert!(
        crate::claude_hooks::outbox::pending_recovery(&fixture.outbox, fixture.credentials)
            .unwrap()
            .is_some()
    );
}
