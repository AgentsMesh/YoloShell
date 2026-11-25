use tokio::{
    net::{UnixListener, UnixStream},
    time::{Instant, timeout_at},
};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};

const ROOT: &str = "/tmp";
const SOCKET: &str = "management.sock";
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
pub const NO_RETRY_WAIT: std::time::Duration = std::time::Duration::from_millis(40);
pub fn deadline() -> Instant {
    Instant::now() + TIMEOUT
}
pub fn listener() -> (tempfile::TempDir, std::path::PathBuf, UnixListener) {
    let directory = tempfile::tempdir_in(ROOT).unwrap();
    let socket = directory.path().join(SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    (directory, socket, listener)
}
pub async fn accept(
    listener: &UnixListener,
    hub: HubId,
    methods: Vec<Method>,
) -> Framed<UnixStream> {
    let (stream, _) = timeout_at(deadline(), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut peer = Framed::new(stream);
    let request = receive(&mut peer).await;
    assert_eq!(request.fields().command, Command::Hello {});
    respond(
        &mut peer,
        &request,
        Reply::Hello {
            hello: Hello {
                hub_id: hub,
                version: HUB_PROTOCOL_VERSION,
                methods,
            },
        },
    )
    .await;
    peer
}
pub async fn receive(peer: &mut Framed<UnixStream>) -> Request {
    let Incoming::Message(request) = peer.receive::<Request>(deadline()).await.unwrap() else {
        panic!()
    };
    request
}
pub async fn respond(peer: &mut Framed<UnixStream>, request: &Request, reply: Reply) {
    peer.send(&Response::new(request, reply).unwrap(), deadline())
        .await
        .unwrap();
}
pub async fn closed(peer: &mut Framed<UnixStream>) {
    assert!(matches!(
        peer.receive::<Request>(deadline()).await,
        Ok(Incoming::Eof)
    ));
}
pub async fn no_retry(listener: &UnixListener) {
    assert!(
        tokio::time::timeout(NO_RETRY_WAIT, listener.accept())
            .await
            .is_err()
    );
}
