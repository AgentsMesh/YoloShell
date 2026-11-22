use super::recovery_protocol::{
    GuardianRecoveryPhase, GuardianRecoveryReceipt, GuardianRecoveryRequest,
};
use super::settlement::GuardianState;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::process::MacosSecondaryCustody;
use crate::protocol::RequestEnvelope;
use crate::pty_custody::{CustodyDurableLease, CustodyUnixStream};
use anyhow::{Context, Result, ensure};
use std::os::fd::AsFd;

const PROTOCOL_DETAIL: &str = "PTY custody Guardian recovery protocol is not current";
const REQUESTER_DETAIL: &str = "PTY custody Guardian recovery requester is not exact";
const GUARDIAN_BINDING_DETAIL: &str = "PTY custody Guardian recovery binding is not exact";
const GUARDIAN_PROCESS_DETAIL: &str = "PTY custody Guardian recovery process is not live";
const DURABLE_RECORD_DETAIL: &str = "PTY custody Guardian recovery durable record is invalid";
const DURABLE_RUNTIME_DETAIL: &str = "PTY custody Guardian recovery runtime is not exact";
const DURABLE_LEASE_DETAIL: &str = "PTY custody Guardian recovery lease is not armed";
const RECOVERY_DIGEST_DETAIL: &str = "PTY custody Guardian recovery digest is not exact";
const DESCRIPTOR_SET_DETAIL: &str = "PTY custody Guardian recovery descriptors are not exact";
const RUNTIME_IDENTITY_DETAIL: &str = "PTY custody Guardian recovery runtime identity changed";
const RUNTIME_RECORD_DETAIL: &str = "PTY custody Guardian recovery runtime record changed";
const DURABLE_IDENTITY_DETAIL: &str = "PTY custody Guardian recovery durable identity changed";
const PTY_MASTER_DETAIL: &str = "PTY custody Guardian recovery master descriptor changed";
const ACKNOWLEDGEMENT_DETAIL: &str = "PTY custody Guardian recovery acknowledgement is not exact";

pub(super) async fn handle(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    state: &GuardianState,
    mut stream: CustodyUnixStream,
    opening: RequestEnvelope,
) -> Result<()> {
    let request = GuardianRecoveryRequest::decode(&opening)?;
    validate(namespace, paths, custody, state, &stream, &request)?;
    let descriptor = custody.duplicate_recovery_reader()?;
    validate(namespace, paths, custody, state, &stream, &request)?;
    stream
        .write(&GuardianRecoveryReceipt::new(
            request.transfer_id,
            GuardianRecoveryPhase::Prepared,
        ))
        .await?;
    stream.send_rights(&[descriptor.as_fd()]).await?;
    validate(namespace, paths, custody, state, &stream, &request)?;
    stream
        .write(&GuardianRecoveryReceipt::new(
            request.transfer_id,
            GuardianRecoveryPhase::Committed,
        ))
        .await?;
    let acknowledged: GuardianRecoveryReceipt = stream.read().await?;
    ensure!(
        acknowledged.validates(&request, GuardianRecoveryPhase::Acknowledged),
        ACKNOWLEDGEMENT_DETAIL
    );
    validate(namespace, paths, custody, state, &stream, &request)?;
    stream
        .write(&GuardianRecoveryReceipt::new(
            request.transfer_id,
            GuardianRecoveryPhase::Completed,
        ))
        .await
}

fn validate(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    state: &GuardianState,
    stream: &CustodyUnixStream,
    request: &GuardianRecoveryRequest,
) -> Result<()> {
    ensure!(request.protocol_is_current(), PROTOCOL_DETAIL);
    super::socket::verify_peer(stream, request.requester).context(REQUESTER_DETAIL)?;
    ensure!(request.guardian == state.binding(), GUARDIAN_BINDING_DETAIL);
    ensure!(
        request.guardian.process().matches(),
        GUARDIAN_PROCESS_DETAIL
    );
    ensure!(request.durable.is_valid(), DURABLE_RECORD_DETAIL);
    ensure!(
        request.durable.runtime == *state.runtime(),
        DURABLE_RUNTIME_DETAIL
    );
    let lease = request
        .durable
        .armed_lease()
        .context(DURABLE_LEASE_DETAIL)?;
    validate_lease(custody, state, lease)?;
    let (runtime, runtime_identity) = state.retained_runtime()?;
    ensure!(
        runtime_identity == request.runtime_file_identity,
        RUNTIME_IDENTITY_DETAIL
    );
    ensure!(
        request.durable.runtime.matches_record(runtime),
        RUNTIME_RECORD_DETAIL
    );
    let current_runtime = namespace
        .read_json_with_identity(&paths.runtime_record(request.durable.runtime.terminal_id))?;
    ensure!(
        current_runtime == (runtime.clone(), runtime_identity),
        RUNTIME_RECORD_DETAIL
    );
    let current_durable = namespace
        .read_json_with_identity(&paths.pty_custody(request.durable.runtime.terminal_id))?;
    ensure!(
        current_durable == (request.durable.clone(), request.durable_file_identity),
        DURABLE_IDENTITY_DETAIL
    );
    custody
        .master_fd_custody_probe()
        .and_then(|probe| probe.confirm_exact())
        .context(PTY_MASTER_DETAIL)
}

fn validate_lease(
    custody: &MacosSecondaryCustody,
    state: &GuardianState,
    lease: CustodyDurableLease,
) -> Result<()> {
    ensure!(
        lease.guardian == Some(state.binding()),
        GUARDIAN_BINDING_DETAIL
    );
    ensure!(
        lease.recovery_identity_sha256 == custody.recovery_identity_sha256(),
        RECOVERY_DIGEST_DETAIL
    );
    ensure!(
        custody.matches_process_descriptor_set(lease.descriptor_set),
        DESCRIPTOR_SET_DETAIL
    );
    Ok(())
}
