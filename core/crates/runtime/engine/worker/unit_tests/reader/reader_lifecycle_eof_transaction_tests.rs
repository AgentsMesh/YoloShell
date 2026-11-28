use crate::worker::pty_output_progress::KernelPtyEofIdentity;

fn retained_eof_fixture() -> (ReaderLifecycle, std::sync::Arc<KernelPtyEofIdentity>) {
    let lifecycle = ReaderLifecycle::default();
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .kernel_eof_for_test()
        .await_eof_publication();
    drop(publication);
    let identity = lifecycle.kernel_eof_identity().unwrap();
    (lifecycle, identity)
}

#[test]
fn rejected_transaction_preserves_the_same_retained_eof() {
    let (lifecycle, identity) = retained_eof_fixture();

    assert!(
        lifecycle
            .with_validated_current_kernel_eof(
                &identity,
                || Ok(true),
                || Ok(KernelEofTransactionDisposition::Preserve),
            )
            .unwrap()
    );

    let current = lifecycle.kernel_eof_identity().unwrap();
    assert!(std::sync::Arc::ptr_eq(&identity, &current));
    assert!(lifecycle.lock_progress().eof.is_retained());
}

#[test]
fn committed_transaction_completes_without_a_second_kernel_eof() {
    let (lifecycle, identity) = retained_eof_fixture();
    lifecycle.mark_supervisor_completed_for_test();
    assert!(!lifecycle.master_aliases_can_close());

    assert!(
        lifecycle
            .with_validated_current_kernel_eof(
                &identity,
                || Ok(true),
                || Ok(KernelEofTransactionDisposition::Complete),
            )
            .unwrap()
    );

    let progress = lifecycle.lock_progress();
    assert!(progress.eof.is_observed());
    assert!(!progress.output_read_in_flight);
    drop(progress);
    assert!(lifecycle.master_aliases_can_close());
}

#[test]
fn rejected_probe_is_promoted_to_a_retained_eof() {
    let lifecycle = ReaderLifecycle::default();
    let identity = lifecycle
        .acquire_kernel_eof_probe(|| Ok(true))
        .unwrap()
        .unwrap();

    assert!(
        lifecycle
            .with_validated_current_kernel_eof_probe(
                &identity,
                || Ok(true),
                || Ok(KernelEofTransactionDisposition::Preserve),
            )
            .unwrap()
    );

    let current = lifecycle.kernel_eof_identity().unwrap();
    assert!(std::sync::Arc::ptr_eq(&identity, &current));
    let progress = lifecycle.lock_progress();
    assert!(progress.kernel_eof_probe_identity.is_none());
    assert!(progress.eof.is_retained());
}

#[test]
fn failed_probe_transaction_is_promoted_before_retry() {
    const FAILURE_DETAIL: &str = "injected probe transaction failure";
    let lifecycle = ReaderLifecycle::default();
    let identity = lifecycle
        .acquire_kernel_eof_probe(|| Ok(true))
        .unwrap()
        .unwrap();

    let result = lifecycle.with_validated_current_kernel_eof_probe(
        &identity,
        || Ok(true),
        || Err(anyhow::anyhow!(FAILURE_DETAIL)),
    );

    assert!(result.is_err());
    let current = lifecycle.kernel_eof_identity().unwrap();
    assert!(std::sync::Arc::ptr_eq(&identity, &current));
}
