const REQUEST_TIMEOUT_DETAIL: &str = "detached guardian successor request timed out";
const SOCKET_UNAVAILABLE_DETAIL: &str = "PTY custody guardian socket is unavailable";
const SUCCESSOR_REJECTION_DETAIL: &str = "PTY custody guardian rejected successor";

pub(super) async fn run(
    namespace: NamespaceLease,
    paths: Paths,
    socket: super::socket::GuardianSocket,
    custody: MacosSecondaryCustody,
    preparation: CustodyGuardianPreparation,
    prepared: CustodyDurableRecord,
    predecessor: CustodyDurableSnapshot,
    runtime: crate::model::RuntimeRecord,
    runtime_identity: FileIdentity,
    retained_runtime_file: super::settlement_runtime_file::RetainedRuntimeFile,
) -> Result<()> {
    let mut state = GuardianState::new(
        preparation,
        prepared,
        predecessor,
        runtime,
        runtime_identity,
        retained_runtime_file,
    )?;
    let mut failure_log = super::settlement_failure::ProgressFailureLog::default();
    let mut idle = idle_observation::IdleObservation::default();
    loop {
        match idle.progress(&namespace, &paths, &custody, &mut state) {
            Ok(Progress::Retired) => return Ok(()),
            Ok(Progress::Pending) => {}
            Err(error) => {
                match super::settlement_failure::resolve(
                    &namespace,
                    state.runtime(),
                    error,
                    &mut failure_log,
                ) {
                    super::settlement_failure::Resolution::Retry => {}
                    super::settlement_failure::Resolution::RetainUntilSettled(error) => {
                        return super::settlement_failure::retain_until_processes_settle(
                            &custody,
                            state.runtime(),
                            error,
                        )
                        .await;
                    }
                }
            }
        }
        tokio::select! {
            accepted = socket.accept() => match accepted {
                Ok(stream) => {
                    idle.invalidate();
                    let request = super::request::handle(
                        &namespace, &paths, &custody, &mut state, stream,
                    );
                    let handled = tokio::time::timeout(
                        crate::session_timing::WORKER_READY_TIMEOUT,
                        request,
                    ).await;
                    if let Err(error) = handled.unwrap_or_else(|_| {
                        Err(anyhow::anyhow!(REQUEST_TIMEOUT_DETAIL))
                    }) {
                        super::terminal_reporting::warn_owned(error, SUCCESSOR_REJECTION_DETAIL);
                    }
                }
                Err(error) => {
                    super::terminal_reporting::error_owned(error, SOCKET_UNAVAILABLE_DETAIL)
                }
            },
            _ = tokio::time::sleep(POLL_INTERVAL) => {}
        }
    }
}
