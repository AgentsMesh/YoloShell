use crate::worker::tests::*;
const RUNTIME_SETTLEMENT_PRIMARY_FAILURE: &str = "injected orphan runtime durability failure";

#[tokio::test]
async fn projection_cleanup_cannot_replace_the_runtime_settlement_failure() {
    let fixture = active_projection_fixture();
    std::fs::remove_file(fixture.state.paths.manifest(fixture.terminal_id())).unwrap();

    let error = settle_worker_runtime_with_projection(
        &fixture.state,
        JoinSet::<()>::new(),
        Err(anyhow::anyhow!(RUNTIME_SETTLEMENT_PRIMARY_FAILURE)),
        AgentProjectionShutdown::Invalidate,
    )
    .await
    .unwrap_err();

    let detail = format!("{error:#}");
    assert!(detail.contains(RUNTIME_SETTLEMENT_PRIMARY_FAILURE));
    assert!(detail.contains(CODEX_PROJECTION_SHUTDOWN_FAILURE));
    assert_eq!(
        error.chain().last().unwrap().to_string(),
        RUNTIME_SETTLEMENT_PRIMARY_FAILURE
    );
    assert_projection_active(&fixture);
}
