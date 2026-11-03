use crate::{ClientError, HubEndpoint};
use std::str::FromStr;

#[test]
fn local_endpoint_preserves_socket_path() {
    let endpoint = HubEndpoint::from_str("local:/tmp/yolo.sock").unwrap();
    assert_eq!(
        endpoint.local_socket().unwrap().to_str(),
        Some("/tmp/yolo.sock")
    );
}

#[test]
fn unsupported_ssh_never_becomes_a_connectable_endpoint() {
    for value in [
        "ssh://stone@example.test",
        "ssh://stone@[::1]:2200",
        "SSH://stone@example.test",
        "ssh:",
        "ssh://user:password@host?query#fragment",
        "ssh://user\n@host",
    ] {
        assert!(matches!(
            value.parse::<HubEndpoint>(),
            Err(ClientError::UnsupportedEndpoint)
        ));
    }
}

#[test]
fn malformed_endpoint_is_rejected_without_network_access() {
    for value in [
        "unix:/tmp/yolo.sock",
        "/tmp/yolo.sock",
        "local:",
        "local:/tmp/\0.sock",
        "local:/tmp/\n.sock",
        "local:/tmp/\x7f.sock",
    ] {
        assert!(matches!(
            HubEndpoint::from_str(value),
            Err(ClientError::InvalidEndpoint)
        ));
    }
}

#[tokio::test]
async fn invalid_programmatic_endpoint_is_rejected_before_connecting() {
    let endpoint = HubEndpoint::local("");
    let result = endpoint
        .connect(yolo_hub_identity::HubId::new(), tokio::time::Instant::now())
        .await;
    assert!(matches!(result, Err(ClientError::InvalidEndpoint)));
}

#[test]
fn local_paths_are_literal_and_do_not_decode_uri_characters() {
    const SOCKET: &str = "/tmp/a b%2F:#?.sock";
    let endpoint = format!("LOCAL:{SOCKET}").parse::<HubEndpoint>().unwrap();
    assert_eq!(endpoint.local_socket(), Some(std::path::Path::new(SOCKET)));
}
