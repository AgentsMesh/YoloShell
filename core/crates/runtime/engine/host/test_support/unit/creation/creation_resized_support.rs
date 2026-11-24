const CREATION_RESIZE_STEP: u16 = 1;
const RESIZED_CREATION_COLUMNS: u16 = CREATION_TEST_COLUMNS + CREATION_RESIZE_STEP;
const RESIZED_CREATION_ROWS: u16 = CREATION_TEST_ROWS + CREATION_RESIZE_STEP;

async fn resized_ready_creation_fixture() -> (
    tempfile::TempDir,
    Host,
    CreateTerminal,
    CreationBinding,
    RuntimeRecord,
) {
    let (directory, host, request, mut binding, _, mut runtime) =
        pending_creation_fixture(false).await;
    runtime.bootstrap_delivery = BootstrapDelivery::NotRequired;
    // Model exactly the durable fields changed by Worker::resize_terminal_admitted.
    runtime.cols = RESIZED_CREATION_COLUMNS;
    runtime.rows = RESIZED_CREATION_ROWS;
    atomic_write_json(
        &host.paths.runtime_record(binding.result.terminal_id),
        &runtime,
    )
    .unwrap();
    lock_recover(&host.creation_capabilities)
        .set_active_phase(
            &host.paths.home().join(CREATION_LEDGER_FILE_NAME),
            binding.result,
            CreationPhase::Ready,
        )
        .unwrap();
    binding.phase = CreationPhase::Ready;
    (directory, host, request, binding, runtime)
}

fn assert_resized_creation_still_pending(host: &Host, binding: &CreationBinding) {
    assert_eq!(
        lock_recover(&host.creation_capabilities).bindings,
        vec![binding.clone()]
    );
    assert!(
        host.creation_receipts
            .lookup_binding(binding)
            .unwrap()
            .is_none()
    );
}
