use super::{
    PtyInputWriter, StartupPtyResources, WorkerCustodyLease, WorkerSecondaryCustodySource,
};
use crate::worker::bootstrap_gate::ExplicitBootstrapExecFailure;
use anyhow::{Context, Result};
use portable_pty::MasterPty;
use std::io::Read;
use std::sync::Arc;

pub(crate) const PRE_ACTIVATION_RELEASE_TIMEOUT_CONTEXT: &str =
    "pre-activation Host PTY custody release timed out";
pub(crate) const FAILURE_RECORD_CONTEXT: &str = "record explicit bootstrap exec failure";
pub(crate) const OWNER_MASTER_RELEASE_CONTEXT: &str = "release startup PTY master owner";
pub(crate) const LOCAL_ALIAS_RELEASE_CONTEXT: &str = "close local startup PTY aliases";
pub(crate) const HOST_RELEASE_PROOF_CONTEXT: &str = "prove pre-activation Host PTY custody release";

pub(crate) async fn rollback_custody(
    failure: ExplicitBootstrapExecFailure,
    startup: &mut StartupPtyResources<Box<dyn MasterPty + Send>>,
    reader: Box<dyn Read + Send>,
    writer: PtyInputWriter,
    custody_source: Arc<WorkerSecondaryCustodySource>,
    custody_lease: WorkerCustodyLease,
) -> Result<()> {
    startup
        .child_mut()
        .record_explicit_bootstrap_exec_failure(failure)
        .context(FAILURE_RECORD_CONTEXT)?;
    drop(reader);
    drop(writer);
    drop(custody_source);
    startup
        .close_owner_master()
        .context(OWNER_MASTER_RELEASE_CONTEXT)?;
    startup
        .child_mut()
        .close_local_aliases_for_host_release()
        .context(LOCAL_ALIAS_RELEASE_CONTEXT)?;
    let exact = tokio::time::timeout(
        crate::session_timing::WORKER_READY_TIMEOUT,
        custody_lease.rollback_before_activation(),
    )
    .await
    .context(PRE_ACTIVATION_RELEASE_TIMEOUT_CONTEXT)??;
    startup
        .child_mut()
        .prove_host_release(exact)
        .context(HOST_RELEASE_PROOF_CONTEXT)
}
