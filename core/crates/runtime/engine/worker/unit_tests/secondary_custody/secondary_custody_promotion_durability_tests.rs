use crate::worker::tests::*;
#[cfg(target_os = "macos")]
const RUNTIME_ABA_FILE_NAME: &str = "runtime-promotion-aba.json";

#[cfg(target_os = "macos")]
fn install_post_publish_runtime_aba(fixture: &Fixture) {
    let state = Arc::clone(&fixture.state);
    let terminal_id = fixture.terminal_id();
    fixture
        .state
        .control_test_hooks
        .after_next_custody_runtime_publish(move || {
            let path = state.paths.runtime_record(terminal_id);
            let committed: (RuntimeRecord, crate::paths::identity::FileIdentity) =
                state.namespace.read_json_with_identity(&path).unwrap();
            let replacement_path = path.with_file_name(RUNTIME_ABA_FILE_NAME);
            crate::paths::atomic_write_json(&replacement_path, &committed.0).unwrap();
            std::fs::rename(&replacement_path, &path).unwrap();
            let replacement: (RuntimeRecord, crate::paths::identity::FileIdentity) =
                state.namespace.read_json_with_identity(&path).unwrap();
            assert_eq!(replacement.0, committed.0);
            assert!(!replacement.1.same_object(committed.1));
        });
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn same_value_new_inode_runtime_aba_fences_promotion() {
    let fixture = Fixture::new();
    assert_indeterminate_promotion_fails_closed(&fixture, install_post_publish_runtime_aba).await;
    assert!(
        fixture
            .state
            .control_test_hooks
            .take_controller_persist_indeterminate()
    );
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn post_rename_runtime_directory_sync_failure_reconciles_exact_commit() {
    let fixture = Fixture::new();
    fixture
        .state
        .control_test_hooks
        .after_next_custody_runtime_publish(crate::paths::test_support::fail_next_directory_sync);
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    wire.receive_commit(&offer).await;
    wire.send_ready(&offer).await;

    let mut peer = wire.take_peer();
    receive_exact_worker_activation(&mut peer, &offer, wire.request.controller_generation).await;
    let memory = lock_recover(&fixture.state.record).clone();
    let durable: RuntimeRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.runtime_record(fixture.terminal_id()))
        .unwrap();
    assert_eq!(durable, memory);
    assert_eq!(
        durable.controller_generation,
        wire.request.controller_generation
    );
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        wire.request.controller_generation
    );
    assert_eq!(
        fixture
            .state
            .controller_persisted_generation
            .load(Ordering::Acquire),
        wire.request.controller_generation
    );
    drop(peer);
    await_adoption_failure(adoption).await;
}
