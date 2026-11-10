use super::*;
use yolo_agent_control_runtime::{ANCHOR_EXTENSION, ANCHOR_PREFIX, JOURNAL_DIRECTORY};

#[tokio::test]
async fn journal_or_anchor_lost_after_reservation_returns_unknown_without_signalling() {
    for external_anchor in [false, true] {
        let source = prepared();
        let fixture = &source.fixture;
        let request = interrupt(&source).await;
        let signals = count_next_signal(fixture);
        let directory = fixture.state.paths.session_dir(request.terminal_id);
        let target = if external_anchor {
            directory
                .join(format!("{ANCHOR_PREFIX}{}", request.runtime_epoch))
                .with_extension(ANCHOR_EXTENSION)
        } else {
            directory
                .join(JOURNAL_DIRECTORY)
                .join(request.runtime_epoch.to_string())
                .join(yolo_agent_control_journal::JOURNAL_FILE)
        };
        fixture
            .state
            .native_control_test_hooks
            .after_next_reserve(move || {
                std::fs::remove_file(target).unwrap();
            });
        let result = fixture
            .state
            .interrupt_native_agent(fixture.controller(), &request)
            .await
            .unwrap();
        assert_eq!(result.outcome, NativeControlOutcome::Unknown);
        assert_eq!(signals.load(Ordering::Acquire), NO_SIGNALS);
        assert!(!fixture.state.native_control_available());
        assert!(
            fixture
                .state
                .interrupt_native_agent(fixture.controller(), &request)
                .await
                .is_err()
        );
        assert_eq!(signals.load(Ordering::Acquire), NO_SIGNALS);
        child_is_live(&source);
    }
}

#[tokio::test]
async fn lease_expiring_after_durable_reservation_is_definitively_not_delivered() {
    let source = prepared();
    let fixture = &source.fixture;
    let request = interrupt(&source).await;
    let signals = count_next_signal(fixture);
    let state = Arc::clone(&fixture.state);
    fixture
        .state
        .native_control_test_hooks
        .after_next_reserve(move || {
            lock_recover(&state.lease).expire_for_test();
        });
    let result = fixture
        .state
        .interrupt_native_agent(fixture.controller(), &request)
        .await
        .unwrap();
    assert_eq!(result.outcome, NativeControlOutcome::NotDelivered);
    assert_eq!(signals.load(Ordering::Acquire), NO_SIGNALS);
    assert_eq!(
        fixture
            .state
            .native_control_receipt(fixture.controller(), &lookup(&request))
            .await
            .unwrap(),
        Some(result)
    );
    child_is_live(&source);
}

#[tokio::test]
async fn source_replaced_after_durable_reservation_cannot_authorize_the_signal() {
    let source = prepared();
    let fixture = &source.fixture;
    let request = interrupt(&source).await;
    let signals = count_next_signal(fixture);
    let path = fixture.state.paths.agent_binding(fixture.terminal_id());
    let mut replacement = source.record.recovery_binding(true);
    replacement.state = crate::agent::AgentState::Running;
    let replacement = serde_json::to_vec(&replacement).unwrap();
    fixture
        .state
        .native_control_test_hooks
        .after_next_reserve(move || {
            // Simulate a non-cooperating writer while the durable admission lock is held.
            std::fs::write(path, replacement).unwrap();
        });
    let result = fixture
        .state
        .interrupt_native_agent(fixture.controller(), &request)
        .await
        .unwrap();
    assert_eq!(result.outcome, NativeControlOutcome::Unknown);
    assert_eq!(signals.load(Ordering::Acquire), NO_SIGNALS);
    assert_eq!(
        fixture
            .state
            .native_control_receipt(fixture.controller(), &lookup(&request))
            .await
            .unwrap(),
        Some(result)
    );
    child_is_live(&source);
}

#[tokio::test]
async fn manifest_archived_or_uncertain_after_reservation_cannot_authorize_the_signal() {
    for archived in [false, true] {
        let source = prepared();
        let fixture = &source.fixture;
        let request = interrupt(&source).await;
        let signals = count_next_signal(fixture);
        let path = fixture.state.paths.manifest(fixture.terminal_id());
        let mut manifest: crate::model::TerminalManifest =
            fixture.state.namespace.read_json(&path).unwrap();
        if archived {
            manifest.archived = true;
        } else {
            manifest.recovery_state = crate::model::RecoveryState::Uncertain;
        }
        let replacement = serde_json::to_vec(&manifest).unwrap();
        fixture
            .state
            .native_control_test_hooks
            .after_next_reserve(move || {
                std::fs::write(path, replacement).unwrap();
            });
        let result = fixture
            .state
            .interrupt_native_agent(fixture.controller(), &request)
            .await
            .unwrap();
        assert_eq!(result.outcome, NativeControlOutcome::Unknown);
        assert_eq!(signals.load(Ordering::Acquire), NO_SIGNALS);
        assert_eq!(
            fixture
                .state
                .native_control_receipt(fixture.controller(), &lookup(&request))
                .await
                .unwrap(),
            Some(result)
        );
        child_is_live(&source);
    }
}
