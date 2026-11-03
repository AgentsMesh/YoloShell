use crate::map_error;
use yolo_client::Client;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Hello, Method, Reply, SUPPORTED_METHODS};
use yolo_hub_terminal_protocol::PublicError;

/// Build the origin-dispatched greeting from the methods this Hub supports.
pub async fn hello(
    client: &Client,
    hub: HubId,
    supports: impl Fn(Method) -> bool,
) -> Result<Reply, PublicError> {
    client.status().await.map_err(map_error)?;
    Ok(Reply::Hello {
        hello: Hello {
            hub_id: hub,
            version: HUB_PROTOCOL_VERSION,
            methods: SUPPORTED_METHODS
                .iter()
                .copied()
                .filter(|method| supports(*method))
                .collect(),
        },
    })
}
