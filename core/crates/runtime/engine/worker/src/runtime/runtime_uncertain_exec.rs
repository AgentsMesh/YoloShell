pub(crate) const QUARANTINE_LOG: &str = "retaining startup custody after uncertain bootstrap exec";

pub(crate) struct UncertainBootstrapExec {
    detail: Arc<str>,
}

pub(crate) struct QuarantinedStartup {
    _uncertainty: UncertainBootstrapExec,
    _startup: StartupPtyResources<Box<dyn MasterPty + Send>>,
    _reader: Box<dyn Read + Send>,
    _writer: PtyInputWriter,
    _custody_source: Arc<WorkerSecondaryCustodySource>,
}

pub(crate) type QuarantineResources = (
    StartupPtyResources<Box<dyn MasterPty + Send>>,
    Box<dyn Read + Send>,
    PtyInputWriter,
    Arc<WorkerSecondaryCustodySource>,
    WorkerCustodyLease,
);

pub(crate) async fn quarantine_uncertain_exec(
    terminal_id: Uuid,
    error: anyhow::Error,
    resources: QuarantineResources,
) -> Result<()> {
    #[cfg(not(test))]
    let _ = terminal_id;
    let (startup, reader, writer, custody_source, custody_lease) = resources;
    let uncertainty = UncertainBootstrapExec {
        detail: format!("{error:#}").into(),
    };
    #[cfg(test)]
    runtime_result_test_hook::record_pending(terminal_id, &uncertainty.detail);
    tracing::error!(detail = %uncertainty.detail, QUARANTINE_LOG);

    // EOF tells the Host that the Worker made no activation decision. The retained
    // startup resources remain the primary custodian until a recovery controller
    // deliberately replaces or terminates this Worker.
    drop(custody_lease);
    let _quarantine = QuarantinedStartup {
        _uncertainty: uncertainty,
        _startup: startup,
        _reader: reader,
        _writer: writer,
        _custody_source: custody_source,
    };
    std::future::pending().await
}
