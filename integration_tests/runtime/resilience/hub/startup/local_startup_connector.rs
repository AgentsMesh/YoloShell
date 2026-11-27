use super::super::local_startup_support::deadline;
use yolo_hub_connector_client::{Client, socket_for_hub};
use yolo_hub_connector_protocol::{Command, Reply, Request};
use yolo_hub_local_startup::ConnectionDocument;

pub async fn assert_empty(document: &ConnectionDocument) {
    let socket = socket_for_hub(&document.socket_path).unwrap();
    assert_eq!(socket.parent(), document.socket_path.parent());
    assert_ne!(socket, document.socket_path);
    let hub = document.entry_hub;
    let mut client = Client::connect_local(&socket, hub, deadline())
        .await
        .unwrap();
    assert_eq!(client.hub_id(), hub);
    let hello = client
        .call(Request::new(hub, Command::Hello {}).unwrap(), deadline())
        .await
        .unwrap();
    assert_eq!(hello, Reply::Hello { hub });
    let Reply::Enrollments { enrollments } = client
        .call(Request::new(hub, Command::List {}).unwrap(), deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(enrollments.is_empty());
    client.close(deadline()).await.unwrap();
}
