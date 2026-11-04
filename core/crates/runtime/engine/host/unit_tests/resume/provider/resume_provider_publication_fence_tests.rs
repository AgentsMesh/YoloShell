use crate::host::tests::*;
const INVALID_PROVIDER_BINDING: &[u8] = b"{";

#[test]
fn provider_snapshot_defers_while_a_lifecycle_publication_is_active() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let binding = fixture.resumed_binding();
    fixture.publish_binding(&binding);
    let holder = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire(
        &fixture.host.paths,
        &fixture.host.namespace,
        fixture.transaction.terminal_id,
    )
    .unwrap();

    assert_eq!(
        fixture
            .host
            .observe_resume_provider_binding(
                &fixture.manifest,
                &fixture.transaction,
                &fixture.runtime,
            )
            .unwrap(),
        ProviderBindingObservation::Pending
    );
    drop(holder);
    assert_eq!(
        fixture
            .host
            .observe_resume_provider_binding(
                &fixture.manifest,
                &fixture.transaction,
                &fixture.runtime,
            )
            .unwrap(),
        ProviderBindingObservation::Candidate(binding)
    );
}

#[test]
fn provider_snapshot_does_not_hide_an_invalid_binding_replacement() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    fixture.publish_binding(&fixture.resumed_binding());
    let holder = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire(
        &fixture.host.paths,
        &fixture.host.namespace,
        fixture.transaction.terminal_id,
    )
    .unwrap();
    std::fs::write(fixture.binding_path(), INVALID_PROVIDER_BINDING).unwrap();

    assert_eq!(
        fixture
            .host
            .observe_resume_provider_binding(
                &fixture.manifest,
                &fixture.transaction,
                &fixture.runtime,
            )
            .unwrap(),
        ProviderBindingObservation::Pending
    );
    drop(holder);
    assert!(
        fixture
            .host
            .observe_resume_provider_binding(
                &fixture.manifest,
                &fixture.transaction,
                &fixture.runtime,
            )
            .is_err()
    );
}

#[test]
fn provider_settlement_defers_while_a_lifecycle_publication_is_active() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let binding = fixture.resumed_binding();
    fixture.publish_binding(&binding);
    let holder = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire(
        &fixture.host.paths,
        &fixture.host.namespace,
        fixture.transaction.terminal_id,
    )
    .unwrap();
    let mut accepted_runtime = fixture.runtime.clone();

    assert!(
        fixture
            .host
            .settle_verified_provider_candidate(
                &fixture.manifest,
                &fixture.transaction,
                &mut accepted_runtime,
                &binding,
                binding.updated_at_ms,
            )
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .host
            .load_resume_transaction(fixture.transaction.terminal_id)
            .unwrap()
            .unwrap()
            .provider_completion
            .is_none()
    );
    drop(holder);
    assert!(
        fixture
            .host
            .settle_verified_provider_candidate(
                &fixture.manifest,
                &fixture.transaction,
                &mut accepted_runtime,
                &binding,
                binding.updated_at_ms,
            )
            .unwrap()
            .is_some()
    );
}
