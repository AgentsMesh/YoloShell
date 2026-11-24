use super::*;
use crate::pty_custody::CustodyControl;
use anyhow::Context;

const PLATFORM_UNAVAILABLE_DETAIL: &str =
    "PTY secondary-custody platform bundle is unavailable during settlement";
const SETTLED_COMMIT_FAILURE_CONTEXT: &str = "commit proven-gone PTY secondary-custody retirement";
const ACTIVE_SETTLEMENT_FAILURE_CONTEXT: &str = "request active PTY secondary-custody settlement";
const SETTLEMENT_LEASE_MISMATCH_DETAIL: &str =
    "exact PTY secondary-custody settlement belongs to a different lease";
const RELEASE_CONTROL_REQUIRED_CONTEXT: &str =
    "exact PTY custody settlement requires a Release control";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActiveSettlementProgress {
    Requested,
    Settled,
}

pub(super) struct ExactSecondaryCustodySettlement<'lease> {
    lease: &'lease SecondaryCustodyLease,
}

impl ExactSecondaryCustodySettlement<'_> {
    pub(super) fn validate(&self, lease: &SecondaryCustodyLease) -> Result<(), WorkerRpcError> {
        std::ptr::eq(self.lease, lease)
            .then_some(())
            .ok_or_else(|| WorkerRpcError::identity_mismatch(SETTLEMENT_LEASE_MISMATCH_DETAIL))
    }
}

pub(super) async fn retain_until_settled(lease: Weak<SecondaryCustodyLease>) {
    retain_until_terminal(lease, SettlementTerminal::Released).await;
}

pub(super) async fn retain_until_superseded(lease: Weak<SecondaryCustodyLease>) {
    retain_until_terminal(lease, SettlementTerminal::Superseded).await;
}

#[derive(Clone, Copy)]
enum SettlementTerminal {
    Released,
    Superseded,
}

async fn retain_until_terminal(lease: Weak<SecondaryCustodyLease>, terminal: SettlementTerminal) {
    let Some(active) = lease.upgrade() else {
        return;
    };
    if !active.begin_settlement() {
        return;
    }
    drop(active);
    let _guard = SettlementGuard(lease.clone());
    loop {
        let Some(active) = lease.upgrade() else {
            return;
        };
        if let Err(error) = request_active_settlement(&active) {
            let detail = super::containment::error_detail(error);
            super::containment::fail_stop(
                &active.request_drain,
                format!("{ACTIVE_SETTLEMENT_FAILURE_CONTEXT}: {detail}"),
            );
            return;
        }
        match prove_exact(&active) {
            Ok(Some(settlement)) => {
                let committed = match terminal {
                    SettlementTerminal::Released => active.commit_settled_release(settlement),
                    SettlementTerminal::Superseded => active.commit_settled_superseded(settlement),
                };
                if let Err(error) = committed {
                    let detail = super::containment::display_detail(&error);
                    super::containment::fail_stop(
                        &active.request_drain,
                        format!("{SETTLED_COMMIT_FAILURE_CONTEXT}: {detail}"),
                    );
                }
                return;
            }
            Ok(None) => {}
            Err(error) => {
                let detail = super::containment::error_detail(error);
                super::containment::fail_stop(&active.request_drain, detail);
                return;
            }
        }
        drop(active);
        tokio::time::sleep(super::super::PROCESS_EXIT_POLL_INTERVAL).await;
    }
}

pub(super) async fn wait_for_exact(
    lease: &SecondaryCustodyLease,
) -> anyhow::Result<ExactSecondaryCustodySettlement<'_>> {
    loop {
        if let Some(settlement) = prove_exact(lease)? {
            return Ok(settlement);
        }
        tokio::time::sleep(super::super::PROCESS_EXIT_POLL_INTERVAL).await;
    }
}

pub(super) async fn wait_for_exact_release<'lease>(
    lease: &'lease SecondaryCustodyLease,
    control: &CustodyControl,
) -> anyhow::Result<ExactSecondaryCustodySettlement<'lease>> {
    let settlement = control
        .release_settlement()
        .context(RELEASE_CONTROL_REQUIRED_CONTEXT)?;
    loop {
        if let Some(settlement) = prove_release(lease, settlement)? {
            return Ok(settlement);
        }
        tokio::time::sleep(super::super::PROCESS_EXIT_POLL_INTERVAL).await;
    }
}

pub(super) fn prove_exact(
    lease: &SecondaryCustodyLease,
) -> anyhow::Result<Option<ExactSecondaryCustodySettlement<'_>>> {
    exact(lease).map(|settled| settled.then_some(ExactSecondaryCustodySettlement { lease }))
}

fn request_active_settlement(
    lease: &SecondaryCustodyLease,
) -> anyhow::Result<ActiveSettlementProgress> {
    let platform = super::super::lock_recover(&lease.platform);
    let settled = platform
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!(PLATFORM_UNAVAILABLE_DETAIL))?
        .request_active_settlement()?;
    Ok(if settled {
        ActiveSettlementProgress::Settled
    } else {
        ActiveSettlementProgress::Requested
    })
}

fn exact(lease: &SecondaryCustodyLease) -> anyhow::Result<bool> {
    if !super::super::process_identity_is_gone(
        lease.runtime.child_pid,
        lease.runtime.child_start_time,
    ) {
        return Ok(false);
    }
    lease.confirm_platform_root_exit()?;
    lease.platform_closure_settled()
}

fn prove_release<'lease>(
    lease: &'lease SecondaryCustodyLease,
    settlement: &crate::pty_custody::CustodyReleaseSettlement,
) -> anyhow::Result<Option<ExactSecondaryCustodySettlement<'lease>>> {
    if !super::super::process_identity_is_gone(
        lease.runtime.child_pid,
        lease.runtime.child_start_time,
    ) {
        return Ok(None);
    }
    lease.confirm_platform_root_exit()?;
    lease
        .platform_closure_settled_for(settlement)
        .map(|settled| settled.then_some(ExactSecondaryCustodySettlement { lease }))
}

struct SettlementGuard(Weak<SecondaryCustodyLease>);

impl Drop for SettlementGuard {
    fn drop(&mut self) {
        if let Some(lease) = self.0.upgrade() {
            lease.end_settlement();
        }
    }
}
