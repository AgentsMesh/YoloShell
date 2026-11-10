use crate::{ClientError, socket_for_hub};
use std::path::Path;
use yolo_hub_connector_protocol::{CONNECTOR_SOCKET_FILE, ConnectorError};

const HUB_SOCKET: &str = "/private/tmp/yolo namespace/hub.sock";
#[test]
fn connector_endpoint_uses_the_discovered_hub_namespace_without_home_assumptions() {
    let hub = Path::new(HUB_SOCKET);
    assert_eq!(
        socket_for_hub(hub).unwrap(),
        hub.parent().unwrap().join(CONNECTOR_SOCKET_FILE)
    );
    for invalid in [
        Path::new(""),
        Path::new("relative/hub.sock"),
        Path::new("/"),
    ] {
        assert!(matches!(
            socket_for_hub(invalid),
            Err(ClientError::Validation(ConnectorError::InvalidRequest))
        ));
    }
}
