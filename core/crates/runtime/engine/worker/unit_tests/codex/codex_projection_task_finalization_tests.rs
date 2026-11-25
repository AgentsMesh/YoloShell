use crate::worker::tests::*;
const PROJECTION_TASK_PANIC: &str = "injected Codex projection task panic";
const PROJECTION_ROUND_DETACH_TIMEOUT: &str =
    "Codex projection run_once remained attached after runtime task cancellation";

#[tokio::test]
async fn projection_task_panic_invalidates_the_persisted_authority_pair() {
    let fixture = active_projection_fixture();
    let task = tokio::spawn(async {
        panic!("{PROJECTION_TASK_PANIC}");
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    });

    let error = codex_hook::complete_projection_task_for_test(Arc::clone(&fixture.state), task)
        .await
        .unwrap_err();

    assert!(format!("{error:#}").contains(PROJECTION_TASK_PANIC));
    assert_projection_unavailable(&fixture);
}

#[tokio::test]
async fn projection_task_abort_invalidates_the_persisted_authority_pair() {
    let fixture = active_projection_fixture();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let _ = entered_tx.send(());
        std::future::pending::<()>().await;
        Ok::<(), anyhow::Error>(())
    });
    entered_rx.await.unwrap();
    task.abort_handle().abort();

    assert!(
        codex_hook::complete_projection_task_for_test(Arc::clone(&fixture.state), task)
            .await
            .is_err()
    );
    assert_projection_unavailable(&fixture);
}

#[tokio::test]
async fn runtime_settlement_drains_projection_before_durable_invalidation() {
    let fixture = active_projection_fixture();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let mut tasks = JoinSet::new();
    supervision::spawn_task(
        &mut tasks,
        supervision::CODEX_AUTHORITY_PROJECTION_TASK,
        codex_hook::run_idle_projection_round_for_test(Arc::clone(&fixture.state), entered_tx),
    );
    let probe = entered_rx.await.unwrap();
    assert!(probe.run_once_is_attached());
    let generation_gate = fixture.state.agent_generation_gate.lock().await;
    let state = Arc::clone(&fixture.state);
    let settlement =
        tokio::spawn(async move { settle_worker_runtime(&state, tasks, Ok(())).await });

    wait_for_projection_detachment(&probe).await;
    assert!(!settlement.is_finished());
    assert_projection_active(&fixture);
    drop(generation_gate);

    settlement.await.unwrap().unwrap();
    assert_projection_unavailable(&fixture);
}

async fn wait_for_projection_detachment(probe: &codex_hook::ProjectionRoundProbe) {
    tokio::time::timeout(CONTROL_BARRIER_TIMEOUT, async {
        while probe.run_once_is_attached() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect(PROJECTION_ROUND_DETACH_TIMEOUT);
}

pub(super) fn active_projection_fixture() -> Fixture {
    let fixture = Fixture::new();
    let record = lock_recover(&fixture.state.record).clone();
    let authority = projection_authority(&fixture, &record);
    publish_as_projection(&fixture, &record, &authority).unwrap();
    assert_projection_pair(&fixture, &authority);
    fixture
}

fn assert_projection_unavailable(fixture: &Fixture) {
    let authority = persisted_projection_authority(fixture);
    assert_eq!(
        authority.status,
        crate::agent::codex::authority::CodexAuthorityStatus::Unavailable
    );
    assert_projection_binding(fixture, authority.binding);
}

pub(super) fn assert_projection_active(fixture: &Fixture) {
    let authority = persisted_projection_authority(fixture);
    assert_eq!(
        authority.status,
        crate::agent::codex::authority::CodexAuthorityStatus::Active
    );
    assert_projection_binding(fixture, authority.binding);
}

fn persisted_projection_authority(
    fixture: &Fixture,
) -> crate::agent::codex::authority::CodexBindingAuthority {
    let store = AgentBindingStore::new(fixture.state.paths.agent_binding(fixture.terminal_id()));
    store
        .codex_authority_store()
        .unwrap()
        .load_in_namespace(fixture.state.namespace.lease())
        .unwrap()
        .unwrap()
}

fn assert_projection_binding(fixture: &Fixture, binding: AgentBinding) {
    let store = AgentBindingStore::new(fixture.state.paths.agent_binding(fixture.terminal_id()));
    assert_eq!(
        store
            .read_raw_in_namespace(fixture.state.namespace.lease())
            .unwrap(),
        Some(binding)
    );
}

#[tokio::test]
async fn exited_child_still_drains_the_final_projection_on_shutdown() {
    let fixture = active_projection_fixture();
    fixture.state.child_waited.store(true, Ordering::Release);
    fixture.state.commit_response_shutdown();
    codex_hook::supervise_closed_projection_for_test(Arc::clone(&fixture.state))
        .await
        .unwrap();
    assert_projection_active(&fixture);
    settle_worker_runtime(&fixture.state, JoinSet::<()>::new(), Ok(()))
        .await
        .unwrap();
    assert_projection_unavailable(&fixture);
}

#[tokio::test]
async fn projection_final_invalidation_failure_cannot_be_hidden_by_an_exited_child() {
    let fixture = active_projection_fixture();
    fixture.state.child_waited.store(true, Ordering::Release);
    fixture.state.commit_response_shutdown();
    fixture
        .state
        .namespace
        .write_json(
            &fixture.state.paths.agent_binding(fixture.terminal_id()),
            &serde_json::Value::Null,
        )
        .unwrap();
    codex_hook::supervise_closed_projection_for_test(Arc::clone(&fixture.state))
        .await
        .unwrap();
    let error = settle_worker_runtime(&fixture.state, JoinSet::<()>::new(), Ok(()))
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains(CODEX_PROJECTION_SHUTDOWN_FAILURE)
    );
}
