use super::Driver;
use crate::hub::tui_queries::constants::*;

const REDACTED_CAPABILITY: &str = "[writer capability redacted]";
const REDACTION_FAILURE: &str = "driver diagnostic could not safely redact its capability";

impl Driver {
    /// Fresh read-only observations use independent RPC connections. Do not
    /// consume/cancel the driver's attachment or renew its writer during diagnosis.
    pub async fn diagnostic(&self) -> String {
        let observed = tokio::time::timeout(DIAGNOSTIC_TIMEOUT, async {
            let (view, snapshot) = tokio::join!(
                self.client.get_session(self.target.terminal_id),
                self.client.snapshot(self.target.terminal_id),
            );
            let runtime = view
                .map(|view| {
                    view.runtime.map(|runtime| {
                        format!(
                            "epoch={} status={:?} worker=({}, {}) driver_shell=({}, {}) driver_shell_alive={:?}",
                            runtime.runtime_epoch, runtime.status,
                            runtime.worker_pid, runtime.worker_start_time,
                            runtime.child_pid, runtime.child_start_time,
                            yolo_runtime::process::process_start_time(runtime.child_pid)
                                .map(|start| start == runtime.child_start_time),
                        )
                    })
                })
                .map_err(|error| error.to_string());
            let fresh_screen = snapshot
                .map(|snapshot| {
                    let mut screen = vt100::Parser::new(snapshot.rows, snapshot.cols, SCROLLBACK);
                    screen.process(&snapshot.data);
                    let text = screen.screen().contents();
                    format!("barrier={} tui_returned={} screen={text:?}", snapshot.barrier_sequence, text.contains(&self.complete))
                })
                .map_err(|error| error.to_string());
            (runtime, fresh_screen)
        })
        .await;
        let observation = format!(
            "driver_target={:?} last_consumed_screen={:?} fresh_read_only={observed:?} output={}",
            self.target,
            self.screen.screen().contents(),
            self.output.evidence(),
        );
        match uuid::Uuid::from_slice(self.grant.writer_capability.as_bytes()) {
            Ok(capability) => {
                observation.replace(capability.to_string().as_str(), REDACTED_CAPABILITY)
            }
            Err(_) => REDACTION_FAILURE.into(),
        }
    }
}
