use super::{super::federation_support::deadline, constants::*, graph::Graph};
use yolo_hub_client::ClientError;
use yolo_hub_file_protocol::*;
use yolo_hub_identity::FileRootId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

pub async fn denied(graph: &Graph) {
    let origin = graph.c.running();
    let hub = origin.node.hub_id();
    let Reply::FileRoots {
        epoch,
        roots,
        next: None,
    } = origin
        .client()
        .await
        .call(hub, Command::ListFileRoots { after: None }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(roots.is_empty());
    let request = ReadFileRequest::new(ReadFileFields {
        schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
        target: FileRootTarget {
            address: FileRootAddress {
                hub_id: hub,
                root_id: FileRootId::new(),
            },
            epoch,
        },
        relative_path: RelativeFilePath::new(PRIVATE_MARKER.into()).unwrap(),
        offset: u64::MIN,
        length: u32::try_from(PRIVATE_CONTENT.len()).unwrap(),
        expected_version: None,
    })
    .unwrap();
    let local = origin
        .client()
        .await
        .call(
            hub,
            Command::ReadFile {
                request: request.clone(),
            },
            deadline(),
        )
        .await;
    assert!(matches!(
        local,
        Err(ClientError::Remote(PublicError::NotFound))
    ));
    for node in [&graph.a, &graph.b] {
        let denied = node
            .client()
            .await
            .call(
                hub,
                Command::ReadFile {
                    request: request.clone(),
                },
                deadline(),
            )
            .await;
        assert!(matches!(
            denied,
            Err(ClientError::Remote(PublicError::Unreachable))
        ));
    }
}
