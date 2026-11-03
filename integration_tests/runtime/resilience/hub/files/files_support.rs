use super::{federation_support::*, files_constants::*, management_support::ManagedNode};
use std::path::Path;
use yolo_hub_federation_settings::{Configuration, ExportPolicy};
use yolo_hub_file_client::{FileRoots, list_roots};
use yolo_hub_file_protocol::*;
use yolo_hub_identity::{FileRootId, HubId};
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

pub fn configure_root(
    base: &Path,
    directory: &str,
    root_id: FileRootId,
    contents: &[u8],
) -> FileRootConfiguration {
    let path = base.join(directory);
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join(RELATIVE_FILE), contents).unwrap();
    FileRootConfiguration::new(FileRootFields {
        root_id,
        name: ROOT_NAME.into(),
        path: path.to_str().unwrap().into(),
    })
    .unwrap()
}
pub fn file_export(origin: HubId, relay: bool) -> ExportPolicy {
    let mut policy = exports(origin, relay);
    policy.rights = policy
        .rights
        .union(&[Right::FileRead].into_iter().collect());
    policy
}
pub fn request(root: &FileRootSummary) -> ReadFileRequest {
    ReadFileRequest::new(ReadFileFields {
        schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
        target: root.fields().target,
        relative_path: RelativeFilePath::new(RELATIVE_FILE.into()).unwrap(),
        offset: 0,
        length: READ_LENGTH,
        expected_version: None,
    })
    .unwrap()
}
pub async fn roots(entry: &RunningNode, origin: HubId) -> FileRoots {
    tokio::time::timeout_at(deadline(), async {
        loop {
            if let Ok(roots) = list_roots(&mut entry.client().await, origin, deadline()).await {
                return roots;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}
pub async fn bytes(entry: &RunningNode, root: &FileRootSummary) -> ReadFileResult {
    yolo_hub_file_client::read(&mut entry.client().await, request(root), deadline())
        .await
        .unwrap()
}
pub async fn protected_preflight_keeps_live_configuration(node: &ManagedNode, private: &Path) {
    let before = node.snapshot().await;
    let mut invalid = before.state.configuration.clone();
    let mut root = invalid.file_roots[0].fields().clone();
    root.path = private.to_str().unwrap().into();
    invalid.file_roots[0] = FileRootConfiguration::new(root).unwrap();
    let change = yolo_hub_management_protocol::ConfigurationChange {
        request_id: uuid::Uuid::new_v4(),
        expected_revision: before.state.revision,
        configuration: invalid,
    };
    assert!(matches!(
        node.client()
            .await
            .call(node.hub, Command::Configure { change }, deadline())
            .await,
        Err(yolo_hub_client::ClientError::Remote(
            PublicError::Unauthorized
        ))
    ));
    assert_eq!(node.snapshot().await, before);
}
pub fn enable_files(
    configuration: &mut Configuration,
    recipient: HubId,
    origin: HubId,
    enabled: bool,
) {
    let policy = configuration
        .peers
        .iter_mut()
        .find(|peer| peer.identity.hub() == recipient)
        .unwrap()
        .exports
        .iter_mut()
        .find(|policy| policy.origin == origin)
        .unwrap();
    let depth = policy.remaining_reexports;
    *policy = if enabled {
        file_export(origin, false)
    } else {
        exports(origin, false)
    };
    policy.remaining_reexports = depth;
}
pub async fn visible_session(entry: &RunningNode, target: yolo_hub_identity::RuntimeTarget) {
    assert!(matches!(
        entry
            .client()
            .await
            .call(
                target.address.hub_id,
                Command::Get {
                    address: target.address
                },
                deadline()
            )
            .await,
        Ok(Reply::Terminal { .. })
    ));
}
