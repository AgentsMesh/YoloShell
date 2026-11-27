use crate::host::tests::*;
const READINESS_BUDGET_TEST_TOTAL: Duration = Duration::from_millis(40);
const READINESS_BUDGET_TEST_SETTLEMENT: Duration = Duration::from_millis(10);

struct ReadinessBudgetFixture {
    host: Host,
    manifest: TerminalManifest,
    binding: CreationBinding,
    planned: CreationLaunchIdentity,
    _directory: HostTestDirectory,
}

async fn readiness_budget_fixture() -> ReadinessBudgetFixture {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let (_, reserved, ledger_path) =
        reserve_test_creation(&host, directory.path(), CREATE_DEADLINE_TEST_SESSION_NAME);
    let manifest = host.prepare_bound_session(&reserved).await.unwrap();
    let planned = lock_recover(&host.creation_capabilities)
        .prepare_launch(&ledger_path, reserved.result)
        .unwrap();
    let binding = lock_recover(&host.creation_capabilities)
        .bindings
        .iter()
        .find(|binding| binding.result == reserved.result)
        .unwrap()
        .clone();
    ReadinessBudgetFixture {
        _directory: HostTestDirectory::capture(directory, &host.paths),
        host,
        manifest,
        binding,
        planned,
    }
}

async fn exhaust_readiness_budget(
    fixture: &ReadinessBudgetFixture,
    outer: HostOperationDeadline,
) -> WorkerStart {
    let readiness = outer.reserving(READINESS_BUDGET_TEST_SETTLEMENT);
    let launch = planned_worker_launch(&fixture.host, &fixture.binding, &fixture.planned);
    fixture
        .host
        .await_worker_start_before(
            &fixture.manifest,
            launch.runtime_epoch,
            launch.worker_id,
            readiness,
        )
        .await
        .unwrap()
}

#[tokio::test(start_paused = true)]
async fn settlement_larger_than_outer_budget_expires_only_the_child_deadline() {
    let outer = HostOperationDeadline::after(READINESS_BUDGET_TEST_SETTLEMENT);
    let readiness = outer.reserving(READINESS_BUDGET_TEST_TOTAL);

    assert!(readiness.is_elapsed());
    assert!(!outer.is_elapsed());
    tokio::time::advance(READINESS_BUDGET_TEST_SETTLEMENT).await;
    assert!(outer.is_elapsed());
}

#[tokio::test(start_paused = true)]
async fn exhausted_readiness_preserves_outer_budget_for_uncertain_publication() {
    let mut fixture = readiness_budget_fixture().await;
    let outer = HostOperationDeadline::after(READINESS_BUDGET_TEST_TOTAL);

    let start = exhaust_readiness_budget(&fixture, outer).await;

    assert_eq!(uncertain_start_detail(start), MISSING_WORKER_DETAIL);
    assert!(!outer.is_elapsed());
    fixture
        .host
        .mark_creation_worker_start_uncertain_before(
            &mut fixture.manifest,
            &anyhow::anyhow!(MISSING_WORKER_DETAIL),
            outer,
        )
        .await
        .unwrap();
    assert_eq!(
        fixture
            .host
            .load_durable_manifest(fixture.manifest.terminal_id)
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );
    assert_eq!(
        fixture
            .host
            .catalog_get(fixture.manifest.terminal_id)
            .await
            .unwrap()
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );
    assert_creation_plan_phase(
        &fixture.host,
        &fixture.binding,
        &fixture.planned,
        CreationPhase::Launching,
    );
}

#[tokio::test(start_paused = true)]
async fn exhausted_outer_budget_cannot_publish_or_confirm_uncertain_state() {
    let mut fixture = readiness_budget_fixture().await;
    let outer = HostOperationDeadline::after(READINESS_BUDGET_TEST_TOTAL);
    let start = exhaust_readiness_budget(&fixture, outer).await;
    assert_eq!(uncertain_start_detail(start), MISSING_WORKER_DETAIL);
    tokio::time::advance(READINESS_BUDGET_TEST_SETTLEMENT).await;

    let error = fixture
        .host
        .mark_creation_worker_start_uncertain_before(
            &mut fixture.manifest,
            &anyhow::anyhow!(MISSING_WORKER_DETAIL),
            outer,
        )
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert_eq!(
        fixture
            .host
            .load_durable_manifest(fixture.manifest.terminal_id)
            .unwrap()
            .recovery_state,
        RecoveryState::Created
    );
    assert_eq!(
        fixture
            .host
            .catalog_get(fixture.manifest.terminal_id)
            .await
            .unwrap()
            .unwrap()
            .recovery_state,
        RecoveryState::Created
    );
    assert_creation_plan_phase(
        &fixture.host,
        &fixture.binding,
        &fixture.planned,
        CreationPhase::Launching,
    );
}
