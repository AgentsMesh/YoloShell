use super::{federation_support::*, management_support::ManagedNode};
use std::path::{Path, PathBuf};
use yolo_hub_file_client::FileRoots;
use yolo_hub_file_executor_test_support::{
    constants::{GATED_READ, READ_READY_FILE},
    proof::Proof,
};
use yolo_hub_identity::HubId;

const FILE_FAULT_ENV: &str = "YOLO_TEST_FILE_FAULT_BINARY";

pub fn gated_program(directory: &Path) -> PathBuf {
    let source = crate::bazel_runfile(&std::env::var(FILE_FAULT_ENV).unwrap()).unwrap();
    let target = directory.join(GATED_READ);
    std::fs::copy(source, &target).unwrap();
    target
}
pub async fn wait_roots(node: &ManagedNode, origin: HubId) -> FileRoots {
    tokio::time::timeout_at(deadline(), async {
        loop {
            if let Ok(roots) =
                yolo_hub_file_client::list_roots(&mut node.client().await, origin, deadline()).await
            {
                return roots;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}
pub async fn wait_read(directory: &Path) -> Proof {
    tokio::time::timeout_at(deadline(), async {
        loop {
            if let Ok(bytes) = std::fs::read(directory.join(READ_READY_FILE)) {
                if let Ok(proof) = serde_json::from_slice(&bytes) {
                    return proof;
                }
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}
pub async fn wait_gone(proof: Proof) {
    tokio::time::timeout_at(deadline(), async {
        while !proof.gone() {
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}
