use crate::{
    Client, ClientError,
    fixtures::{constants::*, *},
    server::*,
};
use tokio::{
    net::UnixListener,
    time::{Instant, timeout_at},
};
use yolo_hub_client::HubEndpoint;
use yolo_hub_jsonrpc_transport::{Framed, TransportError};
use yolo_hub_rpc_protocol::{Reply, SUPPORTED_METHODS};

#[tokio::test]
async fn same_user_unix_peer_authenticates_and_transmits_exact_mutation_once() {
    let directory = tempfile::tempdir_in(SOCKET_ROOT).unwrap();
    let path = directory.path().join(SOCKET_NAME);
    let listener = UnixListener::bind(&path).unwrap();
    let peer = async {
        let (stream, _) = timeout_at(deadline(), listener.accept())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stream.peer_cred().unwrap().uid(), unsafe {
            libc::geteuid()
        });
        let mut server = Framed::new(stream);
        hello(&mut server, SUPPORTED_METHODS.to_vec()).await;
        let request = receive(&mut server).await;
        assert_eq!(request.fields().command, input());
        assert_eq!(request.fields().request_id, id(REQUEST_ID));
        respond(&mut server, &request, Reply::Input { duplicate: false }).await;
        assert!(matches!(
            server
                .receive::<yolo_hub_jsonrpc_protocol::Call>(deadline())
                .await,
            Ok(yolo_hub_jsonrpc_transport::Incoming::Eof)
        ));
    };
    let caller = async {
        let endpoint: HubEndpoint = format!("local:{}", path.display()).parse().unwrap();
        let mut client = endpoint.connect(hub(), deadline()).await.unwrap();
        assert!(matches!(
            client.call(hub(), input(), deadline()).await,
            Ok(Reply::Input { duplicate: false })
        ));
        client.close(deadline()).await.unwrap();
    };
    tokio::join!(caller, peer);
}

#[tokio::test]
async fn a_same_user_socket_still_cannot_substitute_the_pinned_hub_identity() {
    let directory = tempfile::tempdir_in(SOCKET_ROOT).unwrap();
    let path = directory.path().join(SOCKET_NAME);
    let listener = UnixListener::bind(&path).unwrap();
    let peer = async {
        let (stream, _) = timeout_at(deadline(), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut server = Framed::new(stream);
        hello(&mut server, SUPPORTED_METHODS.to_vec()).await;
    };
    let endpoint = HubEndpoint::local(&path);
    let (result, ()) = tokio::join!(endpoint.connect(other_hub(), deadline()), peer);
    assert!(matches!(result, Err(ClientError::InvalidResponse(_))));
}

#[tokio::test]
async fn missing_socket_and_expired_deadline_cannot_report_a_delivered_operation() {
    let directory = tempfile::tempdir_in(SOCKET_ROOT).unwrap();
    let absent = directory.path().join(ABSENT_SOCKET_NAME);
    let error = Client::connect_local(&absent, hub(), deadline())
        .await
        .err()
        .unwrap();
    assert!(matches!(
        error,
        ClientError::Transport(TransportError::Io(_))
    ));
    assert!(!error.outcome_is_unknown());
    let path = directory.path().join(SOCKET_NAME);
    let _listener = UnixListener::bind(&path).unwrap();
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
