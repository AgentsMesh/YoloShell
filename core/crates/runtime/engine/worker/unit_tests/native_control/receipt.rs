use super::*;
use std::{future::Future, task::Poll};

#[tokio::test]
async fn receipt_lookup_waits_behind_a_previously_queued_effect_without_source_refresh() {
    let source = prepared();
    let fixture = &source.fixture;
    let request = interrupt(&source).await;
    let lookup = lookup(&request);
    let held = fixture.state.agent_generation_gate.lock().await;
    let effect = fixture
        .state
        .interrupt_native_agent(fixture.controller(), &request);
    let receipt = fixture
        .state
        .native_control_receipt(fixture.controller(), &lookup);
    tokio::pin!(effect, receipt);
    std::future::poll_fn(|context| {
        assert!(effect.as_mut().poll(context).is_pending());
        assert!(receipt.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(held);
    let delivered = tokio::time::timeout(ADMISSION_TIMEOUT, effect)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(delivered.outcome, NativeControlOutcome::Delivered);
    let observed = tokio::time::timeout(ADMISSION_TIMEOUT, receipt)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(observed, Some(delivered));
}
