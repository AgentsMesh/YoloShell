use super::{Harness, deadline};
use std::path::Path;
use yolo_hub_identity::TerminalAddress;

pub const CREATED_NAME: &str = "Restored finalization shell";
pub const BASELINE_AND_CREATED: usize = 2;
const PENDING_FIELD: &str = "pending";
const UNFINALIZED_DETAIL: &str = "an unfinalized creation cannot be resumed";
const ALREADY_RUNNING_DETAIL: &str = "session is already running";

pub async fn assert_unfinalized(host: &Harness, address: TerminalAddress) {
    assert_resume_conflict(host, address, UNFINALIZED_DETAIL).await;
}

pub async fn assert_finalized_running(host: &Harness, address: TerminalAddress) {
    assert_resume_conflict(host, address, ALREADY_RUNNING_DETAIL).await;
}

async fn assert_resume_conflict(host: &Harness, address: TerminalAddress, expected: &str) {
    let error = host
        .client()
        .resume(address.terminal_id.as_uuid(), false)
        .await
        .unwrap_err();
    assert!(
        matches!(error, yolo_client::ClientError::Remote { code, message, .. }
        if code == yolo_runtime::protocol::error_code::CONFLICT && message == expected)
    );
}

pub async fn wait_acknowledged(directory: &Path) {
    let path = directory.join(yolo_tui_creation_journal::JOURNAL_FILE);
    tokio::time::timeout_at(deadline(), async {
        loop {
            if let Ok(document) = yolo_runtime::paths::read_json::<serde_json::Value>(&path)
                && document
                    .get(PENDING_FIELD)
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
            {
                return;
            }
            tokio::time::sleep(yolo_terminal_query_fixture::constants::POLL).await;
        }
    })
    .await
    .expect("the TUI must durably acknowledge only the restored finalization");
}
