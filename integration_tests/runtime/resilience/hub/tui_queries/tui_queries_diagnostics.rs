use super::*;
use std::io::Read;
use yolo_runtime::worker::RuntimeRef;

impl Source {
    pub(crate) async fn diagnostic(&self) -> String {
        let ready = std::fs::read(self.root.path().join(fixture::READY_FILE))
            .is_ok_and(|bytes| bytes == fixture::READY_VALUE);
        let report = (|| -> anyhow::Result<Report> {
            let file = std::fs::File::open(self.root.path().join(fixture::REPORT_FILE))?;
            Ok(serde_json::from_reader(
                file.take(fixture::MAX_COMMAND_BYTES),
            )?)
        })();
        let runtime = RuntimeRef::new(
            self.target.address.terminal_id.as_uuid(),
            self.target.runtime_epoch.as_uuid(),
        );
        let observation = tokio::time::timeout(DIAGNOSTIC_TIMEOUT, async {
            let (view, lease, snapshot) = tokio::join!(
                self.client.get_session(runtime.terminal_id),
                self.client.get_writer_lease(runtime),
                self.client.snapshot(runtime.terminal_id),
            );
            let runtime = view.map(|view| view.runtime);
            let terminal = snapshot.map(|snapshot| {
                let bytes = &snapshot.data[..snapshot.data.len().min(DIAGNOSTIC_SNAPSHOT_BYTES)];
                (
                    snapshot.rows,
                    snapshot.cols,
                    String::from_utf8_lossy(bytes).into_owned(),
                )
            });
            (runtime, lease, terminal)
        })
        .await;
        format!(
            "source={} sequence={} ready={ready} last_report={report:?} host={observation:?}",
            self.name, self.sequence
        )
    }
}
