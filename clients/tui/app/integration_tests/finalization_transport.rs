use super::{
    channel_peer::*,
    constants::*,
    creation_support::{Fixture, create},
};
use crate::{Options, channel_network, events, network::Update};
use tokio::{net::UnixListener, sync::mpsc, time::timeout_at};
use yolo_hub_client::HubEndpoint;
use yolo_hub_client_channels::ControlEvent;
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};

#[tokio::test]
async fn retained_finalize_traverses_its_owned_channel_with_attempt_identity_intact() {
    let root = tempfile::tempdir_in(NETWORK_TEST_ROOT).unwrap();
    let socket = root.path().join(NETWORK_TEST_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let mut fixture = Fixture::new();
    let hub = fixture.origin;
    let (jobs, incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let (updates, mut responses) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let worker = AbortOnDrop(tokio::spawn(channel_network::network(
        Options {
            endpoint: HubEndpoint::local(socket),
            hub,
            layout: None,
        },
        incoming,
        updates,
    )));
    let (stream, _) = timeout_at(deadline(), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut control = Framed::new(stream);
    handshake(&mut control, hub).await;
    let Some(Update::Control(ControlEvent::Ready { generation, .. })) =
        timeout_at(deadline(), responses.recv()).await.unwrap()
    else {
        panic!()
    };
    fixture.state.control_generation = Some(generation);
    let admitted = fixture.start();
    fixture.jobs = jobs;
    let created = fixture.created();
    crate::creation_events::created(
        &mut fixture.state,
        &fixture.jobs,
        create(&admitted).clone(),
        created,
    );
    let (stream, _) = timeout_at(deadline(), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut operation = Framed::new(stream);
    handshake(&mut operation, hub).await;
    let request = receive(&mut operation).await;
    assert_eq!(
        request.fields().command,
        Command::Finalize {
            creation_token: create(&admitted).fields().creation_token,
            creation: created,
        }
    );
    respond(&mut operation, &request, Reply::Completed {}).await;
    let update = timeout_at(deadline(), responses.recv())
        .await
        .unwrap()
        .unwrap();
    let Update::Control(ControlEvent::Finished {
        request,
        result,
        connected,
    }) = update
    else {
        panic!()
    };
    assert!(connected);
    assert!(request.validity.as_ref().unwrap().is_current());
    assert!(
        fixture.state.creations.finalizing[&create(&admitted).fields().creation_token]
            .matches_attempt(&request)
    );
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Finished {
            request,
            result,
            connected,
        }),
    );
    assert!(fixture.state.creations.finalizing.is_empty());
    assert!(fixture.state.writers.is_empty());
    assert_eq!(fixture.original_binding(), Some(created.address));
    worker.0.abort();
}

async fn handshake(peer: &mut Peer, hub_id: yolo_hub_identity::HubId) {
    let request = receive(peer).await;
    assert!(matches!(request.fields().command, Command::Hello {}));
    respond(
        peer,
        &request,
        Reply::Hello {
            hello: Hello {
                hub_id,
                version: HUB_PROTOCOL_VERSION,
                methods: vec![Method::Hello, Method::Finalize],
            },
        },
    )
    .await;
}

struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
