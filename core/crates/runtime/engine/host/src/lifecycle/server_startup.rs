pub(crate) async fn stop_ingress_after_startup_failure(
    ingress: &mut HostIngress,
    service: Result<()>,
    maintenance: Result<()>,
) -> Result<()> {
    let ingress_outcome = ingress.shutdown().await;
    host_shutdown_result(service, ingress_outcome, maintenance)
}

pub(crate) async fn stop_after_maintenance_start_failure(
    ingress: &mut HostIngress,
    host: Arc<Host>,
    serving: &mut ServingHost,
    start_error: anyhow::Error,
) -> Result<()> {
    let mut maintenance = serving.take_maintenance();
    if let Some(worker) = maintenance.as_ref() {
        worker.request_shutdown();
    }
    let ingress_outcome = ingress.shutdown().await;
    drop(host);
    serving.release();
    let cleanup_outcome = match maintenance.as_mut() {
        Some(worker) => worker.shutdown().await,
        None => Ok(()),
    };
    let maintenance_outcome = combine_maintenance_start_errors(start_error, cleanup_outcome);
    host_shutdown_result(Ok(()), ingress_outcome, maintenance_outcome)
}

pub(crate) fn combine_maintenance_start_errors(
    start: anyhow::Error,
    cleanup: Result<()>,
) -> Result<()> {
    match cleanup {
        Ok(()) => Err(start),
        Err(cleanup) => Err(HostShutdownErrors {
            failures: vec![
                HostShutdownFailure {
                    stage: HostShutdownStage::Maintenance,
                    error: start,
                },
                HostShutdownFailure {
                    stage: HostShutdownStage::Maintenance,
                    error: cleanup,
                },
            ],
        }
        .into()),
    }
}
