mod activated;
mod connection;
#[path = "../test_support/constants.rs"]
mod constants;
mod deadlines;
mod dispatch;
#[path = "../test_support/fixture.rs"]
mod fixture;
#[path = "../test_support/host.rs"]
mod host;
#[path = "../test_support/host_view.rs"]
mod host_view;
#[path = "../test_support/publisher.rs"]
mod publisher;
mod roles;

#[test]
fn connector_socket_name_matches_the_runtime_namespace_contract() {
    assert_eq!(
        yolo_hub_connector_protocol::CONNECTOR_SOCKET_FILE,
        yolo_paths::constants::CONNECTOR_SOCKET_FILE_NAME
    );
}
