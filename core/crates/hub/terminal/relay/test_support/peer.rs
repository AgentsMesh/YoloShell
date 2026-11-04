use super::{constants::*, fixtures::*};
use tokio::{
    net::{UnixListener, UnixStream},
    time::timeout_at,
};
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Reply, SUPPORTED_METHODS};

pub fn listener() -> (tempfile::TempDir, std::path::PathBuf, UnixListener) {
    let directory = tempfile::tempdir_in(SOCKET_ROOT).unwrap();
    let path = directory.path().join(SOCKET_NAME);
    let listener = UnixListener::bind(&path).unwrap();
    (directory, path, listener)
}
pub async fn accept(listener: &UnixListener) -> Framed<UnixStream> {
    let (stream, _) = timeout_at(deadline(), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut peer = Framed::new(stream);
    let request = receive(&mut peer).await;
    assert_eq!(request.fields().destination, hub());
    assert_eq!(request.fields().command, Command::Hello {});
    respond(
        &mut peer,
        &request,
        Reply::Hello {
            hello: Hello {
                hub_id: hub(),
                version: HUB_PROTOCOL_VERSION,
                methods: SUPPORTED_METHODS.to_vec(),
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
    if let Reply::Attached { attachment } = reply {
        yolo_hub_terminal_wire::send_attachment(peer, request, &attachment, deadline())
            .await
            .unwrap();
        return;
    }
    peer.send(&Response::new(request, reply).unwrap(), deadline())
        .await
        .unwrap();
}
pub async fn attach(listener: &UnixListener) -> Framed<UnixStream> {
    let mut peer = accept(listener).await;
    let request = receive(&mut peer).await;
    assert!(
        matches!(request.fields().command, Command::Attach { request } if request.target() == target() && request.cursor().is_none())
    );
    respond(
        &mut peer,
        &request,
        Reply::Attached {
            attachment: initial(),
        },
    )
    .await;
    peer
}
pub async fn closed(peer: &mut Framed<UnixStream>) {
    assert!(matches!(
        peer.receive::<Request>(deadline()).await,
        Ok(Incoming::Eof)
    ));
}

pub async fn observing(listener: &UnixListener) -> (Framed<UnixStream>, Framed<UnixStream>) {
    let mut control = accept(listener).await;
    let request = receive(&mut control).await;
    assert_eq!(
        request.fields().command,
        Command::ObserveLease { target: target() }
    );
    respond(
        &mut control,
        &request,
        Reply::LeaseObservation { lease: None },
    )
    .await;
    let terminal = attach(listener).await;
    (control, terminal)
}

pub async fn acquiring(
    listener: &UnixListener,
    grant: yolo_hub_terminal_protocol::LeaseGrant,
) -> (Framed<UnixStream>, Framed<UnixStream>) {
    let mut control = accept(listener).await;
    let request = receive(&mut control).await;
    assert!(
        matches!(request.fields().command, Command::Lease { request } if request.target == target() && request.principal == principal())
    );
    respond(&mut control, &request, Reply::Lease { grant }).await;
    let terminal = attach(listener).await;
    (control, terminal)
}

pub fn observation(generation: u64) -> yolo_hub_terminal_protocol::LeaseObservation {
    use yolo_hub_terminal_protocol::{LeaseObservation, LeaseObservationFields};
    LeaseObservation::new(LeaseObservationFields {
        target: target(),
        generation,
        holder_id: OTHER_HOLDER.into(),
        controller_instance_id: None,
        expires_at_ms: EXPIRES_MS,
    })
    .unwrap()
}
