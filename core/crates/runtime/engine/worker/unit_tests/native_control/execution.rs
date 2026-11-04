use super::submit_support::{WriteBehavior, record_writes};
use super::*;

#[tokio::test]
async fn exact_native_interrupt_is_once_only_and_digest_collision_is_rejected() {
    let source = prepared();
    let fixture = &source.fixture;
    let request = interrupt(&source).await;
    let first_signal = count_next_signal(fixture);
    let first = fixture
        .state
        .interrupt_native_agent(fixture.controller(), &request)
        .await
        .unwrap();
    assert_eq!(first.outcome, NativeControlOutcome::Delivered);
    assert_eq!(first_signal.load(Ordering::Acquire), ONE_SIGNAL);
    assert!(first.matches(
        request.terminal_id,
        request.runtime_epoch,
        request.request_id,
        request.command_digest
    ));
    let later_signals = count_next_signal(fixture);
    let repeated = fixture
        .state
        .interrupt_native_agent(fixture.controller(), &request)
        .await
        .unwrap();
    assert_eq!(repeated, first);
    assert_eq!(later_signals.load(Ordering::Acquire), NO_SIGNALS);
    let mut conflict = request.clone();
    conflict.command_digest = yolo_crypto::Sha256Digest::capture(&OTHER_COMMAND).unwrap();
    let error = fixture
        .state
        .interrupt_native_agent(fixture.controller(), &conflict)
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(later_signals.load(Ordering::Acquire), NO_SIGNALS);
    assert_eq!(
        fixture
            .state
            .native_control_receipt(fixture.controller(), &lookup(&request))
            .await
            .unwrap(),
        Some(first)
    );
}

#[tokio::test]
async fn completed_receipt_remains_readable_after_source_revocation_and_writer_expiry() {
    let source = prepared();
    let fixture = &source.fixture;
    let request = interrupt(&source).await;
    let delivered = fixture
        .state
        .interrupt_native_agent(fixture.controller(), &request)
        .await
        .unwrap();
    source.revoke();
    lock_recover(&fixture.state.lease).expire_for_test();
    assert_eq!(
        fixture
            .state
            .native_control_receipt(fixture.controller(), &lookup(&request))
            .await
            .unwrap(),
        Some(delivered)
    );
    assert!(
        fixture
            .state
            .interrupt_native_agent(fixture.controller(), &request)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn changed_source_and_wrong_source_witness_never_reserve_or_signal() {
    let mut source = prepared();
    let request = interrupt(&source).await;
    let mut fields = request.witness.fields().clone();
    fields.authority_digest = yolo_crypto::Sha256Digest::capture(&OTHER_COMMAND).unwrap();
    let mut wrong = request.clone();
    wrong.witness = NativeControlWitness::new(fields).unwrap();
    assert!(
        source
            .fixture
            .state
            .interrupt_native_agent(source.fixture.controller(), &wrong)
            .await
            .is_err()
    );
    source.change_activity();
    assert!(
        source
            .fixture
            .state
            .interrupt_native_agent(source.fixture.controller(), &request)
            .await
            .is_err()
    );
    assert_unreserved(&source, &request).await;
}

const INTERRUPT_KEY: &[u8] = b"\x1b";

fn key_interrupt_registry() -> yolo_agent_providers::ProviderRuntimeRegistry {
    let builtin = yolo_agent_providers::builtin_provider_runtime();
    let runtimes = builtin.runtimes().map(|runtime| {
        if *runtime.descriptor().kind() == crate::agent::AgentKind::ClaudeCode {
            runtime
                .clone()
                .with_interrupt_delivery(yolo_agent_providers::InterruptDelivery::Key(INTERRUPT_KEY))
        } else {
            runtime.clone()
        }
    });
    yolo_agent_providers::ProviderRuntimeRegistry::from_runtimes(runtimes).unwrap()
}

#[tokio::test]
async fn key_interrupt_writes_the_provider_key_and_signals_nothing() {
    let source = prepared_with_provider_runtime(key_interrupt_registry());
    let request = interrupt(&source).await;
    let writes = record_writes(&source, WriteBehavior::Complete);
    let signals = count_next_signal(&source.fixture);
    let result = source
        .fixture
        .state
        .interrupt_native_agent(source.fixture.controller(), &request)
        .await
        .unwrap();
    assert_eq!(result.outcome, NativeControlOutcome::Delivered);
    assert_eq!(*lock_recover(&writes), [INTERRUPT_KEY.to_vec()]);
    assert_eq!(signals.load(Ordering::Acquire), NO_SIGNALS);
}
