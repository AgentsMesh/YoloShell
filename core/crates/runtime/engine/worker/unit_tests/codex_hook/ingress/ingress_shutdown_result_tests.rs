use super::*;

const SETTLEMENT_FAILURE: &str = "final Codex intake failed";

#[test]
fn completed_listener_preserves_success_and_settlement_failure() {
    listener_settlement(Ok(Ok(()))).unwrap();
    let error = listener_settlement(Ok(Err(anyhow::anyhow!(SETTLEMENT_FAILURE)))).unwrap_err();
    assert!(format!("{error:#}").contains(SETTLEMENT_FAILURE));
}

#[tokio::test]
async fn cancelled_listener_is_not_a_successful_drain() {
    let listener = tokio::spawn(std::future::pending::<anyhow::Result<()>>());
    listener.abort();
    assert!(listener_settlement(listener.await).is_err());
}

#[tokio::test]
async fn panicking_listener_is_not_a_successful_drain() {
    let listener = tokio::spawn(async { panic!("{SETTLEMENT_FAILURE}") });
    assert!(listener_settlement(listener.await).is_err());
}
