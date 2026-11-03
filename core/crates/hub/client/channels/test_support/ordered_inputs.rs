use crate::{constants::TIMEOUT, harness::Harness, peer::*};
use std::time::Instant;
use uuid::Uuid;
use yolo_hub_client_channels::{ControlError, ControlEvent, RequestScope, RequestValidity};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{InputFields, InputRequest, LeaseGrant};

pub const PREFIX: &[u8] = b"printf '%s' 'complete";
pub const SUFFIX: &[u8] = b"'\r";

pub fn validity(scope: &RequestScope) -> RequestValidity {
    scope.validity_until(Instant::now() + TIMEOUT)
}

pub fn input(grant: LeaseGrant, data: &[u8]) -> Command {
    Command::Input {
        request: InputRequest::new(InputFields {
            request_id: Uuid::new_v4(),
            grant,
            data: data.to_vec(),
        })
        .unwrap(),
    }
}

pub async fn rejected(harness: &mut Harness, expected: &Command) {
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::Invalidated),
        connected: true,
    } = harness.event().await
    else {
        panic!()
    };
    assert_eq!(&request.command, expected);
}

/// Reach a real independent request before consuming failure events. This
/// proves cancellation happened inside dispatch, without any UI round trip.
pub async fn wire_barrier(harness: &Harness, peer: &mut Peer, generation: Uuid) {
    harness.send(Some(generation), Command::Hello {}).await;
    let request = receive(peer).await;
    assert!(matches!(request.fields().command, Command::Hello {}));
    reply(peer, &request, hello_reply(harness.endpoint.hub)).await;
}

pub async fn completed_barrier(harness: &mut Harness) {
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            request: yolo_hub_client_channels::ControlRequest {
                command: Command::Hello {},
                ..
            },
            result: Ok(Reply::Hello { .. }),
            connected: true,
        }
    ));
}
