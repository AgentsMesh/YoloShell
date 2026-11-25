use crate::{constants::*, control::LiveControl};
use anyhow::{Result, bail};
use tokio::time::Instant;
use yolo_hub_management_protocol::{
    CONFIGURATION_REVISION_STEP, ConfigurationChange, ConfigurationSnapshot,
};

pub(crate) async fn apply(
    control: &mut LiveControl,
    change: ConfigurationChange,
    deadline: Instant,
) -> Result<ConfigurationSnapshot> {
    match control.configure(change.clone(), deadline).await {
        Ok(snapshot) => Ok(snapshot),
        Err(error) if error.outcome_is_unknown() => {
            if !control.is_usable() {
                bail!(CONFIGURATION_UNKNOWN);
            }
            inspect(control, &change, deadline).await
        }
        Err(error) => Err(error.into()),
    }
}

async fn inspect(
    control: &mut LiveControl,
    change: &ConfigurationChange,
    deadline: Instant,
) -> Result<ConfigurationSnapshot> {
    while Instant::now() < deadline {
        if let Ok(snapshot) = control.snapshot(deadline).await {
            if confirmed(&snapshot, change) {
                return Ok(snapshot);
            }
            if snapshot.state.revision != change.expected_revision {
                break;
            }
        }
        if !control.is_usable() {
            break;
        }
        tokio::time::sleep_until((Instant::now() + RECONCILE_POLL).min(deadline)).await;
    }
    bail!(CONFIGURATION_UNKNOWN)
}

pub(crate) fn confirmed(snapshot: &ConfigurationSnapshot, change: &ConfigurationChange) -> bool {
    snapshot.applied
        && snapshot.state.last_change == change.request_id
        && change
            .expected_revision
            .checked_add(CONFIGURATION_REVISION_STEP)
            == Some(snapshot.state.revision)
        && snapshot.state.configuration == change.configuration
}
