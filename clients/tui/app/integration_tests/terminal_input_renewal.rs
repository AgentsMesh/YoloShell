use super::{
    channel_peer::*,
    input_transport::{Harness, hello_reply},
};
use crate::{constants::QUEUE_CAPACITY, network::Job};
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

const PREFIX: &[u8] = b"printf 'complete";
const SUFFIX: &[u8] = b"'\r";

#[tokio::test]
async fn a_full_input_backlog_cannot_prevent_lease_maintenance_or_hide_unknown_delivery() {
    let mut harness = Harness::start().await;
    let first = harness.send_input(PREFIX.to_vec()).await;
    let blocked = receive(&mut harness.peer).await;
    let mut suffixes = Vec::new();
    for _ in 0..QUEUE_CAPACITY {
        suffixes.push(harness.send_input(SUFFIX.to_vec()).await);
    }
    harness
        .jobs
        .send(Job::Call {
            generation: Some(harness.generation),
            origin: harness.grant.fields().target.address.hub_id,
            command: Command::Renew {
                grant: harness.grant,
            },
        })
        .await
        .unwrap();
    let (stream, _) = tokio::time::timeout_at(deadline(), harness.listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut maintenance = yolo_hub_jsonrpc_transport::Framed::new(stream);
    let greeting = receive(&mut maintenance).await;
    let hub = harness.grant.fields().target.address.hub_id;
    respond(&mut maintenance, &greeting, hello_reply(hub)).await;
    let renew = receive(&mut maintenance).await;
    assert_eq!(
        renew.fields().command,
        Command::Renew {
            grant: harness.grant
        }
    );
    respond(
        &mut maintenance,
        &renew,
        Reply::Lease {
            grant: harness.grant,
        },
    )
    .await;
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Ok(Reply::Lease { .. }),
            connected: true,
            ..
        }
    ));
    respond(
        &mut harness.peer,
        &blocked,
        Reply::Error {
            code: PublicError::OutcomeUnknown,
        },
    )
    .await;
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::Client(_)),
        connected: true,
    } = harness.event().await
    else {
        panic!()
    };
    assert_eq!(request.command, first.command);
    for expected in suffixes {
        let ControlEvent::Finished {
            request,
            result: Err(ControlError::Invalidated),
            connected: true,
        } = harness.event().await
        else {
            panic!()
        };
        assert_eq!(request.command, expected.command);
    }
    harness.send_hello().await;
    let barrier = receive(&mut harness.peer).await;
    assert!(matches!(barrier.fields().command, Command::Hello {}));
    respond(&mut harness.peer, &barrier, hello_reply(hub)).await;
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Ok(Reply::Hello { .. }),
            ..
        }
    ));
}
