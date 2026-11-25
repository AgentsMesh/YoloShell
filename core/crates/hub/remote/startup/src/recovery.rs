use crate::{RemoteDirectories, constants::*, control::LiveControl, receipt, require_budget};
use anyhow::{Result, ensure};
use tokio::time::Instant;
use yolo_hub_enrollment_store::{EnrollmentRecord, Observation};
use yolo_hub_local_startup::ConnectionDocument;
use yolo_remote_provisioning_protocol::{
    EnrollmentOutcome, EnrollmentRecovery, RemoteBootstrapRequest,
};
use yolo_hub_state::HubState;
use yolo_paths::{directory::PrivateDirectory, private_file::io_kind};

/// Reads retained journal state without creating directories, locks, or services.
pub fn inspect(
    directories: &RemoteDirectories,
    request: &RemoteBootstrapRequest,
) -> Result<Option<EnrollmentRecord>> {
    Ok(observation(directories, request)?.map(|value| value.record().clone()))
}

/// Only reads an existing journal and existing running Hub. It never submits a mutation.
pub async fn recover(
    directories: &RemoteDirectories,
    request: &RemoteBootstrapRequest,
    deadline: Instant,
) -> Result<EnrollmentRecovery> {
    require_budget(deadline)?;
    let Some(observation) = observation(directories, request)? else {
        return Ok(EnrollmentRecovery::new(
            request,
            EnrollmentOutcome::Absent {},
        )?);
    };
    let connection = ConnectionDocument::local(
        observation.record().prepared.peer.hub(),
        HubState::socket_path_for(&directories.state)?,
    );
    recover_observed(&connection, observation, deadline).await
}

pub(crate) async fn recover_observed(
    connection: &ConnectionDocument,
    observation: Observation,
    deadline: Instant,
) -> Result<EnrollmentRecovery> {
    observation.verify()?;
    let prepared = &observation.record().prepared;
    ensure!(connection.entry_hub == prepared.peer.hub(), PIN_MISMATCH);
    let outcome = match observed_outcome(connection, prepared, deadline).await {
        Ok(outcome) => outcome,
        Err(_) => EnrollmentOutcome::Unknown {},
    };
    observation.verify()?;
    Ok(EnrollmentRecovery::new(&prepared.request, outcome)?)
}

async fn observed_outcome(
    connection: &ConnectionDocument,
    prepared: &yolo_hub_enrollment_store::PreparedEnrollment,
    deadline: Instant,
) -> Result<EnrollmentOutcome> {
    let mut control = LiveControl::open(connection, prepared.peer, deadline).await?;
    let (build, current) = control.observe(&prepared.request, deadline).await?;
    if prepared.applied_to(&current) {
        return Ok(EnrollmentOutcome::Ready {
            receipt: receipt::create(prepared, &current, build)?,
        });
    }
    Ok(
        if current.state == prepared.snapshot.state || !current.applied {
            EnrollmentOutcome::Unknown {}
        } else {
            EnrollmentOutcome::Superseded {}
        },
    )
}

fn observation(
    directories: &RemoteDirectories,
    request: &RemoteBootstrapRequest,
) -> Result<Option<Observation>> {
    let path = yolo_paths::platform::platform_path(&directories.state);
    let parent = match PrivateDirectory::open(&path) {
        Ok(parent) => parent,
        Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => return Ok(None),
        Err(error) => return Err(error),
    };
    match parent.open_descendant(
        &path
            .join(yolo_hub_enrollment_store::ENROLLMENT_DIRECTORY)
            .join(request.fields().request_id.to_string()),
    ) {
        Ok(_) => {}
        Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => {
            parent.verify_path()?;
            return Ok(None);
        }
        Err(error) => return Err(error),
    }
    let peer = HubState::inspect_peer_identity(&path, &directories.host_home)?
        .ok_or_else(|| anyhow::anyhow!(PIN_MISMATCH))?;
    ensure!(
        request
            .fields()
            .expected_peer
            .is_none_or(|expected| expected == peer),
        PIN_MISMATCH
    );
    let observed = yolo_hub_enrollment_store::inspect(&parent, peer, request)?;
    parent.verify_path()?;
    Ok(observed)
}
