use super::*;
use crate::pty_custody::{CustodyControl, CustodyControlAck};

const RELEASE_MISMATCH_DETAIL: &str =
    "pre-activation PTY custody Release does not match its exact offer";
const RELEASE_STATE_DETAIL: &str =
    "pre-activation PTY custody Release arrived in the wrong lease state";
const SETTLEMENT_CONTEXT: &str = "prove pre-activation PTY custody process settlement";
const SETTLEMENT_TIMEOUT_DETAIL: &str = "pre-activation PTY custody process settlement timed out";
const ACK_WRITE_CONTEXT: &str = "write pre-activation PTY custody Released acknowledgement";
const STREAM_READ_CONTEXT: &str = "read pre-activation PTY custody rollback stream";
const UNEXPECTED_CONTROL_DETAIL: &str =
    "pre-activation PTY custody rollback received an unexpected second control";
#[cfg(all(target_os = "macos", any(test, feature = "custody-release-test-hooks")))]
const RELEASE_PROBE_CONTEXT: &str = "capture pre-activation PTY custody release probe";

pub(super) async fn before_activation(
    lease: &Arc<SecondaryCustodyLease>,
    stream: &mut CustodyUnixStream,
    control: CustodyControl,
) -> Result<(), WorkerRpcError> {
    if !control.matches_release(&lease.offer) {
        return Err(WorkerRpcError::identity_mismatch(RELEASE_MISMATCH_DETAIL));
    }
    let acknowledgement = CustodyControlAck::released(&control)
        .ok_or_else(|| WorkerRpcError::protocol(RELEASE_MISMATCH_DETAIL))?;
    begin(lease)?;
    let settlement = wait_for_settlement_or_stream_failure(lease, stream).await?;
    let probe = release_probe(lease)?;
    pause(lease, BeforeOrAfterDrop::Before, probe.clone()).await?;
    let pending = lease.commit_rollback_release(settlement)?;
    #[cfg(test)]
    test_hook::pause_after_platform_drop(lease.runtime.terminal_id).await;
    if let Err(error) = pause(lease, BeforeOrAfterDrop::After, probe).await {
        pending.fail(super::containment::display_detail(&error));
        return Err(error);
    }
    if let Err(error) = stream.write(&acknowledgement).await {
        let failure = transport(ACK_WRITE_CONTEXT, error);
        pending.fail(super::containment::display_detail(&failure));
        return Err(failure);
    }
    pending.acknowledge();
    Ok(())
}

async fn wait_for_settlement_or_stream_failure<'lease>(
    lease: &'lease SecondaryCustodyLease,
    stream: &mut CustodyUnixStream,
) -> Result<super::settlement::ExactSecondaryCustodySettlement<'lease>, WorkerRpcError> {
    let settlement = wait_for_exact_settlement(lease);
    tokio::pin!(settlement);
    tokio::select! {
        biased;
        inbound = stream.read::<CustodyControl>() => match inbound {
            Ok(_) => Err(WorkerRpcError::protocol(UNEXPECTED_CONTROL_DETAIL)),
            Err(error) => Err(transport(STREAM_READ_CONTEXT, error)),
        },
        result = &mut settlement => result,
    }
}

fn begin(lease: &SecondaryCustodyLease) -> Result<(), WorkerRpcError> {
    let mut state = super::super::lock_recover(&lease.state);
    if *state != SecondaryCustodyState::Preparing {
        return Err(WorkerRpcError::protocol(RELEASE_STATE_DETAIL));
    }
    *state = SecondaryCustodyState::RollingBack;
    lease.publish_state(SecondaryCustodyState::RollingBack);
    Ok(())
}

async fn wait_for_exact_settlement(
    lease: &SecondaryCustodyLease,
) -> Result<super::settlement::ExactSecondaryCustodySettlement<'_>, WorkerRpcError> {
    tokio::time::timeout(
        crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT,
        super::settlement::wait_for_exact(lease),
    )
    .await
    .map_err(|_| WorkerRpcError::transport(SETTLEMENT_TIMEOUT_DETAIL))?
    .map_err(|error| {
        let detail = super::containment::error_detail(error);
        WorkerRpcError::transport(format!("{SETTLEMENT_CONTEXT}: {detail}"))
    })
}

#[derive(Clone, Copy)]
enum BeforeOrAfterDrop {
    Before,
    After,
}

#[cfg(all(target_os = "macos", any(test, feature = "custody-release-test-hooks")))]
type ReleaseProbe = Option<crate::process::PtyMasterFdCustodyProbe>;
#[cfg(not(all(target_os = "macos", any(test, feature = "custody-release-test-hooks"))))]
type ReleaseProbe = ();

#[cfg(all(target_os = "macos", any(test, feature = "custody-release-test-hooks")))]
fn release_probe(lease: &SecondaryCustodyLease) -> Result<ReleaseProbe, WorkerRpcError> {
    lease
        .master_fd_custody_probe()
        .map_err(|error| transport(RELEASE_PROBE_CONTEXT, error))
}

#[cfg(not(all(target_os = "macos", any(test, feature = "custody-release-test-hooks"))))]
fn release_probe(_lease: &SecondaryCustodyLease) -> Result<ReleaseProbe, WorkerRpcError> {
    Ok(())
}

#[cfg(all(target_os = "macos", any(test, feature = "custody-release-test-hooks")))]
async fn pause(
    lease: &SecondaryCustodyLease,
    boundary: BeforeOrAfterDrop,
    probe: ReleaseProbe,
) -> Result<(), WorkerRpcError> {
    use crate::host::secondary_custody_release_test_hook::CustodyReleaseBoundary;
    let boundary = match boundary {
        BeforeOrAfterDrop::Before => CustodyReleaseBoundary::BeforePlatformDrop,
        BeforeOrAfterDrop::After => CustodyReleaseBoundary::AfterPlatformDrop,
    };
    crate::host::secondary_custody_release_test_hook::pause(
        lease.runtime.terminal_id,
        boundary,
        probe,
    )
    .await
    .map_err(|error| WorkerRpcError::transport(super::containment::error_detail(error)))
}

#[cfg(not(all(target_os = "macos", any(test, feature = "custody-release-test-hooks"))))]
async fn pause(
    _lease: &SecondaryCustodyLease,
    _boundary: BeforeOrAfterDrop,
    _probe: ReleaseProbe,
) -> Result<(), WorkerRpcError> {
    Ok(())
}

fn transport(context: &'static str, error: anyhow::Error) -> WorkerRpcError {
    let detail = super::containment::error_detail(error);
    WorkerRpcError::transport(format!("{context}: {detail}"))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/unit/secondary_custody/secondary_custody_rollback_test_hook.rs"]
pub(super) mod test_hook;
