use crate::{
    RemoteDirectories, configuration, constants::*, control::LiveControl, receipt, reconcile,
    recovery,
};
use anyhow::{Result, ensure};
use tokio::time::Instant;
use yolo_hub_enrollment_store::{Admission, PreparedEnrollment, StartupAdmission};
use yolo_hub_local_startup::ConnectionDocument;
use yolo_remote_provisioning_protocol::{
    EnrollmentOutcome, RemoteBootstrapReceipt, RemoteBootstrapRequest,
};
use yolo_hub_state::HubState;

#[cfg(test)]
pub(crate) async fn enroll(
    directories: &RemoteDirectories,
    connection: &ConnectionDocument,
    request: &RemoteBootstrapRequest,
    deadline: Instant,
) -> Result<RemoteBootstrapReceipt> {
    let admission = crate::admission::acquire(directories, request, deadline).await?;
    admitted(directories, connection, request, admission, deadline).await
}

pub(crate) async fn admitted(
    directories: &RemoteDirectories,
    connection: &ConnectionDocument,
    request: &RemoteBootstrapRequest,
    admission: StartupAdmission,
    deadline: Instant,
) -> Result<RemoteBootstrapReceipt> {
    crate::require_budget(deadline)?;
    admission.verify()?;
    let peer = HubState::inspect_peer_identity(&directories.state, &directories.host_home)?
        .ok_or_else(|| anyhow::anyhow!(PIN_MISMATCH))?;
    ensure!(
        peer.hub() == connection.entry_hub
            && request
                .fields()
                .expected_peer
                .is_none_or(|expected| expected == peer),
        PIN_MISMATCH
    );
    let claim = match admission.claim(peer)? {
        Admission::Fresh(claim) => claim,
        Admission::Existing(existing) => {
            return ready(recovery::recover_observed(connection, existing, deadline).await?);
        }
    };
    let mut control = LiveControl::open(connection, peer, deadline).await?;
    let (_, snapshot) = control.observe(request, deadline).await?;
    let change = configuration::plan(&snapshot, request)?;
    let permit = claim.prepare(PreparedEnrollment {
        request: request.clone(),
        peer,
        snapshot,
        change,
    })?;
    permit.verify()?;
    // Only the process-local permit returned by the first durable prepare reaches Configure.
    if let Some(change) = &permit.prepared().change {
        reconcile::apply(&mut control, change.clone(), deadline).await?;
    }
    permit.verify()?;
    let (build, applied) = control.observe(request, deadline).await?;
    let receipt = receipt::create(permit.prepared(), &applied, build)?;
    let completed = permit.complete(receipt)?;
    completed
        .completion
        .ok_or_else(|| anyhow::anyhow!(CONFIGURATION_UNKNOWN))
}

pub(crate) fn ready(
    recovered: yolo_remote_provisioning_protocol::EnrollmentRecovery,
) -> Result<RemoteBootstrapReceipt> {
    match recovered.fields().outcome.clone() {
        EnrollmentOutcome::Ready { receipt } => Ok(receipt),
        _ => anyhow::bail!(CONFIGURATION_UNKNOWN),
    }
}
