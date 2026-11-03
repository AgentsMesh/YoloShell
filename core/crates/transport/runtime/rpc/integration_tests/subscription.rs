use super::{boundary::*, *};
use yolo_wire::EventEnvelope;
const READ_TIMEOUT: Duration = Duration::from_millis(100);

#[tokio::test]
async fn observed_exchange_and_typed_call_install_native_event_subscriptions_then_close_detaches_them()
 {
    for correlated in [true, false] {
        let mut harness = Harness::new(Shape::Correlated, true, true).await;
        let _: ResponseEnvelope = if correlated {
            harness
                .client
                .exchange(&request(), deadline())
                .await
                .unwrap()
        } else {
            harness.client.call(&request(), deadline()).await.unwrap()
        };
        assert_eq!(harness.confirmed.load(Ordering::SeqCst), 1);
        let event =
            EventEnvelope::new(EVENT_NAME, None, None, &Value::Data(RESPONSE.to_vec())).unwrap();
        for bounded in [true, false] {
            harness
                .events
                .send(yolo_capnp_codec::to_value(&event).unwrap())
                .await
                .unwrap();
            let received = if bounded {
                harness.client.next_event(deadline()).await.unwrap()
            } else {
                harness.client.wait_event().await.unwrap()
            };
            assert_eq!(received, event);
        }
        harness.client.close();
        assert!(matches!(
            harness.client.wait_event().await,
            Err(ProtocolError::ConnectionPoisoned)
        ));
        wait_for_retirement(&harness.retired).await;
        harness.finish().await;
    }
}

#[tokio::test]
async fn cancelling_idle_observation_releases_only_its_subscription_and_does_not_replay_runtime_calls()
 {
    let mut harness = Harness::new(Shape::Correlated, true, true).await;
    harness
        .client
        .exchange(&request(), deadline())
        .await
        .unwrap();
    {
        let entered = harness.entered.notified();
        let waiting = harness.client.wait_event();
        tokio::pin!(waiting);
        tokio::select! {
            result = &mut waiting => panic!("unexpected idle event: {result:?}"),
            observed = tokio::time::timeout(DEADLINE, entered) => observed.unwrap(),
        }
    }
    wait_for_retirement(&harness.retired).await;
    assert!(matches!(
        harness.client.wait_event().await,
        Err(ProtocolError::EndOfStream)
    ));
    let result = harness
        .client
        .exchange(&request(), deadline())
        .await
        .unwrap();
    assert_eq!(result.payload.as_slice(), Some(RESPONSE));
    assert_eq!(harness.calls.load(Ordering::SeqCst), 2);
    harness.finish().await;
}

#[tokio::test]
async fn malformed_events_fail_typed_decoding_and_read_deadlines_retire_idle_subscriptions() {
    let mut harness = Harness::new(Shape::Correlated, true, false).await;
    harness
        .client
        .exchange(&request(), deadline())
        .await
        .unwrap();
    for bounded in [true, false] {
        harness.events.send(Value::Boolean(false)).await.unwrap();
        let result = if bounded {
            harness.client.next_event(deadline()).await
        } else {
            harness.client.wait_event().await
        };
        assert!(matches!(result, Err(ProtocolError::Decode(_))));
    }
    harness.client.close();
    wait_for_retirement(&harness.retired).await;
    harness.finish().await;

    let mut idle = Harness::new(Shape::Correlated, true, false).await;
    idle.client.exchange(&request(), deadline()).await.unwrap();
    assert!(matches!(
        idle.client.next_event(Instant::now() + READ_TIMEOUT).await,
        Err(ProtocolError::Io(_))
    ));
    wait_for_retirement(&idle.retired).await;
    assert!(matches!(
        idle.client.wait_event().await,
        Err(ProtocolError::EndOfStream)
    ));
    idle.finish().await;
}
