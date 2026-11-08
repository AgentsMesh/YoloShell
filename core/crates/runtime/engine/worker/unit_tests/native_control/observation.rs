use super::*;

#[tokio::test]
async fn native_observation_requires_matching_binding_and_valid_control_history() {
    let source = prepared();
    let state = &source.fixture.state;
    let request = observation(&source);
    let observed = state
        .observe_native_agent(source.fixture.controller(), &request)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(observed.binding(), &source.record.recovery_binding(true));
    assert_eq!(
        observed.witness().fields().binding_digest,
        request.binding_digest
    );
    let mut stale = request;
    stale.binding_digest = yolo_crypto::Sha256Digest::capture(&OTHER_COMMAND).unwrap();
    assert!(
        state
            .observe_native_agent(source.fixture.controller(), &stale)
            .await
            .unwrap()
            .is_none()
    );
    child_is_live(&source);
}

#[tokio::test]
async fn unavailable_control_history_keeps_the_terminal_and_writer_lease_usable() {
    let source = NativeSource::new();
    let fixture = &source.fixture;
    assert!(!fixture.state.native_control_available());
    assert!(
        fixture
            .state
            .observe_native_agent(fixture.controller(), &observation(&source))
            .await
            .unwrap()
            .is_none()
    );
    fixture
        .state
        .acquire_lease(fixture.controller(), &fixture.lease_request(WRITER), false)
        .await
        .unwrap();
    assert_eq!(fixture.state.status().status, TerminalRuntimeStatus::Running);
    child_is_live(&source);
}

#[test]
fn private_protocol_support_is_stable_while_the_journal_is_busy_or_unavailable() {
    let source = prepared();
    let state = &source.fixture.state;
    let has_support = || {
        state
            .hello()
            .capabilities
            .iter()
            .any(|value| value == crate::protocol::capability::NATIVE_AGENT_CONTROL)
    };
    let journal = lock_recover(&state.native_control);
    assert!(has_support());
    drop(journal);
    *lock_recover(&state.native_control) = None;
    assert!(has_support());
    assert!(!state.native_control_available());
}

#[tokio::test]
async fn revoked_and_uncertain_sources_are_not_advertised() {
    for uncertain in [false, true] {
        let source = prepared();
        let fixture = &source.fixture;
        if uncertain {
            let path = fixture.state.paths.manifest(fixture.terminal_id());
            let mut manifest: crate::model::TerminalManifest =
                fixture.state.namespace.read_json(&path).unwrap();
            manifest.recovery_state = crate::model::RecoveryState::Uncertain;
            fixture
                .state
                .namespace
                .lease()
                .write_json(&path, &manifest)
                .unwrap();
        } else {
            source.revoke();
        }
        assert!(
            fixture
                .state
                .observe_native_agent(fixture.controller(), &observation(&source))
                .await
                .unwrap()
                .is_none()
        );
        child_is_live(&source);
    }
}
