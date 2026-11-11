use crate::host::tests::*;
include!("../../../test_support/unit/resume/resume_provider_binding_completion_constants.rs");

const COMPLETION_PROVIDER_SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const COMPLETION_PROVIDER_CWD_DIRECTORY: &str = "provider-cwd";
const COMPLETION_PROVIDER_HOME_DIRECTORY: &str = "provider-home";
const COMPLETION_PROVIDER_EVIDENCE_DIRECTORY: &str = "2026/08/11";
const COMPLETION_PROVIDER_ROLLOUT_STAMP: &str = "2026-08-11T00-00-00";
const COMPLETION_PROVIDER_CHANGED_FIELD: &str = "-changed";
const COMPLETION_PROVIDER_PREVIOUS_PID: u32 = 31_901;
const COMPLETION_PROVIDER_PREVIOUS_START_TIME: u64 = 41_901;
const COMPLETION_PROVIDER_INITIAL_TIMESTAMP: i64 = 1;
const COMPLETION_PROVIDER_TIMESTAMP_INCREMENT: i64 = 1;
const COMPLETION_PROVIDER_GEOMETRY_INCREMENT: u16 = 1;
const COMPLETION_PROVIDER_SERVER_TIMEOUT: Duration = crate::session_timing::WORKER_AUTH_TIMEOUT;
const COMPLETION_PROVIDER_FAILURE_TIMEOUT: Duration = Duration::from_millis(120);

#[tokio::test]
async fn provider_completion_waits_for_exact_authoritative_binding_publication() {
    let fixture = ProviderWaitFixture::new(COMPLETION_PROVIDER_FAILURE_TIMEOUT);

    let missing = finish_provider_auth_only(
        &fixture.host,
        fixture.manifest.clone(),
        &fixture.transaction,
        &fixture.runtime,
    )
    .await;
    assert_provider_completion_pending_retains(&fixture.host, &fixture.transaction, missing);
    fixture.assert_authority_files_retained();

    fixture.publish_binding(&fixture.planned);
    let manifest = fixture
        .host
        .load_durable_manifest(fixture.transaction.terminal_id)
        .unwrap();
    let stale = finish_provider_auth_only(
        &fixture.host,
        manifest,
        &fixture.transaction,
        &fixture.runtime,
    )
    .await;
    assert_provider_completion_pending_retains(&fixture.host, &fixture.transaction, stale);
    fixture.assert_authority_files_retained();

    let resumed = fixture.resumed_binding();
    fixture.publish_binding(&resumed);
    let manifest = fixture
        .host
        .load_durable_manifest(fixture.transaction.terminal_id)
        .unwrap();
    let view = finish_provider_ready(
        &fixture.host,
        manifest,
        &fixture.transaction,
        &fixture.runtime,
    )
    .await
    .unwrap();

    assert_eq!(view.manifest.recovery_state, RecoveryState::Resumed);
    assert_eq!(view.agent.as_ref(), Some(&resumed));
    assert!(
        !fixture
            .host
            .paths
            .worker_launch(fixture.transaction.terminal_id)
            .exists()
    );
    assert!(
        !fixture
            .host
            .paths
            .resume_transaction(fixture.transaction.terminal_id)
            .exists()
    );
}

#[tokio::test]
async fn provider_completion_retry_accepts_resize_while_hooks_were_pending() {
    let mut fixture = ProviderWaitFixture::new(COMPLETION_PROVIDER_FAILURE_TIMEOUT);
    let pending = finish_provider_auth_only(
        &fixture.host,
        fixture.manifest.clone(),
        &fixture.transaction,
        &fixture.runtime,
    )
    .await;
    assert_provider_completion_pending_retains(&fixture.host, &fixture.transaction, pending);
    fixture.runtime.cols += COMPLETION_PROVIDER_GEOMETRY_INCREMENT;
    fixture.runtime.rows += COMPLETION_PROVIDER_GEOMETRY_INCREMENT;
    fixture
        .host
        .write_namespace_json(
            &fixture
                .host
                .paths
                .runtime_record(fixture.transaction.terminal_id),
            &fixture.runtime,
        )
        .unwrap();
    let resumed = fixture.resumed_binding();
    fixture.publish_binding(&resumed);
    let manifest = fixture
        .host
        .load_durable_manifest(fixture.transaction.terminal_id)
        .unwrap();
    let view = finish_provider_ready(
        &fixture.host,
        manifest,
        &fixture.transaction,
        &fixture.runtime,
    )
    .await
    .unwrap();
    assert_eq!(view.manifest.recovery_state, RecoveryState::Resumed);
    assert_eq!(view.agent.as_ref(), Some(&resumed));
    let current: RuntimeRecord = fixture
        .host
        .read_namespace_json(
            &fixture
                .host
                .paths
                .runtime_record(fixture.transaction.terminal_id),
        )
        .unwrap();
    assert_eq!(current, fixture.runtime);
    assert!(
        !fixture
            .host
            .paths
            .resume_transaction(fixture.transaction.terminal_id)
            .exists()
    );
}

#[test]
fn provider_completion_requires_stable_provider_identity() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let planned = fixture.planned.clone();
    let current = fixture.resumed_binding();
    assert!(provider_binding_stable_fields_match(&planned, &current));

    let mutations: &[fn(&mut AgentBinding)] = &[
        |value| value.terminal_id = Uuid::new_v4(),
        |value| value.kind = crate::agent::AgentKind::ClaudeCode,
        |value| {
            value
                .provider_session_id
                .push_str(COMPLETION_PROVIDER_CHANGED_FIELD)
        },
        |value| value.executable.push_str(COMPLETION_PROVIDER_CHANGED_FIELD),
        |value| value.cwd.push_str(COMPLETION_PROVIDER_CHANGED_FIELD),
        |value| {
            value
                .evidence_path
                .push_str(COMPLETION_PROVIDER_CHANGED_FIELD)
        },
        |value| value.data_home.push_str(COMPLETION_PROVIDER_CHANGED_FIELD),
        |value| {
            value.detected_at_ms = value
                .detected_at_ms
                .saturating_add(COMPLETION_PROVIDER_TIMESTAMP_INCREMENT)
        },
    ];
    for mutate in mutations {
        let mut changed = current.clone();
        mutate(&mut changed);
        assert!(!provider_binding_stable_fields_match(&planned, &changed));
    }

    let authority = ProviderResumeAuthority::new(
        &fixture.transaction,
        &fixture.runtime,
        current.clone(),
        current.updated_at_ms,
    )
    .unwrap();
    authority.validate_for(&fixture.transaction).unwrap();
    authority.validate_runtime(&fixture.runtime).unwrap();

    let mut unavailable = current;
    unavailable.recovery_availability = crate::agent::AgentRecoveryAvailability::Unavailable;
    assert!(
        ProviderResumeAuthority::new(
            &fixture.transaction,
            &fixture.runtime,
            unavailable.clone(),
            unavailable.updated_at_ms,
        )
        .is_err()
    );
}

include!("../../../test_support/unit/resume/resume_provider_binding_completion_support.rs");
