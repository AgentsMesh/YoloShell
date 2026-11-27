use super::*;
use std::sync::atomic::AtomicUsize;
use yolo_crypto::Sha256Digest;

pub(super) fn prepared() -> NativeSource {
    let source = NativeSource::new();
    initialize::initialize(&source.fixture);
    source
}

pub(super) fn prepared_with_provider_runtime(
    provider_runtime: yolo_agent_providers::ProviderRuntimeRegistry,
) -> NativeSource {
    let source = NativeSource::with_provider_runtime(provider_runtime);
    initialize::initialize(&source.fixture);
    source
}

pub(super) fn observation(source: &NativeSource) -> NativeObservationRequest {
    NativeObservationRequest {
        terminal_id: source.fixture.terminal_id(),
        runtime_epoch: source.fixture.state.runtime_epoch(),
        binding_digest: binding_digest(&source.record.recovery_binding(true)).unwrap(),
    }
}

pub(super) async fn interrupt(source: &NativeSource) -> NativeInterruptRequest {
    let fixture = &source.fixture;
    let state = &fixture.state;
    let observed = state
        .observe_native_agent(fixture.controller(), &observation(source))
        .await
        .unwrap()
        .unwrap();
    let mut lease_request = fixture.lease_request(WRITER);
    lease_request.ttl_ms = Some(WRITER_TTL_MS);
    let lease = state
        .acquire_lease(fixture.controller(), &lease_request, false)
        .await
        .unwrap();
    NativeInterruptRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: state.runtime_epoch(),
        request_id: Uuid::new_v4(),
        generation: lease.generation,
        writer_capability: lease.writer_capability,
        witness: observed.witness().clone(),
        command_digest: Sha256Digest::capture(&COMMAND).unwrap(),
    }
}

pub(super) fn lookup(request: &NativeInterruptRequest) -> NativeControlLookupRequest {
    NativeControlLookupRequest {
        terminal_id: request.terminal_id,
        runtime_epoch: request.runtime_epoch,
        request_id: request.request_id,
        command_digest: request.command_digest,
    }
}

pub(super) fn child_is_live(source: &NativeSource) {
    let runtime = lock_recover(&source.fixture.state.record);
    assert_eq!(
        process::process_identity_relation(runtime.child_pid, runtime.child_start_time),
        process::ProcessIdentityRelation::Matching
    );
}

pub(super) async fn assert_unreserved(source: &NativeSource, request: &NativeInterruptRequest) {
    assert!(
        source
            .fixture
            .state
            .native_control_receipt(source.fixture.controller(), &lookup(request))
            .await
            .unwrap()
            .is_none()
    );
    child_is_live(source);
}

pub(super) fn count_next_signal(fixture: &Fixture) -> Arc<AtomicUsize> {
    let signals = Arc::new(AtomicUsize::new(NO_SIGNALS));
    let observed = Arc::clone(&signals);
    fixture
        .state
        .native_control_test_hooks
        .before_next_signal(move || {
            observed.fetch_add(ONE_SIGNAL, Ordering::AcqRel);
        });
    signals
}
