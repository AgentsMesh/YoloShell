use crate::{
    Client, ClientError,
    fixtures::{constants::*, *},
    server::*,
};
use tokio::{
    net::UnixListener,
    time::{Instant, timeout_at},
};
use yolo_hub_connector_protocol::Reply;
use yolo_hub_jsonrpc_transport::{Framed, TransportError};

#[tokio::test]
async fn local_peer_handshake_activation_and_publication_share_one_authenticated_connection() {
    let directory = tempfile::tempdir_in(SOCKET_ROOT).unwrap();
    let path = directory.path().join(SOCKET_NAME);
    let listener = UnixListener::bind(&path).unwrap();
    let peer = async {
        let (stream, _) = timeout_at(deadline(), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut server = Framed::new(stream);
        handshake(&mut server).await;
        let request = receive(&mut server).await;
        assert_eq!(request, activation_request());
        respond(
            &mut server,
            &request,
            Reply::Activated {
                activation: activation(),
            },
        )
        .await;
        let request = receive(&mut server).await;
        assert_eq!(request, publication_request());
        respond(
            &mut server,
            &request,
            Reply::Published {
                ack: acknowledgement(false),
            },
        )
        .await;
        eof(&mut server).await;
    };
    let caller = async {
        let mut client = Client::connect_local(&path, hub(), deadline())
            .await
            .unwrap();
        assert_eq!(
            client.call(activation_request(), deadline()).await.unwrap(),
            Reply::Activated {
                activation: activation()
            }
        );
        assert_eq!(
            client
                .call(publication_request(), deadline())
                .await
                .unwrap(),
            Reply::Published {
                ack: acknowledgement(false)
            }
        );
        client.close(deadline()).await.unwrap();
    };
    tokio::join!(caller, peer);
    assert!(
        timeout_at(Instant::now() + SHORT_TIMEOUT, listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn same_user_socket_cannot_substitute_another_hub_identity() {
    let directory = tempfile::tempdir_in(SOCKET_ROOT).unwrap();
    let path = directory.path().join(SOCKET_NAME);
    let listener = UnixListener::bind(&path).unwrap();
    let peer = async {
        let (stream, _) = timeout_at(deadline(), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut server = Framed::new(stream);
        let request = receive(&mut server).await;
        respond(&mut server, &request, Reply::Hello { hub: other_hub() }).await;
    };
    let (result, ()) = tokio::join!(Client::connect_local(&path, hub(), deadline()), peer);
    let error = result.err().unwrap();
    assert!(!error.outcome_is_unknown());
}

#[tokio::test]
async fn missing_socket_and_expired_local_deadline_are_known_not_delivered() {
    let directory = tempfile::tempdir_in(SOCKET_ROOT).unwrap();
    let path = directory.path().join(ABSENT_SOCKET);
    let error = Client::connect_local(&path, hub(), deadline())
        .await
        .err()
        .unwrap();
    assert!(matches!(
        error,
        ClientError::Transport(TransportError::Io(_))
    ));
    assert!(!error.outcome_is_unknown());
    let error = Client::connect_local(&path, hub(), Instant::now())
        .await
        .err()
        .unwrap();
    assert!(matches!(
        error,
        ClientError::Transport(TransportError::Deadline)
    ));
    assert!(!error.outcome_is_unknown());
}
