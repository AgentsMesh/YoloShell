use super::{POLL_INTERVAL, TerminalSize};
use anyhow::{Result, ensure};
use tokio::time::{Instant, sleep_until};
use yolo_process_test_support::{ProcessIdentityObservation, observe_process_identity};
use yolo_runtime::paths::{
    Paths, ReadVersionChanged, private_file::PrivateFileEntry, read_entry_guarded,
};
use yolo_runtime::{RuntimeRecord, TerminalView};

pub(super) const DIMENSIONS_TIMEOUT: &str =
    "relay dimensions did not converge before the original deadline";
const INITIAL_RUNTIME_MISSING: &str = "relay fixture has no authenticated runtime";
const RUNTIME_REPLACED: &str = "relay fixture runtime identity changed";
const PROCESS_CHANGED: &str = "relay fixture worker or child identity is no longer live";

pub(super) struct RuntimeDimensions {
    entry: PrivateFileEntry,
    expected: RuntimeRecord,
}

impl RuntimeDimensions {
    pub fn capture(paths: &Paths, session: &TerminalView) -> Result<Self> {
        let entry = PrivateFileEntry::open(&paths.runtime_record(session.manifest.terminal_id))?;
        let expected: RuntimeRecord = read_entry_guarded(entry.reopen()?, || Ok(()))?;
        let runtime = session
            .runtime
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(INITIAL_RUNTIME_MISSING))?;
        ensure!(
            expected.terminal_id == session.manifest.terminal_id
                && expected.runtime_epoch == runtime.runtime_epoch
                && expected.worker_id == runtime.worker_id
                && expected.worker_pid == runtime.worker_pid
                && expected.worker_start_time == runtime.worker_start_time
                && expected.child_pid == runtime.child_pid
                && expected.child_start_time == runtime.child_start_time,
            RUNTIME_REPLACED
        );
        require_live(&expected)?;
        Ok(Self { entry, expected })
    }

    pub async fn wait(&mut self, size: TerminalSize, deadline: Instant) -> Result<()> {
        self.wait_with(size, deadline, |entry| {
            read_entry_guarded(entry.reopen()?, || Ok(()))
        })
        .await
    }

    async fn wait_with(
        &mut self,
        size: TerminalSize,
        deadline: Instant,
        mut read: impl FnMut(&PrivateFileEntry) -> Result<RuntimeRecord>,
    ) -> Result<()> {
        loop {
            ensure!(Instant::now() < deadline, DIMENSIONS_TIMEOUT);
            require_live(&self.expected)?;
            ensure!(Instant::now() < deadline, DIMENSIONS_TIMEOUT);
            let observed = read(&self.entry);
            ensure!(Instant::now() < deadline, DIMENSIONS_TIMEOUT);
            require_live(&self.expected)?;
            match observed {
                Ok(record) => {
                    record.validate_update_from(&self.expected)?;
                    let complete = (record.cols, record.rows) == (size.cols, size.rows);
                    self.expected = record;
                    if complete {
                        ensure!(Instant::now() < deadline, DIMENSIONS_TIMEOUT);
                        return Ok(());
                    }
                }
                Err(error) if error.downcast_ref::<ReadVersionChanged>().is_some() => {}
                Err(error) => return Err(error),
            }
            sleep_until(std::cmp::min(Instant::now() + POLL_INTERVAL, deadline)).await;
        }
    }
}

fn require_live(record: &RuntimeRecord) -> Result<()> {
    for (pid, start) in [
        (record.worker_pid, record.worker_start_time),
        (record.child_pid, record.child_start_time),
    ] {
        ensure!(
            observe_process_identity(pid, start) == ProcessIdentityObservation::Matching,
            PROCESS_CHANGED
        );
    }
    Ok(())
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "relay/unit_tests/dimensions.rs"]
mod tests;
