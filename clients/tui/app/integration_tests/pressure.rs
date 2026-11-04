use super::{channel_peer::*, constants::*};
use crate::{
    Options, channel_network,
    constants::QUEUE_CAPACITY,
    network::{Job, Update},
};
use tokio::{net::UnixListener, sync::mpsc, time::timeout_at};
use uuid::Uuid;
use yolo_hub_client::HubEndpoint;
use yolo_hub_client_channels::ControlEvent;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn full_control_queue_does_not_block_catalogue_dispatch_in_the_real_ui_adapter() {
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
    let call = || Job::Call {
        generation: Some(generation),
        origin: hub,
        command: Command::Hello {},
    };
    jobs.send(call()).await.ok().unwrap();
    let blocked = receive(&mut control).await;
    assert!(matches!(blocked.fields().command, Command::Hello {}));
    for _ in 0..QUEUE_CAPACITY + QUEUE_OVERFLOW {
        jobs.send(call()).await.ok().unwrap();
    }
    let request_id = Uuid::new_v4();
    jobs.send(Job::Refresh { request_id }).await.ok().unwrap();
    let (stream, _) = timeout_at(deadline(), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut catalog = Framed::new(stream);
    hello(&mut catalog, hub).await;
    let request = receive(&mut catalog).await;
    assert!(matches!(request.fields().command, Command::List { .. }));
    respond(
        &mut catalog,
        &request,
        Reply::Terminals {
            terminals: vec![],
            next: None,
        },
    )
    .await;
    let Some(Update::Catalog(event)) = timeout_at(deadline(), responses.recv()).await.unwrap()
    else {
        panic!()
    };
    assert_eq!(event.request_id, request_id);
    assert!(event.result.is_ok());
    // No control reply was supplied; the directory still completed through the App adapter.
    task.0.abort();
    assert!((&mut task.0).await.unwrap_err().is_cancelled());
}

struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
