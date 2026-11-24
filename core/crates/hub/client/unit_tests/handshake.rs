use super::{
    fixtures::{constants::*, *},
    server::*,
};
use crate::{Client, ClientError};
use tokio::{io::duplex, time::Instant};
use yolo_hub_jsonrpc_protocol::Answer as Response;
use yolo_hub_jsonrpc_transport::{Framed, Incoming, TransportError};
use yolo_hub_rpc_protocol::{
    Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply, SUPPORTED_METHODS,
};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn hello_advertises_exact_methods_and_channel_can_transfer_ownership() {
    let (client, mut server) = connected().await;
    assert_eq!(client.hello().hub_id, hub());
    assert_eq!(client.hello().methods.as_slice(), SUPPORTED_METHODS);
    assert!(client.is_usable());
    let mut channel = client.into_channel().unwrap();
    channel.close(deadline()).await.unwrap();
    assert_eq!(
        server.receive::<Response>(deadline()).await.unwrap(),
        Incoming::Eof
    );
}

#[tokio::test]
async fn handshake_rejects_remote_error_and_mismatched_payload_without_creating_client() {
    for wrong_identity in [false, true] {
        let (left, right) = duplex(PIPE_CAPACITY);
        let peer = async {
            let mut server = Framed::new(right);
            let request = receive(&mut server).await;
            let reply = if wrong_identity {
                Reply::Hello {
                    hello: Hello {
                        hub_id: other_hub(),
                        version: HUB_PROTOCOL_VERSION,
                        methods: vec![Method::Hello],
                    },
                }
            } else {
                Reply::Error {
                    code: PublicError::Unauthorized,
                }
            };
            respond(&mut server, &request, reply).await;
        };
        let (result, ()) = tokio::join!(Client::open(left, hub(), deadline()), peer);
        let error = result.err().unwrap();
        if wrong_identity {
            assert!(matches!(
                error,
                ClientError::InvalidResponse(PublicError::InvalidPayload)
            ));
        } else {
            assert!(matches!(
                error,
                ClientError::Remote(PublicError::Unauthorized)
            ));
        }
        assert!(!error.outcome_is_unknown());
    }
}

#[tokio::test]
async fn handshake_requires_a_complete_response_and_respects_preflight_deadline() {
    let (left, right) = duplex(PIPE_CAPACITY);
    drop(right);
    assert!(matches!(
        Client::open(left, hub(), deadline()).await,
        Err(ClientError::Transport(TransportError::Io(_)))
    ));
    let (left, _right) = duplex(PIPE_CAPACITY);
    assert!(matches!(
        Client::open(left, hub(), Instant::now()).await,
        Err(ClientError::Transport(TransportError::Deadline))
    ));
}

#[tokio::test]
async fn unsupported_and_validation_preflight_leave_transport_reusable() {
    let (mut client, mut server) = connected_with(vec![Method::Hello]).await;
    assert!(matches!(
        client.call(hub(), list(), deadline()).await,
        Err(ClientError::Unsupported)
    ));
    assert!(client.is_usable());
    let peer = async {
        hello(&mut server, vec![Method::Hello]).await;
    };
    let (reply, ()) = tokio::join!(client.call(hub(), Command::Hello {}, deadline()), peer);
    assert!(matches!(reply, Ok(Reply::Hello { .. })));
    let (mut client, mut server) = connected().await;
    assert!(matches!(
        client.call(hub(), invalid(), deadline()).await,
        Err(ClientError::Validation(PublicError::InvalidRequest))
    ));
    assert!(matches!(
        client.call(other_hub(), input(), deadline()).await,
        Err(ClientError::Validation(PublicError::OriginMismatch))
    ));
    let peer = async {
        let request = receive(&mut server).await;
        assert_eq!(request.fields().command, list());
        respond(
            &mut server,
            &request,
            Reply::Terminals {
                terminals: Vec::new(),
                next: None,
            },
        )
        .await;
    };
    let (reply, ()) = tokio::join!(client.call(hub(), list(), deadline()), peer);
    assert!(matches!(reply, Ok(Reply::Terminals { .. })));
}
