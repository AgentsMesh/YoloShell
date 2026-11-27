use super::{boundary::*, *};

#[tokio::test]
async fn proxy_forward_defers_observation_until_downstream_has_validated_the_exact_reply() {
    let mut harness = Harness::new(Shape::Correlated, true, true).await;
    let reply = harness
        .client
        .forward(&request(), deadline())
        .await
        .unwrap();
    let response: ResponseEnvelope = yolo_capnp_codec::from_value(reply.value).unwrap();
    assert_eq!(response.request_id, REQUEST_ID);
    assert_eq!(response.method, METHOD);
    assert_eq!(response.payload.as_slice(), Some(RESPONSE));
    assert_eq!(harness.confirmed.load(Ordering::SeqCst), 0);
    assert!(matches!(
        harness.client.next_event(deadline()).await,
        Err(ProtocolError::ConnectionPoisoned)
    ));
    reply.receipt.unwrap().confirm(deadline()).await.unwrap();
    assert_eq!(harness.confirmed.load(Ordering::SeqCst), 1);
    let mut events = reply.events.unwrap();
    harness
        .events
        .send(Value::Data(RESPONSE.to_vec()))
        .await
        .unwrap();
    assert_eq!(
        events.next(deadline()).await.unwrap().as_slice(),
        Some(RESPONSE)
    );
    drop(events);
    wait_for_retirement(&harness.retired).await;
    assert_eq!(harness.calls.load(Ordering::SeqCst), 1);
    harness.finish().await;
}

#[tokio::test]
async fn wrong_business_identity_or_malformed_envelope_is_rejected_before_receipt_confirmation() {
    for shape in [
        Shape::WrongId,
        Shape::Malformed,
        Shape::MissingPayload,
        Shape::UnknownMetadata,
    ] {
        for forward in [false, true] {
            let mut harness = Harness::new(shape, false, true).await;
            let result = if forward {
                harness
                    .client
                    .forward(&request(), deadline())
                    .await
                    .map(|_| ())
            } else {
                harness
                    .client
                    .exchange(&request(), deadline())
                    .await
                    .map(|_| ())
            };
            assert!(matches!(
                result,
                Err(ProtocolError::ResponseMismatch | ProtocolError::Decode(_))
            ));
            assert_eq!(harness.confirmed.load(Ordering::SeqCst), 0);
            assert_eq!(harness.calls.load(Ordering::SeqCst), 1);
            harness.finish().await;
        }
    }
}

#[tokio::test]
async fn typed_call_and_correlated_exchange_work_without_optional_receipts_or_event_capabilities() {
    for correlated in [true, false] {
        let mut harness = Harness::new(Shape::Correlated, false, false).await;
        let response: ResponseEnvelope = if correlated {
            harness
                .client
                .exchange(&request(), deadline())
                .await
                .unwrap()
        } else {
            harness.client.call(&request(), deadline()).await.unwrap()
        };
        assert_eq!(response.payload.as_slice(), Some(RESPONSE));
        assert!(matches!(
            harness.client.wait_event().await,
            Err(ProtocolError::ConnectionPoisoned)
        ));
        harness.client.close();
        assert!(matches!(
            harness.client.exchange(&request(), deadline()).await,
            Err(ProtocolError::EndOfStream)
        ));
        assert_eq!(harness.calls.load(Ordering::SeqCst), 1);
        harness.finish().await;
    }
}

#[tokio::test]
async fn typed_call_cannot_acknowledge_a_value_that_failed_domain_deserialization() {
    let mut harness = Harness::new(Shape::Malformed, false, true).await;
    let result = harness
        .client
        .call::<_, ResponseEnvelope>(&request(), deadline())
        .await;
    assert!(matches!(result, Err(ProtocolError::Decode(_))));
    assert_eq!(harness.confirmed.load(Ordering::SeqCst), 0);
    harness.finish().await;
}
