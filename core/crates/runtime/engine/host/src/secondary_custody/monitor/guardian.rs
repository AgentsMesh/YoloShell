use super::*;
use crate::pty_custody::CustodyHolderObservation;
use super::super::failure::CustodyMonitorFault;

pub(in super::super) async fn guardian_fault(
    lease_ref: Weak<SecondaryCustodyLease>,
) -> Result<(), CustodyMonitorFault> {
    let Some(lease) = lease_ref.upgrade() else { return Ok(()) };
    let mut states = lease.subscribe_state();
    drop(lease);
    loop {
        let Some(lease) = lease_ref.upgrade() else { return Ok(()) };
        if lease.guardian_monitor_retired() { return Ok(()); }
        let guardian = lease.guardian;
        drop(lease);
        let Some(guardian) = guardian else {
            if states.changed().await.is_err() { return Ok(()); }
            continue;
        };
        let identity = guardian.binding().process();
        match identity.observation() {
            CustodyHolderObservation::ProvenGone =>
                return Err(CustodyMonitorFault::guardian_gone(GUARDIAN_GONE_DETAIL)),
            CustodyHolderObservation::Indeterminate =>
                return Err(CustodyMonitorFault::failed(GUARDIAN_INDETERMINATE_DETAIL)),
            CustodyHolderObservation::Current => {}
        }
        #[cfg(target_os = "linux")]
        {
            let process = crate::process::ProcessIdentity {
                pid: identity.pid, start_time: identity.start_time,
            };
            tokio::select! {
                _ = process.wait_for_linux_exit() => {}
                changed = states.changed() => {
                    if changed.is_err() { return Ok(()); }
                    continue;
                }
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(crate::host::PROCESS_EXIT_POLL_INTERVAL) => {}
            changed = states.changed() => if changed.is_err() { return Ok(()); }
        }
    }
}
