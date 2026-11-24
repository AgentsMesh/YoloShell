const RUNTIME_ERROR_CUSTODY_POLL_INTERVAL: Duration = Duration::from_millis(10);
const RUNTIME_ERROR_CUSTODY_TIMEOUT: Duration = crate::session_timing::WORKER_AUTH_TIMEOUT;
const RUNTIME_ERROR_RECORD_TIMEOUT: Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const RUNTIME_ERROR_CUSTODY_DESCRIPTOR_COUNT_DETAIL: &str =
    "runtime-error Host received an unexpected custody descriptor count";
const RUNTIME_ERROR_CUSTODY_RECORD_TIMEOUT_DETAIL: &str =
    "runtime-error worker did not publish pending custody in time";
const RUNTIME_ERROR_CUSTODY_PROTOCOL_FAILURE_DETAIL: &str = "runtime-error custody protocol failed";
const RUNTIME_ERROR_PRE_CLEANUP_PENDING_DETAIL: &str =
    "runtime-error worker has not exposed its pre-cleanup result";
const RUNTIME_ERROR_CUSTODY_SETTLEMENT_TIMEOUT_DETAIL: &str =
    "runtime-error Host custody did not settle before release";
const RUNTIME_ERROR_CUSTODY_SETTLEMENT_PROBE_DETAIL: &str =
    "inspect runtime-error Host custody settlement";
const RUNTIME_ERROR_CUSTODY_IMPORT_DETAIL: &str = "import runtime-error Host custody";
const RUNTIME_ERROR_WORKER_HANDLE_DETAIL: &str = "runtime-error worker handle is unavailable";
const RUNTIME_ERROR_WORKER_JOIN_DETAIL: &str = "runtime-error worker task failed to join";
const RUNTIME_ERROR_WORKER_TIMEOUT_DETAIL: &str =
    "runtime-error worker did not return before the test deadline";
const RUNTIME_ERROR_WORKER_EXPECTED_FAILURE_DETAIL: &str =
    "runtime-error worker unexpectedly succeeded";

struct RuntimeErrorWorker(Option<tokio::task::JoinHandle<Result<()>>>);

impl RuntimeErrorWorker {
    fn spawn(paths: Paths, terminal_id: Uuid) -> Self {
        Self(Some(tokio::spawn(run(paths, terminal_id))))
    }

    async fn finish_error(mut self) -> anyhow::Error {
        let joined = self
            .join()
            .await
            .unwrap_or_else(|error| panic!("{error:#}"));
        joined.expect_err(RUNTIME_ERROR_WORKER_EXPECTED_FAILURE_DETAIL)
    }

    async fn protocol_failure(&mut self, terminal_id: Uuid, protocol: anyhow::Error) -> ! {
        let worker = self.join().await;
        let runtime = super::runtime_result_test_hook::take(terminal_id)
            .unwrap_or_else(|| RUNTIME_ERROR_PRE_CLEANUP_PENDING_DETAIL.to_owned());
        panic!(
            "{RUNTIME_ERROR_CUSTODY_PROTOCOL_FAILURE_DETAIL}: {protocol:#}; \
             pre-cleanup: {runtime}; worker: {worker:?}"
        );
    }

    async fn join(&mut self) -> Result<Result<()>> {
        let worker = self
            .0
            .as_mut()
            .context(RUNTIME_ERROR_WORKER_HANDLE_DETAIL)?;
        let joined = match tokio::time::timeout(RUNTIME_ERROR_CUSTODY_TIMEOUT, &mut *worker).await {
            Ok(joined) => joined,
            Err(error) => {
                worker.abort();
                return Err(error).context(RUNTIME_ERROR_WORKER_TIMEOUT_DETAIL);
            }
        };
        self.0 = None;
        joined.context(RUNTIME_ERROR_WORKER_JOIN_DETAIL)
    }
}

impl Drop for RuntimeErrorWorker {
    fn drop(&mut self) {
        if let Some(worker) = self.0.as_ref()
            && !worker.is_finished()
        {
            worker.abort();
        }
    }
}

#[cfg(target_os = "macos")]
fn import_runtime_error_custody(
    record: &RuntimeRecord,
    offer: &crate::pty_custody::CustodyOffer,
    descriptors: Vec<std::os::fd::OwnedFd>,
) -> Result<crate::process::MacosSecondaryCustody> {
    let ordered: [std::os::fd::OwnedFd; crate::process::MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT] =
        descriptors
            .try_into()
            .map_err(|_| anyhow::anyhow!(RUNTIME_ERROR_CUSTODY_DESCRIPTOR_COUNT_DETAIL))?;
    crate::process::MacosSecondaryCustody::from_transferred(
        crate::process::MacosSecondaryCustodyDescriptorParts::from_ordered(ordered),
        &crate::pty_custody::CustodyRuntimeIdentity::from_record(record),
        offer.recovery_identity_sha256,
        offer.descriptor_set,
        offer.profile,
    )
    .context(RUNTIME_ERROR_CUSTODY_IMPORT_DETAIL)
}

#[cfg(not(target_os = "macos"))]
fn import_runtime_error_custody(
    _record: &RuntimeRecord,
    _offer: &crate::pty_custody::CustodyOffer,
    descriptors: Vec<std::os::fd::OwnedFd>,
) -> Result<Vec<std::os::fd::OwnedFd>> {
    Ok(descriptors)
}

#[cfg(target_os = "macos")]
async fn await_runtime_error_custody_settlement(
    record: &RuntimeRecord,
    custody: &crate::process::MacosSecondaryCustody,
) -> Result<()> {
    let child = crate::process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    };
    tokio::time::timeout(RUNTIME_ERROR_CUSTODY_TIMEOUT, async {
        loop {
            if !child.matches() {
                custody
                    .confirm_root_exit()
                    .context(RUNTIME_ERROR_CUSTODY_SETTLEMENT_PROBE_DETAIL)?;
                if custody
                    .closure_settled()
                    .context(RUNTIME_ERROR_CUSTODY_SETTLEMENT_PROBE_DETAIL)?
                {
                    return Ok(());
                }
            }
            tokio::time::sleep(RUNTIME_ERROR_CUSTODY_POLL_INTERVAL).await;
        }
    })
    .await
    .context(RUNTIME_ERROR_CUSTODY_SETTLEMENT_TIMEOUT_DETAIL)?
}

#[cfg(not(target_os = "macos"))]
async fn await_runtime_error_custody_settlement(
    record: &RuntimeRecord,
    _custody: &Vec<std::os::fd::OwnedFd>,
) -> Result<()> {
    let child = crate::process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    };
    tokio::time::timeout(RUNTIME_ERROR_CUSTODY_TIMEOUT, async {
        while child.matches() {
            tokio::time::sleep(RUNTIME_ERROR_CUSTODY_POLL_INTERVAL).await;
        }
    })
    .await
    .context(RUNTIME_ERROR_CUSTODY_SETTLEMENT_TIMEOUT_DETAIL)?;
    Ok(())
}

async fn wait_for_runtime_error_record(paths: &Paths, terminal_id: Uuid) -> Result<RuntimeRecord> {
    tokio::time::timeout(RUNTIME_ERROR_RECORD_TIMEOUT, async {
        loop {
            if let Ok(record) = read_json(&paths.runtime_record(terminal_id)) {
                return record;
            }
            tokio::time::sleep(RUNTIME_ERROR_CUSTODY_POLL_INTERVAL).await;
        }
    })
    .await
    .context(RUNTIME_ERROR_CUSTODY_RECORD_TIMEOUT_DETAIL)
}

include!("runtime_error_custody_protocol.rs");
