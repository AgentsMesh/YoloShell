use super::{agent::*, constants::*, fixtures::*, peer::*};
use crate::{
    ControlCommand, ControlIntent, Options, STATUS_QUEUE_CAPACITY, Status, authority::Authority,
    control::Control,
};
use tokio::{
    sync::{mpsc, oneshot},
    time::{Instant, timeout_at},
};
use yolo_hub_relay_control_protocol::{NotSentReason, Outcome};

#[tokio::test]
async fn expired_or_wrong_principal_authority_cannot_dispatch_even_with_a_live_connection() {
    for expired in [true, false] {
        let (_directory, socket, listener) = listener();
        let options = Options {
            socket,
            entry: hub(),
            target: target(),
            principal: principal(),
            intent: ControlIntent::Observe,
            size: None,
        };
        let (result, mut peer) = tokio::join!(Control::connect(options), accept(&listener));
        let mut control = result.unwrap();
        let now = Instant::now();
        let grant = grant();
        control.authority = Authority::received(grant, now, now);
        if expired {
            control.authority.as_mut().unwrap().expires_at = now;
        } else {
            control.options.principal.controller_instance_id =
                uuid::Uuid::from_u128(ALTERNATE_VALUE);
        }
        let (status, mut statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
        let (response, receive) = oneshot::channel();
        let request = interrupt();
        control
            .semantic_interrupt(
                ControlCommand {
                    request: request.clone(),
                    response,
                },
                launch(),
                &status,
            )
            .await
            .unwrap();
        let response = timeout_at(deadline(), receive).await.unwrap().unwrap();
        response.validate_for(&request).unwrap();
        assert_eq!(
            response.fields().outcome,
            Outcome::NotSent {
                reason: if expired {
                    NotSentReason::Expired
                } else {
                    NotSentReason::TargetChanged
                }
            }
        );
        if expired {
            assert!(control.authority.is_none());
            assert_eq!(
                statuses.recv().await,
                Some(Status::ControlLost { holder_id: None })
            );
        } else {
            assert!(matches!(
                statuses.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
        }
        drop(control);
        closed(&mut peer).await;
    }
}
