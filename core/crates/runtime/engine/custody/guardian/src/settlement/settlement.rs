use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::process::MacosSecondaryCustody;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyDurableSnapshot, CustodyDurableState, CustodyGuardianPreparation,
};
use anyhow::{Result, ensure};
use std::time::Duration;
pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(20);
pub(super) const RECORD_DETAIL: &str =
    "detached guardian durable custody record changed unexpectedly";
pub(super) const PROCESS_DETAIL: &str = "detached guardian process observation is indeterminate";
pub(super) use super::settlement_state::GuardianState;

#[path = "settlement_durable_progress.rs"]
mod durable_progress;
pub(super) use durable_progress::Progress;
use durable_progress::progress;
#[path = "../observation/idle_observation.rs"]
mod idle_observation;

include!("settlement_run.rs");

pub(super) fn progress_record(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    state: &mut GuardianState,
    record: CustodyDurableRecord,
    identity: FileIdentity,
) -> Result<Progress> {
    match record.state.clone() {
        CustodyDurableState::Pending {} => {
            ensure!(!state.was_armed(), RECORD_DETAIL);
            Ok(Progress::Pending)
        }
        CustodyDurableState::PreparedGuardian { .. } => {
            match super::settlement_prepared::resolve(namespace, paths, state, &record, identity)? {
                super::settlement_prepared::Resolution::Pending => Ok(Progress::Pending),
                super::settlement_prepared::Resolution::InitialOrphan => {
                    let retired = super::settlement_initial::progress(
                        namespace, paths, custody, &record, identity,
                    )?;
                    Ok(if retired {
                        Progress::Retired
                    } else {
                        Progress::Pending
                    })
                }
                super::settlement_prepared::Resolution::Restored(restored, restored_identity)
                | super::settlement_prepared::Resolution::Armed(restored, restored_identity) => {
                    progress_record(
                        namespace,
                        paths,
                        custody,
                        state,
                        restored,
                        restored_identity,
                    )
                }
            }
        }
        CustodyDurableState::Armed { .. } => {
            #[cfg(any(test, feature = "adversarial-test-hooks"))]
            super::test_probe::publish_armed(
                namespace,
                paths,
                &record,
                identity,
                state.binding(),
                custody.master_fd_custody_probe()?,
            )?;
            match super::settlement_armed::resolve(namespace, paths, state, &record, identity)? {
                super::settlement_armed::Resolution::AwaitingActivation => {
                    return Ok(Progress::Pending);
                }
                super::settlement_armed::Resolution::Continue => {}
                super::settlement_armed::Resolution::Restored(restored, restored_identity) => {
                    return progress_record(
                        namespace,
                        paths,
                        custody,
                        state,
                        restored,
                        restored_identity,
                    );
                }
            }
            if state.needs_armed_promotion() {
                crate::custody_promotion::publish_exact(namespace, paths, &record, identity)?;
                state.adopt_durable_binding(&record)?;
                state.mark_armed_promotion_verified();
            }
            state.mark_armed();
            state.clear_abandoned_prepare(&record)?;
            if !super::settlement_failure::processes_settled(custody, &record.runtime)? {
                return Ok(Progress::Pending);
            }
            let retirement = super::settlement_retirement::publish(
                namespace,
                paths,
                custody,
                &record,
                state.retained_runtime()?.0,
                state.binding(),
            )?;
            Ok(match retirement {
                super::settlement_retirement::Progress::Pending => Progress::Pending,
                super::settlement_retirement::Progress::Retired => Progress::Retired,
            })
        }
        CustodyDurableState::Retired { .. } => {
            state.adopt_durable_binding(&record)?;
            let retirement = super::settlement_retirement::publish(
                namespace,
                paths,
                custody,
                &record,
                state.retained_runtime()?.0,
                state.binding(),
            )?;
            Ok(match retirement {
                super::settlement_retirement::Progress::Pending => Progress::Pending,
                super::settlement_retirement::Progress::Retired => Progress::Retired,
            })
        }
    }
}
