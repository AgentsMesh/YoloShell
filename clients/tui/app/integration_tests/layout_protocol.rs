use super::{channel_peer::*, constants::*};
use crate::{Options, layout_sync::startup::open};
use tokio::net::UnixListener;
use uuid::Uuid;
use yolo_hub_client::HubEndpoint;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};
use yolo_presentation_protocol::{LayoutUpdate, SharedLayoutDocument};
use yolo_tui_tab::TabStore;

const CLIENT_DIRECTORY: &str = "client";

async fn connection(listener: &UnixListener, hub: HubId) -> Peer {
    let (stream, _) = tokio::time::timeout_at(deadline(), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut peer = Framed::new(stream);
    let hello = receive(&mut peer).await;
    assert!(matches!(hello.fields().command, Command::Hello {}));
    respond(
        &mut peer,
        &hello,
        Reply::Hello {
            hello: Hello {
                hub_id: hub,
                version: HUB_PROTOCOL_VERSION,
                methods: vec![Method::Hello, Method::GetLayout, Method::UpdateLayout],
            },
        },
    )
    .await;
    peer
}

#[tokio::test]
async fn new_client_publishes_layout_then_another_client_attaches_via_public_protocol() {
    let root = tempfile::tempdir_in(NETWORK_TEST_ROOT).unwrap();
    let socket = root.path().join(NETWORK_TEST_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let hub = HubId::new();
    let server = tokio::spawn(async move {
        let mut peer = connection(&listener, hub).await;
        let read = receive(&mut peer).await;
        let Command::GetLayout { address } = read.fields().command else {
            panic!()
        };
        respond(&mut peer, &read, Reply::Layout { document: None }).await;
        let mut peer = connection(&listener, hub).await;
        let create = receive(&mut peer).await;
        let Command::UpdateLayout { ref change } = create.fields().command else {
            panic!()
        };
        assert_eq!(change.address, address);
        let created = change.apply(None).unwrap();
        respond(
            &mut peer,
            &create,
            Reply::Layout {
                document: Some(created.clone()),
            },
        )
        .await;
        let mut peer = connection(&listener, hub).await;
        let read = receive(&mut peer).await;
        assert_eq!(read.fields().command, Command::GetLayout { address });
        respond(
            &mut peer,
            &read,
            Reply::Layout {
                document: Some(created.clone()),
            },
        )
        .await;
        created
    });
    let mut store = TabStore::open(&root.path().join(CLIENT_DIRECTORY), hub).unwrap();
    let options = Options {
        endpoint: HubEndpoint::local(socket),
        hub,
        layout: None,
    };
    let mut state = crate::tab_persistence::restore(&store);
    let private = state.tab_layout().focused();
    open(&options, &mut store, &mut state).await.unwrap();
    assert_eq!(state.tab_layout().focused(), private);
    crate::tab_persistence::save(&mut store, &state).unwrap();
    assert!(store.document().shared.is_some());
    let mut other = crate::client_capability_support::state();
    let options = Options {
        layout: Some(store.document().layout_address.layout_id),
        ..options
    };
    open(&options, &mut store, &mut other).await.unwrap();
    let published = server.await.unwrap();
    assert_eq!(other.tab_snapshot().shared_tabs().unwrap(), published.tabs);
    assert!(other.terminals.is_empty());
    assert!(other.writers.is_empty());
}

#[tokio::test]
async fn explicit_missing_layout_never_creates_replacement_or_changes_private_identity() {
    let root = tempfile::tempdir_in(NETWORK_TEST_ROOT).unwrap();
    let socket = root.path().join(NETWORK_TEST_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let hub = HubId::new();
    let server = tokio::spawn(async move {
        let mut peer = connection(&listener, hub).await;
        let read = receive(&mut peer).await;
        respond(&mut peer, &read, Reply::Layout { document: None }).await;
    });
    let mut store = TabStore::open(&root.path().join(CLIENT_DIRECTORY), hub).unwrap();
    let before = store.document().clone();
    let options = Options {
        endpoint: HubEndpoint::local(socket),
        hub,
        layout: Some(Uuid::new_v4()),
    };
    assert!(
        open(
            &options,
            &mut store,
            &mut crate::client_capability_support::state()
        )
        .await
        .is_err()
    );
    assert_eq!(store.document(), &before);
    server.await.unwrap();
}

#[tokio::test]
async fn independent_layout_lane_delivers_correlated_public_rpc_result() {
    let root = tempfile::tempdir_in(NETWORK_TEST_ROOT).unwrap();
    let socket = root.path().join(NETWORK_TEST_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let hub = HubId::new();
    let server = tokio::spawn(async move {
        let mut peer = connection(&listener, hub).await;
        let request = receive(&mut peer).await;
        respond(&mut peer, &request, Reply::Layout { document: None }).await;
    });
    let document: SharedLayoutDocument = LayoutUpdate {
        address: yolo_presentation_protocol::LayoutAddress {
            hub_id: hub,
            layout_id: Uuid::new_v4(),
        },
        request_id: Uuid::new_v4(),
        expected_revision: 0,
        tabs: vec![],
    }
    .apply(None)
    .unwrap();
    let options = Options {
        endpoint: HubEndpoint::local(socket),
        hub,
        layout: None,
    };
    let operation = crate::layout_sync::Operation {
        token: Uuid::new_v4(),
        command: Command::GetLayout {
            address: document.address,
        },
    };
    let (jobs, incoming) = tokio::sync::mpsc::channel(1);
    let (updates, mut responses) = tokio::sync::mpsc::channel(1);
    jobs.send(operation.clone()).await.unwrap();
    drop(jobs);
    crate::layout_sync::network::run(&options, incoming, &updates).await;
    let crate::network::Update::Layout {
        operation: completed,
        result,
    } = responses.recv().await.unwrap()
    else {
        panic!()
    };
    assert_eq!(completed.token, operation.token);
    assert_eq!(result.unwrap(), Reply::Layout { document: None });
    server.await.unwrap();
}
