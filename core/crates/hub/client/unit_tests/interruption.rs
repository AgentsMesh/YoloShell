use super::{
    fixtures::{constants::*, *},
    server::*,
};
use crate::{ClientError, exchange::send};
use tokio::{
    io::duplex,
    time::{Instant, timeout},
};
use yolo_hub_jsonrpc_protocol::Call as Request;

use yolo_hub_jsonrpc_transport::{Framed, Incoming, TransportError};

#[tokio::test]
async fn peer_eof_after_request_is_unknown_only_for_mutations() {
    for mutation in [false, true] {
        let (mut client, mut server) = connected().await;
        let peer = async move {
            let _request = receive(&mut server).await;
        };
        let command = if mutation { input() } else { list() };
        let (result, ()) = tokio::join!(client.call(hub(), command, deadline()), peer);
        let error = result.err().unwrap();
        assert_eq!(error.outcome_is_unknown(), mutation);
        if mutation {
            assert!(matches!(
                error,
                ClientError::OutcomeUnknown(TransportError::TruncatedFrame)
            ));
        } else {
            assert!(matches!(
                error,
                ClientError::Transport(TransportError::TruncatedFrame)
            ));
        }
        assert!(!client.is_usable());
    }
}

#[tokio::test(start_paused = true)]
async fn receive_deadline_and_cancellation_close_the_channel_without_replaying() {
    for cancellation in [false, true] {
        for mutation in [false, true] {
            let (mut client, mut server) = connected().await;
            let command = if mutation { input() } else { list() };
            let peer = async {
                let _request = receive(&mut server).await;
                assert_eq!(
                    server.receive::<Request>(deadline()).await.unwrap(),
                    Incoming::Eof
                );
            };
            let caller = async {
                if cancellation {
                    assert!(
                        timeout(SHORT_TIMEOUT, client.call(hub(), command, deadline()))
                            .await
                            .is_err()
                    );
                } else {
                    let error = client
                        .call(hub(), command, Instant::now() + SHORT_TIMEOUT)
                        .await
                        .err()
                        .unwrap();
                    assert_eq!(error.outcome_is_unknown(), mutation);
                }
                assert!(!client.is_usable());
                assert!(matches!(
                    client.call(hub(), input(), deadline()).await,
                    Err(ClientError::Poisoned)
                ));
            };
            tokio::join!(caller, peer);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn blocked_write_deadline_and_broken_pipe_are_never_reusable() {
    for mutation in [false, true] {
        let (left, _right) = duplex(BLOCKED_PIPE_CAPACITY);
        let mut channel = Framed::new(left);
        let failure = send(&mut channel, &SAMPLE_PAYLOAD, mutation, deadline())
            .await
            .err()
            .unwrap();
        assert!(!failure.reusable && !channel.is_usable());
        assert_eq!(failure.error.outcome_is_unknown(), mutation);
        let (left, right) = duplex(PIPE_CAPACITY);
        drop(right);
        let mut channel = Framed::new(left);
        let failure = send(&mut channel, &SAMPLE_PAYLOAD, mutation, deadline())
            .await
            .err()
            .unwrap();
        assert!(!failure.reusable);
        assert_eq!(failure.error.outcome_is_unknown(), mutation);
    }
}

#[tokio::test]
async fn close_is_terminal_and_cannot_yield_a_reusable_channel() {
    let (mut client, mut server) = connected().await;
    client.close(deadline()).await.unwrap();
    assert!(!client.is_usable());
    assert!(matches!(
        client.close(deadline()).await,
        Err(ClientError::Poisoned)
    ));
    assert_eq!(
        server.receive::<Request>(deadline()).await.unwrap(),
        Incoming::Eof
    );
    assert!(matches!(client.into_channel(), Err(ClientError::Poisoned)));
}
