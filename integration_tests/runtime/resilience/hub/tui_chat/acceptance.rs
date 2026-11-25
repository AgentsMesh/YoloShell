use super::constants::POLL;
use crate::hub::support::deadline;
use std::path::Path;
use yolo_hub_identity::RuntimeTarget;
use yolo_runtime::model::TerminalView;

pub async fn submissions(marker: &Path, expected: &[String]) {
    tokio::time::timeout_at(deadline(), async {
        loop {
            let bytes = std::fs::read(marker).unwrap();
            let submitted = super::submissions::committed(&bytes).unwrap();
            assert!(submitted.len() <= expected.len());
            assert_eq!(submitted, expected[..submitted.len()]);
            if submitted == expected {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}

pub async fn same_processes(host: &crate::Harness, target: RuntimeTarget, original: &TerminalView) {
    let after = host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let before = original.runtime.as_ref().unwrap();
    let now = after.runtime.as_ref().unwrap();
    assert_eq!(
        (
            before.runtime_epoch,
            before.worker_pid,
            before.worker_start_time,
            before.child_pid,
            before.child_start_time
        ),
        (
            now.runtime_epoch,
            now.worker_pid,
            now.worker_start_time,
            now.child_pid,
            now.child_start_time
        )
    );
    assert_eq!(
        after.agent.as_ref().unwrap().process_pid,
        original.agent.as_ref().unwrap().process_pid
    );
}
