use super::{channel_peer::*, constants::*};
use crate::{
    Options, channel_network,
    constants::QUEUE_CAPACITY,
    network::{Job, Update},
};
use tokio::{net::UnixListener, sync::mpsc, time::timeout_at};
use yolo_hub_client::HubEndpoint;
use yolo_hub_client_channels::ControlEvent;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn stalled_management_does_not_block_the_existing_control_channel() {
    let directory = tempfile::tempdir_in(NETWORK_TEST_ROOT).unwrap();
    let socket = directory.path().join(NETWORK_TEST_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let hub = HubId::new();
    let (jobs, incoming) = mpsc::channel(QUEUE_CAPACITY);
    let (updates, mut responses) = mpsc::channel(QUEUE_CAPACITY);
    let task = tokio::spawn(channel_network::network(
        Options {
            endpoint: HubEndpoint::local(socket),
            hub,
            layout: None,
        },
        incoming,
        updates,
    ));
    let mut task = AbortOnDrop(task);
    let (stream, _) = timeout_at(deadline(), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut control = Framed::new(stream);
    hello(&mut control, hub).await;
    let Some(Update::Control(ControlEvent::Ready { generation, .. })) = responses.recv().await
    else {
        panic!()
    };
    let token = uuid::Uuid::new_v4();
    jobs.send(Job::Management(yolo_tui_hub_manager::Operation {
        token,
        command: Command::GetConfiguration {},
    }))
    .await
    .ok()
    .unwrap();
    let (stream, _) = timeout_at(deadline(), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut management = Framed::new(stream);
    let request = receive(&mut management).await;
    respond(
        &mut management,
        &request,
        Reply::Hello {
            hello: Hello {
                hub_id: hub,
                version: HUB_PROTOCOL_VERSION,
                methods: vec![Method::Hello, Method::GetConfiguration],
            },
        },
    )
    .await;
    let pending = receive(&mut management).await;
    assert_eq!(pending.fields().command, Command::GetConfiguration {});
    jobs.send(Job::Call {
        generation: Some(generation),
        origin: hub,
        command: Command::Hello {},
    })
    .await
    .ok()
    .unwrap();
    let control_request = receive(&mut control).await;
    respond(
        &mut control,
        &control_request,
        Reply::Hello {
            hello: Hello {
                hub_id: hub,
                version: HUB_PROTOCOL_VERSION,
                methods: vec![Method::Hello],
            },
        },
    )
    .await;
    let Some(Update::Control(ControlEvent::Finished { result, .. })) =
        timeout_at(deadline(), responses.recv()).await.unwrap()
    else {
        panic!()
    };
    assert!(result.is_ok());
    respond(
        &mut management,
        &pending,
        Reply::Error {
            code: PublicError::Unreachable,
        },
    )
    .await;
    let Some(Update::Management { operation, result }) =
        timeout_at(deadline(), responses.recv()).await.unwrap()
    else {
        panic!()
    };
    assert_eq!(operation.token, token);
    assert_eq!(
        result,
        Err(yolo_hub_management_client::ManagementError::Unavailable)
    );
    drop(jobs);
    timeout_at(deadline(), &mut task.0).await.unwrap().unwrap();
}
struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
