use super::{
    StartupPtyResources, WorkerCustodyLease, WorkerNamespace, WorkerSecondaryCustodySource,
    secondary_custody_capture, secondary_custody_handshake, secondary_custody_persistence,
    startup_checkpoint,
};
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use anyhow::Result;
use portable_pty::MasterPty;
use std::future::Future;
use std::sync::Arc;
use tokio::net::UnixListener;

pub(crate) fn establish<'a>(
    listener: &'a UnixListener,
    namespace: &'a WorkerNamespace,
    paths: &'a Paths,
    runtime: &'a RuntimeRecord,
    startup: &mut StartupPtyResources<Box<dyn MasterPty + Send>>,
    checkpoint: &mut startup_checkpoint::ActiveStartupCheckpoint,
) -> Result<
    impl Future<Output = Result<(Arc<WorkerSecondaryCustodySource>, WorkerCustodyLease)>> + Send + 'a,
> {
    let (durable, durable_identity) =
        secondary_custody_persistence::publish_pending(namespace, paths, runtime, checkpoint)?;
    let source = secondary_custody_capture::capture(startup, runtime)?;
    Ok(async move {
        let lease = secondary_custody_handshake::establish_initial(
            listener,
            namespace,
            paths,
            runtime,
            &durable,
            durable_identity,
            Arc::clone(&source),
        )
        .await?;
        Ok((source, lease))
    })
}
