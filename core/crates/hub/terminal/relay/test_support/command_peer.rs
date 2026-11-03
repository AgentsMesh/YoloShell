use super::{agent::*, fixtures::*, peer::*};
use tokio::{
    net::UnixStream,
    sync::{mpsc, oneshot},
    time::timeout_at,
};
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_relay_control_protocol::{Request, Response};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::LeaseGrant;
use yolo_hub_terminal_relay::{COMMAND_QUEUE_CAPACITY, CommandChannel, ControlCommand};

pub fn channel() -> (mpsc::Sender<ControlCommand>, CommandChannel) {
    let (send, incoming) = mpsc::channel(COMMAND_QUEUE_CAPACITY);
    (
        send,
        CommandChannel {
            launch_id: launch(),
            incoming,
        },
    )
}

pub async fn enqueue(
    sender: &mpsc::Sender<ControlCommand>,
    request: Request,
) -> oneshot::Receiver<Response> {
    let (response, receive) = oneshot::channel();
    sender
        .send(ControlCommand { request, response })
        .await
        .unwrap();
    receive
}

pub async fn reply(receiver: oneshot::Receiver<Response>, request: &Request) -> Response {
    let response = timeout_at(deadline(), receiver).await.unwrap().unwrap();
    response.validate_for(request).unwrap();
    response
}

pub async fn exact_release(peer: &mut Framed<UnixStream>, grant: LeaseGrant) {
    let release = receive(peer).await;
    assert_eq!(release.fields().command, Command::Release { grant });
    respond(peer, &release, Reply::Completed {}).await;
    closed(peer).await;
}
