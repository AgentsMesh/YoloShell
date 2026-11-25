use super::recovery_protocol::{
    GUARDIAN_RECOVERY_METHOD, GuardianRecoveryPhase, GuardianRecoveryReceipt,
    GuardianRecoveryRequest, RECOVERY_DESCRIPTOR_COUNT,
};
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::process::{PtyCustodyRecoveryAuthority, PtyMasterFdCustodyProbe};
use crate::pty_custody::{CustodyDurableRecord, CustodyHolderIdentity, CustodyUnixStream};
use anyhow::{Context, Result, ensure};
use std::future::Future;
use std::os::fd::OwnedFd;
use std::time::Duration;
use tokio::time::{Instant, timeout_at};

const CONNECT_CONTEXT: &str = "connect to PTY custody Guardian recovery channel";
const RESPONSE_DETAIL: &str = "PTY custody Guardian recovery response is not exact";
const RECORD_DETAIL: &str = "PTY custody Guardian recovery authority changed";
const RECOVERY_NAMESPACE_LABEL: &str = "PTY custody Guardian recovery session";
const RECOVERY_CLIENT_TIMEOUT: Duration = crate::session_timing::SESSION_STARTUP_CLIENT_TIMEOUT;
const RECOVERY_TIMEOUT_DETAIL: &str = "PTY custody Guardian recovery exceeded its deadline";

pub async fn recover_authority(
    paths: &Paths,
    runtime_file_identity: FileIdentity,
    durable: &CustodyDurableRecord,
    durable_file_identity: FileIdentity,
    probe: PtyMasterFdCustodyProbe,
) -> Result<PtyCustodyRecoveryAuthority> {
    let deadline = Instant::now() + RECOVERY_CLIENT_TIMEOUT;
    within_deadline(
        deadline,
        recover_authority_inner(
            paths,
            runtime_file_identity,
            durable,
            durable_file_identity,
            probe,
        ),
    )
    .await
}

async fn within_deadline<T>(
    deadline: Instant,
    operation: impl Future<Output = Result<T>>,
) -> Result<T> {
    timeout_at(deadline, operation)
        .await
        .context(RECOVERY_TIMEOUT_DETAIL)?
}

async fn recover_authority_inner(
    paths: &Paths,
    runtime_file_identity: FileIdentity,
    durable: &CustodyDurableRecord,
    durable_file_identity: FileIdentity,
    probe: PtyMasterFdCustodyProbe,
) -> Result<PtyCustodyRecoveryAuthority> {
    confirm_records(paths, runtime_file_identity, durable, durable_file_identity)?;
    let lease = durable.armed_lease().context(RECORD_DETAIL)?;
    let guardian = lease.guardian.context(RECORD_DETAIL)?;
    ensure!(guardian.process().matches(), RECORD_DETAIL);
    probe.confirm_exact()?;
    let requester = CustodyHolderIdentity::capture()?;
    let request = GuardianRecoveryRequest::new(
        requester,
        guardian,
        runtime_file_identity,
        durable.clone(),
        durable_file_identity,
    );
    let path = super::socket::path_for(paths, guardian.guardian_id())?;
    super::socket::verify_named(&path)?;
    let mut stream = CustodyUnixStream::connect(&path)
        .await
        .context(CONNECT_CONTEXT)?;
    super::socket::verify_peer(&stream, guardian.process())?;
    let descriptor = exchange(&mut stream, &request).await?;
    super::socket::verify_peer(&stream, guardian.process())?;
    let runtime = confirm_records(paths, runtime_file_identity, durable, durable_file_identity)?;
    let authority = PtyCustodyRecoveryAuthority::from_transferred(
        probe,
        descriptor,
        &runtime,
        lease.descriptor_set,
        lease.recovery_identity_sha256,
    )?;
    stream
        .write(&GuardianRecoveryReceipt::new(
            request.transfer_id,
            GuardianRecoveryPhase::Acknowledged,
        ))
        .await?;
    let completed: GuardianRecoveryReceipt = stream.read().await?;
    ensure!(
        completed.validates(&request, GuardianRecoveryPhase::Completed),
        RESPONSE_DETAIL
    );
    Ok(authority)
}

async fn exchange(
    stream: &mut CustodyUnixStream,
    request: &GuardianRecoveryRequest,
) -> Result<OwnedFd> {
    stream
        .write_method_opening(GUARDIAN_RECOVERY_METHOD, request)
        .await?;
    let prepared: GuardianRecoveryReceipt = stream.read().await?;
    ensure!(
        prepared.validates(request, GuardianRecoveryPhase::Prepared),
        RESPONSE_DETAIL
    );
    let descriptor = receive_reader(stream).await?;
    let committed: GuardianRecoveryReceipt = stream.read().await?;
    ensure!(
        committed.validates(request, GuardianRecoveryPhase::Committed),
        RESPONSE_DETAIL
    );
    Ok(descriptor)
}

async fn receive_reader(stream: &CustodyUnixStream) -> Result<OwnedFd> {
    let descriptors = stream.receive_rights(RECOVERY_DESCRIPTOR_COUNT).await?;
    let [descriptor]: [OwnedFd; RECOVERY_DESCRIPTOR_COUNT] = descriptors
        .try_into()
        .map_err(|_| anyhow::anyhow!(RESPONSE_DETAIL))?;
    Ok(descriptor)
}

fn confirm_records(
    paths: &Paths,
    runtime_file_identity: FileIdentity,
    durable: &CustodyDurableRecord,
    durable_file_identity: FileIdentity,
) -> Result<crate::model::RuntimeRecord> {
    let session_dir = paths.session_dir(durable.runtime.terminal_id);
    let namespace = crate::namespace_lease::NamespaceLease::capture(&[(
        RECOVERY_NAMESPACE_LABEL,
        &session_dir,
    )])?;
    let runtime: (crate::model::RuntimeRecord, FileIdentity) =
        namespace.read_json_with_identity(&paths.runtime_record(durable.runtime.terminal_id))?;
    ensure!(runtime.1 == runtime_file_identity, RECORD_DETAIL);
    ensure!(durable.runtime.matches_record(&runtime.0), RECORD_DETAIL);
    let current: (CustodyDurableRecord, FileIdentity) =
        namespace.read_json_with_identity(&paths.pty_custody(durable.runtime.terminal_id))?;
    ensure!(
        current == (durable.clone(), durable_file_identity),
        RECORD_DETAIL
    );
    Ok(runtime.0)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/recovery_client_deadline_tests.rs"]
mod deadline_tests;
