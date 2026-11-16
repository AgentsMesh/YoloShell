use super::*;

const HANDSHAKE_TIMEOUT_CONTEXT: &str = "PTY secondary-custody handshake timed out";

pub(crate) async fn establish(
    listener: &UnixListener,
    namespace: &WorkerNamespace,
    paths: &Paths,
    runtime: &RuntimeRecord,
    durable: &CustodyDurableRecord,
    durable_identity: FileIdentity,
    source: Arc<WorkerSecondaryCustodySource>,
) -> Result<WorkerCustodyLease> {
    let lease = tokio::time::timeout(
        crate::session_timing::WORKER_READY_TIMEOUT,
        super::accept_valid(
            listener,
            namespace,
            paths,
            runtime,
            durable,
            durable_identity,
            Arc::clone(&source),
        ),
    )
    .await
    .context(HANDSHAKE_TIMEOUT_CONTEXT)??;
    source.complete_startup_transfer();
    Ok(lease)
}
