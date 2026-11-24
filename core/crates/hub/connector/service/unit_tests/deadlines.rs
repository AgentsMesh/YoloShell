use super::{constants::*, fixture::*, host::Host, publisher::*};
use crate::{connection, constants::OPERATION_TIMEOUT};
use std::{
    future::Future,
    task::{Context, Waker},
};
use tokio::time::{Instant, timeout_at};
use yolo_hub_connector_jsonrpc::{Answer, Call};
use yolo_hub_connector_protocol::{Command, ConnectorError, Reply};
use yolo_hub_jsonrpc_transport::Incoming;
use yolo_local_unix::LocalUnixStream;

#[tokio::test]
async fn registry_wait_timeout_distinguishes_observation_from_uncertain_mutation() {
    for mutation in [false, true] {
        let host = Host::new();
        // Retain the registry guard while the Host reply is withheld. This
        // future is deliberately not polled again until cancellation below.
        let mut held = Box::pin(
            host.fixture
                .backend
                .register_connector(uuid::Uuid::new_v4(), host.scope.clone()),
        );
        let (_host_channel, _) = tokio::select! {
            request = host.accept() => request,
            _ = held.as_mut() => panic!("{EARLY_COMPLETION}"),
        };
        let command = if mutation {
            Command::Register {
                scope: host.scope.clone(),
            }
        } else {
            Command::List {}
        };
        let request = Call::from_request(host.fixture.request(command));
        let hello = Call::from_request(host.fixture.request(Command::Hello {}));
        let (left, right) = LocalUnixStream::pair().unwrap();
        let mut channel = Channel::new(left);
        channel.send(&hello, deadline()).await.unwrap();
        channel.send(&request, deadline()).await.unwrap();
        // Buffer both frames and prime OS readiness before freezing time. A
        // single poll can now consume Hello, reply, then block in dispatch.
        right.readable().await.unwrap();
        right.writable().await.unwrap();
        let mut serving = Box::pin(connection::serve(
            right,
            host.fixture.state.clone(),
            host.fixture.backend.clone(),
        ));
        tokio::time::pause();
        let started = Instant::now();
        assert!(
            serving
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        tokio::time::advance(OPERATION_TIMEOUT).await;
        assert_eq!(Instant::now() - started, OPERATION_TIMEOUT);
        // Real socket response reads must not race automatic virtual-time
        // jumps; the dispatch timer is already expired at the exact deadline.
        tokio::time::resume();
        let read = async {
            for expected in [&hello, &request] {
                let Incoming::Message(response) =
                    channel.receive::<Answer>(deadline()).await.unwrap()
                else {
                    panic!("{UNEXPECTED_REPLY}");
                };
                let reply = if expected == &hello {
                    Reply::Hello {
                        hub: host.fixture.state.hub_id(),
                    }
                } else {
                    Reply::Error {
                        code: if mutation {
                            ConnectorError::OutcomeUnknown
                        } else {
                            ConnectorError::Unavailable
                        },
                    }
                };
                assert_eq!(response.reply(expected).unwrap(), reply);
            }
            channel.close(deadline()).await.unwrap();
        };
        let (result, ()) = timeout_at(deadline(), async { tokio::join!(serving, read) })
            .await
            .unwrap();
        result.unwrap();
        drop(held);
        assert!(
            host.fixture
                .backend
                .list_connectors()
                .await
                .unwrap()
                .is_empty()
        );
    }
}
