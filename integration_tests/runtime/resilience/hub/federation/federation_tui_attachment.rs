use crate::{Harness, ProcessIdentity};
use yolo_hub_identity::HubId;
use yolo_runtime::TerminalView;

pub(super) async fn size(
    client: &yolo_client::Client,
    target: yolo_runtime::worker::RuntimeRef,
    grant: &yolo_runtime::model::WriterLeaseGrant,
) {
    client
        .resize(
            target,
            grant.generation,
            &grant.writer_capability,
            COLUMNS,
            ROWS,
        )
        .await
        .unwrap();
}

pub(super) async fn open(
    client: &yolo_client::Client,
    harness: &Harness,
    driver: &TerminalView,
    entry: HubId,
) -> yolo_client::Attachment {
    client
        .attach(
            driver.manifest.terminal_id,
            yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE,
        )
        .await
        .unwrap_or_else(|error| {
            let runtime = driver.runtime.as_ref().unwrap();
            let worker = ProcessIdentity {
                pid: runtime.worker_pid,
                start_time: runtime.worker_start_time,
            };
            panic!(
                "TUI driver attach failed before launch: entry={entry}, session={}, host_live={}, worker_live={}, error={error}, host_stderr={}",
                driver.manifest.terminal_id,
                harness.host_identity().matches(),
                worker.matches(),
                harness.host_stderr_tail()
            );
        })
}

const COLUMNS: u16 = 160;
const ROWS: u16 = 40;
