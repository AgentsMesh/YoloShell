use crate::{commands::*, harness::Harness, peer::*, scoped_requests::*};
use std::time::{Duration, Instant};
use yolo_hub_client_channels::{ControlError, ControlEvent, RequestScope};

const CURRENT_VALIDITY: Duration = Duration::from_secs(8);

#[tokio::test]
async fn surviving_validity_cannot_cross_control_generation_and_retired_scope_cannot_use_new_generation()
 {
    let mut harness = Harness::start();
    let mut old_peer = harness.accept().await;
    let previous = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let scope = RequestScope::default();
    let validity = scope.validity_until(Instant::now() + CURRENT_VALIDITY);
    let first = input(grant);
    send(&harness, previous, first.clone(), validity.clone()).await;
    assert_eq!(receive(&mut old_peer).await.fields().command, first);
    drop(old_peer);
    crate::operations::generation_retired(&mut harness, previous).await;
    let ControlEvent::Finished {
        result: Err(ControlError::Client(error)),
        connected: false,
        ..
    } = harness.event().await
    else {
        panic!()
    };
    assert!(error.outcome_is_unknown());
    let mut current_peer = harness.accept().await;
    let current_generation = harness.ready().await;
    assert_ne!(current_generation, previous);
    send(&harness, previous, input(grant), validity.clone()).await;
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Err(ControlError::StaleGeneration),
            connected: true,
            ..
        }
    ));
    drop(scope);
    send(&harness, current_generation, input(grant), validity).await;
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Err(ControlError::Invalidated),
            connected: true,
            ..
        }
    ));
    let replacement = RequestScope::default();
    let intended = input(grant);
    send(
        &harness,
        current_generation,
        intended.clone(),
        replacement.validity_until(Instant::now() + CURRENT_VALIDITY),
    )
    .await;
    expect_current(&mut harness, &mut current_peer, &intended).await;
    harness.finish().await;
    eof(&mut current_peer).await;
}
