use crate::LocalBackend;
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply, Request, Response};
use yolo_paths::directory::PrivateDirectory;
use yolo_presentation_protocol::{LayoutAddress, LayoutUpdate};

const UNUSED_SOCKET: &str = "missing.sock";
const PRIVATE_LAYOUT_DIRECTORY: &str = "layout-root";

#[tokio::test]
async fn out_of_order_layout_creation_returns_a_protocol_valid_listing() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap();
    let hub = HubId::new();
    let backend = LocalBackend::new(hub, &path, &path.join(UNUSED_SOCKET))
        .unwrap()
        .with_layouts(&PrivateDirectory::ensure(&path.join(PRIVATE_LAYOUT_DIRECTORY)).unwrap())
        .unwrap();
    let mut ids = [Uuid::new_v4(), Uuid::new_v4()];
    ids.sort_by_key(|id| std::cmp::Reverse(*id));
    for id in ids {
        backend
            .execute(&Command::UpdateLayout {
                change: LayoutUpdate {
                    address: LayoutAddress {
                        hub_id: hub,
                        layout_id: id,
                    },
                    request_id: Uuid::new_v4(),
                    expected_revision: 0,
                    tabs: vec![],
                },
            })
            .await
            .unwrap();
    }
    let request = Request::new(hub, Command::ListLayouts {}).unwrap();
    let reply = backend.execute(&request.fields().command).await.unwrap();
    Response::new(&request, hub, reply.clone())
        .validate_for(&request)
        .unwrap();
    let Reply::Layouts { documents } = reply else {
        panic!()
    };
    assert!(
        documents
            .windows(2)
            .all(|pair| pair[0].address < pair[1].address)
    );
    assert!(
        documents
            .iter()
            .all(|summary| summary.tab_count == 0 && summary.pane_count == 0)
    );
}
