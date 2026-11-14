use crate::process::{
    MacosSecondaryCustody, ProcessIdentity, ProcessIdentityRelation, process_identity_relation,
};
use crate::pty_custody::CustodyRuntimeIdentity;
use anyhow::Result;
use tokio::time::Instant;
const NAMESPACE_SETTLEMENT_DETAIL: &str =
    "PTY custody guardian retained custody while orphaned processes settle";
const NAMESPACE_SETTLEMENT_RETRY_DETAIL: &str =
    "PTY custody guardian retried orphaned process settlement";
const NAMESPACE_CONTAINMENT_RETRY_DETAIL: &str =
    "PTY custody guardian retried exact orphan process containment";

pub(super) use super::settlement_failure_log::ProgressFailureLog;
pub(super) use resolution::{Resolution, resolve};
pub(super) async fn retain_until_processes_settle(
    custody: &MacosSecondaryCustody,
    runtime: &CustodyRuntimeIdentity,
    error: anyhow::Error,
) -> Result<()> {
    let mut containment =
        super::settlement_containment::NamespaceLossContainment::new(runtime, Instant::now());
    super::terminal_reporting::error_ref(&error, NAMESPACE_SETTLEMENT_DETAIL);
    let mut settlement_log = super::settlement_backoff::FailureLogBackoff::default();
    let mut containment_log = super::settlement_backoff::FailureLogBackoff::default();
    loop {
        let exact_settlement = match containment.reconcile_custody(custody, Instant::now()) {
            Ok(settlement) => settlement,
            Err(containment_error) => {
                super::settlement_failure_log::report_throttled(
                    &mut containment_log,
                    containment_error,
                    NAMESPACE_CONTAINMENT_RETRY_DETAIL,
                );
                tokio::time::sleep(super::settlement::POLL_INTERVAL).await;
                continue;
            }
        };
        match namespace_processes_settled(custody, runtime, exact_settlement) {
            Ok(Some(_settled)) => return Err(error),
            Ok(None) => {}
            Err(settlement_error) => super::settlement_failure_log::report_throttled(
                &mut settlement_log,
                settlement_error,
                NAMESPACE_SETTLEMENT_RETRY_DETAIL,
            ),
        }
        tokio::time::sleep(super::settlement::POLL_INTERVAL).await;
    }
}
pub(super) fn processes_settled(
    custody: &MacosSecondaryCustody,
    runtime: &CustodyRuntimeIdentity,
) -> Result<bool> {
    if !gone(worker_identity(runtime))? {
        return Ok(false);
    }
    let _ = custody.request_active_settlement()?;
    passive_processes_settled(custody, runtime)
}

#[must_use]
struct NamespaceRuntimeSettled {
    #[cfg(test)]
    worker: ProcessIdentity,
    #[cfg(test)]
    child: ProcessIdentity,
    _kernel_eof: crate::process::MacosPtyKernelEof,
}

fn namespace_processes_settled(
    custody: &MacosSecondaryCustody,
    runtime: &CustodyRuntimeIdentity,
    exact: Option<super::settlement_containment::NamespaceLossExactSettlement>,
) -> Result<Option<NamespaceRuntimeSettled>> {
    let child = child_identity(runtime);
    if !gone(child)? {
        return Ok(None);
    }
    custody.confirm_root_exit()?;
    let Some(exact) = exact else { return Ok(None) };
    let worker = worker_identity(runtime);
    if !gone(worker)? {
        return Ok(None);
    }
    match capability::exact_settlement(custody)? {
        capability::ExactSettlement::Pending => return Ok(None),
        capability::ExactSettlement::Released
        | capability::ExactSettlement::PrematurelyReleased => {}
    }
    let Some(kernel_eof) = custody.drain_namespace_loss_output_to_kernel_eof(exact.finalized())?
    else {
        return Ok(None);
    };
    Ok(Some(NamespaceRuntimeSettled {
        #[cfg(test)]
        worker,
        #[cfg(test)]
        child,
        _kernel_eof: kernel_eof,
    }))
}

fn passive_processes_settled(
    custody: &MacosSecondaryCustody,
    runtime: &CustodyRuntimeIdentity,
) -> Result<bool> {
    if !gone(child_identity(runtime))? {
        return Ok(false);
    }
    custody.confirm_root_exit()?;
    custody.closure_settled()
}

#[path = "settlement_failure_capability.rs"]
mod capability;

fn gone(identity: ProcessIdentity) -> Result<bool> {
    match process_identity_relation(identity.pid, identity.start_time) {
        ProcessIdentityRelation::Matching => Ok(false),
        ProcessIdentityRelation::Gone => Ok(true),
        ProcessIdentityRelation::Unknown => anyhow::bail!(super::settlement::PROCESS_DETAIL),
    }
}

fn runtime_processes_gone(runtime: &CustodyRuntimeIdentity) -> Result<bool> {
    let worker_gone = gone(worker_identity(runtime))?;
    let child_gone = gone(child_identity(runtime))?;
    Ok(worker_gone && child_gone)
}

#[path = "settlement_failure_resolution.rs"]
mod resolution;

fn worker_identity(runtime: &CustodyRuntimeIdentity) -> ProcessIdentity {
    ProcessIdentity {
        pid: runtime.worker_pid,
        start_time: runtime.worker_start_time,
    }
}

fn child_identity(runtime: &CustodyRuntimeIdentity) -> ProcessIdentity {
    ProcessIdentity {
        pid: runtime.child_pid,
        start_time: runtime.child_start_time,
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/settlement/settlement_failure_containment_tests.rs"]
mod containment_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/settlement/settlement_failure_tests.rs"]
mod tests;
