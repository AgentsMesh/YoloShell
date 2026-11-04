use crate::worker::tests::*;
const EXACT_TERMINATION_EXIT_CODE: i32 = 0;
const EXACT_TERMINATION_OUTPUT_SEQUENCE: u64 = 0;
const UNSUPPORTED_TERMINATION_METHOD: &str = stringify!(unsupported_termination);

#[tokio::test]
async fn exact_termination_rejects_wrong_status_without_side_effects() {
    let fixture = Fixture::new();
    let request = exact_termination_request(&fixture, TerminalRuntimeStatus::Unreachable);

    let error = dispatch_test(&fixture.state, method::TERMINATE_RUNTIME_EXACT, &request)
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_termination_was_not_started(&fixture);
}

#[tokio::test]
async fn exact_termination_rejects_wrong_epoch_without_side_effects() {
    let fixture = Fixture::new();
    let request = FencedTerminateRequest {
        runtime_epoch: Uuid::new_v4(),
        ..exact_termination_request(&fixture, TerminalRuntimeStatus::Running)
    };

    let error = dispatch_test(&fixture.state, method::TERMINATE_RUNTIME_EXACT, &request)
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert_termination_was_not_started(&fixture);
}

#[tokio::test]
async fn unknown_exact_termination_method_is_non_mutating() {
    let fixture = Fixture::new();
    let request = exact_termination_request(&fixture, TerminalRuntimeStatus::Running);

    for method in [UNSUPPORTED_TERMINATION_METHOD, method::TERMINATE_SESSION] {
        let error = dispatch_test(&fixture.state, method, &request)
            .await
            .unwrap_err();

        assert_eq!(error.code, crate::protocol::error_code::UNSUPPORTED_METHOD);
        assert_termination_was_not_started(&fixture);
    }
}

#[tokio::test]
async fn capability_fenced_termination_accepts_the_current_request() {
    let fixture = Fixture::with_contained_command(CommandBuilder::new(FIXTURE_COMMAND));
    assert_dispatch_termination(&fixture, &fixture.state).await;
}

#[tokio::test]
async fn exact_termination_accepts_exited_for_every_prior_live_status() {
    let fixture = Fixture::new();
    record_exact_termination_exit(&fixture);
    for expected_status in [
        TerminalRuntimeStatus::Starting,
        TerminalRuntimeStatus::Running,
        TerminalRuntimeStatus::Finalizing,
        TerminalRuntimeStatus::OutputBlocked,
        TerminalRuntimeStatus::Unreachable,
    ] {
        let request = exact_termination_request(&fixture, expected_status);
        dispatch_test(&fixture.state, method::TERMINATE_RUNTIME_EXACT, &request)
            .await
            .unwrap();
    }
    let tombstone: TerminationTombstone = read_json(
        &fixture
            .state
            .paths
            .terminate_tombstone(fixture.terminal_id()),
    )
    .unwrap();
    assert_eq!(
        tombstone.validate_for(fixture.terminal_id(), fixture.state.runtime_epoch()),
        Ok(())
    );
}

#[test]
fn exact_termination_wire_is_strict_and_requires_non_nil_identity() {
    let request = FencedTerminateRequest {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        expected_status: Some(TerminalRuntimeStatus::Running),
        force: false,
    };
    let value = serde_json::to_value(request).unwrap();
    assert!(serde_json::from_value::<FencedTerminateRequest>(value.clone()).is_ok());
    for field in [
        crate::protocol::field::TERMINAL_ID,
        crate::protocol::field::RUNTIME_EPOCH,
    ] {
        let mut invalid = value.clone();
        invalid[field] = serde_json::Value::String(Uuid::nil().to_string());
        assert!(serde_json::from_value::<FencedTerminateRequest>(invalid).is_err());
    }
    for field in [
        crate::protocol::field::TERMINAL_ID,
        crate::protocol::field::RUNTIME_EPOCH,
        crate::protocol::field::FORCE,
    ] {
        let mut invalid = value.clone();
        invalid.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<FencedTerminateRequest>(invalid).is_err());
    }
    let mut no_expected_status = value.clone();
    no_expected_status
        .as_object_mut()
        .unwrap()
        .remove(crate::protocol::field::EXPECTED_STATUS);
    assert_eq!(
        serde_json::from_value::<FencedTerminateRequest>(no_expected_status)
            .unwrap()
            .expected_status,
        None
    );
    let mut unknown = value;
    unknown[crate::protocol::field::UNSUPPORTED_EXACT_TERMINATION_FIELD] =
        serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<FencedTerminateRequest>(unknown).is_err());
}

#[test]
fn exact_termination_uses_the_termination_rpc_deadline() {
    assert_eq!(
        rpc_timeout(method::TERMINATE_RUNTIME_EXACT),
        WORKER_TERMINATION_TIMEOUT
    );
    assert_eq!(rpc_timeout(WORKER_STATUS_METHOD), WORKER_OPERATION_TIMEOUT);
}

pub(super) fn exact_termination_request(
    fixture: &Fixture,
    expected_status: TerminalRuntimeStatus,
) -> FencedTerminateRequest {
    FencedTerminateRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        expected_status: Some(expected_status),
        force: false,
    }
}

pub(super) fn assert_termination_was_not_started(fixture: &Fixture) {
    assert_eq!(fixture.state.termination_reason(), None);
    assert!(
        !fixture
            .state
            .paths
            .terminate_tombstone(fixture.terminal_id())
            .try_exists()
            .unwrap()
    );
}

fn record_exact_termination_exit(fixture: &Fixture) {
    *lock_recover(&fixture.state.exit) = Some(ExitRecord {
        runtime_epoch: fixture.state.runtime_epoch(),
        code: Some(EXACT_TERMINATION_EXIT_CODE),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: now_ms(),
        last_output_sequence: EXACT_TERMINATION_OUTPUT_SEQUENCE,
    });
}
