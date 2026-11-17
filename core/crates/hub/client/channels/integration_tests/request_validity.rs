use crate::{commands::*, harness::Harness, peer::*, scoped_requests::*};
use std::time::{Duration, Instant};
use yolo_hub_client_channels::{ControlError, ControlEvent, RequestScope};
use yolo_hub_rpc_protocol::{Command, Reply};

const SHORT_VALIDITY: Duration = Duration::from_millis(30);
const CURRENT_VALIDITY: Duration = Duration::from_secs(5);

#[derive(Clone, Copy)]
enum Retirement {
    Drop,
    Explicit,
    Expire,
}

#[tokio::test]
async fn retired_or_expired_request_queued_behind_rpc_sends_zero_bytes_and_current_request_flows() {
    for retirement in [Retirement::Drop, Retirement::Explicit, Retirement::Expire] {
        let mut harness = Harness::start();
        let mut peer = harness.accept().await;
        let generation = harness.ready().await;
        let grant = grant(harness.endpoint.hub);
        let blocked = input(grant);
        harness.send(Some(generation), blocked.clone()).await;
        let in_flight = receive(&mut peer).await;
        assert_eq!(in_flight.fields().command, blocked);
        let scope = RequestScope::default();
        let expires = Instant::now()
            + match retirement {
                Retirement::Expire => SHORT_VALIDITY,
                Retirement::Drop | Retirement::Explicit => CURRENT_VALIDITY,
            };
        let validity = scope.validity_until(expires);
        let retired = input(grant);
        send(&harness, generation, retired.clone(), validity.clone()).await;
        match retirement {
            Retirement::Drop => drop(scope),
            Retirement::Explicit => scope.retire(),
            Retirement::Expire => {
                tokio::time::sleep_until(tokio::time::Instant::from_std(expires)).await
            }
        }
        assert!(!validity.is_current());
        let current_scope = RequestScope::default();
        let current = input(grant);
        send(
            &harness,
            generation,
            current.clone(),
            current_scope.validity_until(Instant::now() + CURRENT_VALIDITY),
        )
        .await;
        reply(&mut peer, &in_flight, Reply::Input { duplicate: false }).await;
        delivered(&mut harness, &blocked).await;
        let ControlEvent::Finished {
            request,
            result,
            connected,
        } = harness.event().await
        else {
            panic!()
        };
        assert_eq!(request.command, retired);
        assert_eq!(request.validity.as_ref().unwrap().deadline(), expires);
        assert!(matches!(result, Err(ControlError::Invalidated)));
        assert!(connected);
        expect_current(&mut harness, &mut peer, &current).await;
        harness.finish().await;
        eof(&mut peer).await;
    }
}

#[tokio::test]
async fn scope_drop_after_dispatch_preserves_final_reply_but_fences_later_work_and_cleanup_still_drains()
 {
    let mut harness = Harness::start();
    let mut peer = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let scope = RequestScope::default();
    let validity = scope.validity_until(Instant::now() + CURRENT_VALIDITY);
    let first = input(grant);
    send(&harness, generation, first.clone(), validity.clone()).await;
    let in_flight = receive(&mut peer).await;
    let retired = input(grant);
    send(&harness, generation, retired.clone(), validity).await;
    drop(scope);
    reply(&mut peer, &in_flight, Reply::Input { duplicate: false }).await;
    delivered(&mut harness, &first).await;
    let ControlEvent::Finished {
        request,
        result,
        connected,
    } = harness.event().await
    else {
        panic!()
    };
    assert_eq!(request.command, retired);
    assert!(matches!(result, Err(ControlError::Invalidated)));
    assert!(connected);
    harness.controls.close();
    harness
        .send(Some(generation), Command::Release { grant })
        .await;
    harness.commands.take();
    let cleanup = receive(&mut peer).await;
    assert_eq!(cleanup.fields().command, Command::Release { grant });
    reply(&mut peer, &cleanup, Reply::Completed {}).await;
    harness.finish().await;
    eof(&mut peer).await;
}
