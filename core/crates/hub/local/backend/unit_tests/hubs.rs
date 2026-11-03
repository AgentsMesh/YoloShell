use super::fixtures::{constants::*, *};
use crate::LocalBackend;
use yolo_hub_dispatch::{Dispatcher, Operation};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, HubSummary, Method, Reply, Request, Response};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn local_hub_inventory_needs_no_live_host_or_agent_process() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    let socket = path.join(ABSENT_HOST_SOCKET);
    let local = target().address.hub_id;
    let backend = LocalBackend::new(local, &path, &socket).unwrap();
    assert!(backend.supports(Method::ListHubs));
    let request = Request::new(local, Command::ListHubs { after: None }).unwrap();
    let Operation::Reply(reply) = backend.dispatch(request.clone()).await.unwrap() else {
        panic!()
    };
    assert_eq!(
        reply,
        Reply::Hubs {
            hubs: vec![HubSummary {
                hub_id: local,
                reachable: true,
                label: None,
            }],
            next: None
        }
    );
    assert!(
        Response::new(&request, local, reply)
            .validate_for(&request)
            .is_ok()
    );
    assert!(!socket.exists());
}

#[tokio::test]
async fn origin_backend_reports_only_self_and_respects_the_hub_cursor() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    let local = HubId::from_uuid(uuid::Uuid::from_u128(SESSION_VALUE)).unwrap();
    let before = HubId::from_uuid(uuid::Uuid::from_u128(HUB_VALUE)).unwrap();
    let after = HubId::from_uuid(uuid::Uuid::from_u128(EPOCH_VALUE)).unwrap();
    let backend = LocalBackend::new(local, &path, &path.join(ABSENT_HOST_SOCKET)).unwrap();
    for cursor in [None, Some(before), Some(local), Some(after)] {
        let request = Request::new(local, Command::ListHubs { after: cursor }).unwrap();
        let Operation::Reply(reply) = backend.dispatch(request).await.unwrap() else {
            panic!()
        };
        let Reply::Hubs { hubs, next } = reply else {
            panic!()
        };
        assert_eq!(
            hubs.is_empty(),
            cursor.is_some_and(|cursor| cursor >= local)
        );
        assert!(hubs.iter().all(|hub| hub.hub_id == local && hub.reachable));
        assert_eq!(next, None);
    }
    let remote = Request::new(after, Command::ListHubs { after: None }).unwrap();
    assert!(matches!(
        backend.dispatch(remote).await,
        Err(PublicError::Unreachable)
    ));
}
