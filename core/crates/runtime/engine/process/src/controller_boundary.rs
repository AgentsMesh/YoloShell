use super::{LiveProcessIdentity, ProcessIdentity};
use anyhow::{Context, Result, ensure};
use std::collections::HashSet;

const CONTROLLER_MISSING_DETAIL: &str = "controller process identity is unavailable";
const CONTROLLER_CHAIN_CHANGED_DETAIL: &str = "controller process ancestry changed during capture";
const CONTROLLER_CHAIN_CYCLE_DETAIL: &str = "controller process ancestry contains a cycle";
const EXCLUSION_GONE_DETAIL: &str = "captured process-tree exclusion is unavailable";
const ROOT_EXCLUSION_DETAIL: &str = "captured process-tree root cannot be excluded";
const PROTECTED_SIGNAL_DETAIL: &str =
    "captured process-tree signal targets its controller ancestry";
#[cfg(target_os = "macos")]
const SYSTEM_INIT_PROCESS_ID: u32 = 1;

pub(super) struct ProcessDiscoveryBoundary {
    identities: HashSet<LiveProcessIdentity>,
}

impl ProcessDiscoveryBoundary {
    pub(super) fn capture(
        root: &LiveProcessIdentity,
        excluded: impl IntoIterator<Item = ProcessIdentity>,
    ) -> Result<Self> {
        let mut identities = controller_identities()?;
        identities.remove(root);
        for durable in excluded {
            let identity = LiveProcessIdentity::capture(durable)?.context(EXCLUSION_GONE_DETAIL)?;
            ensure!(&identity != root, ROOT_EXCLUSION_DETAIL);
            identities.insert(identity);
        }
        Ok(Self { identities })
    }

    pub(super) fn contains(&self, identity: &LiveProcessIdentity) -> bool {
        self.identities.contains(identity)
    }

    pub(super) fn identities(&self) -> &HashSet<LiveProcessIdentity> {
        &self.identities
    }

    pub(super) fn lineages(&self) -> HashSet<u64> {
        self.identities
            .iter()
            .map(LiveProcessIdentity::lineage_id)
            .collect()
    }
}

pub(super) fn ensure_signal_targets_are_external(
    targets: &[(LiveProcessIdentity, ProcessIdentity)],
) -> Result<()> {
    let controllers = controller_identities()?;
    ensure!(
        targets
            .iter()
            .all(|(identity, _durable)| !controllers.contains(identity)),
        PROTECTED_SIGNAL_DETAIL
    );
    Ok(())
}

#[cfg(target_os = "macos")]
fn controller_identities() -> Result<HashSet<LiveProcessIdentity>> {
    let mut identities = HashSet::new();
    let mut metadata = yolo_process_identity::macos_lineage::capture(std::process::id())?
        .context(CONTROLLER_MISSING_DETAIL)?;
    loop {
        let identity = LiveProcessIdentity::from_kernel(metadata.pid, metadata.unique_id)?;
        ensure!(identities.insert(identity), CONTROLLER_CHAIN_CYCLE_DETAIL);
        if metadata.parent_pid == u32::default() {
            break;
        }
        let parent =
            LiveProcessIdentity::from_kernel(metadata.parent_pid, metadata.parent_unique_id)?;
        if metadata.parent_pid == SYSTEM_INIT_PROCESS_ID {
            ensure!(identities.insert(parent), CONTROLLER_CHAIN_CYCLE_DETAIL);
            break;
        }
        let observed = yolo_process_identity::macos_lineage::capture(metadata.parent_pid)?
            .context(CONTROLLER_MISSING_DETAIL)?;
        ensure!(
            observed.unique_id == metadata.parent_unique_id,
            CONTROLLER_CHAIN_CHANGED_DETAIL
        );
        metadata = observed;
    }
    Ok(identities)
}

#[cfg(not(target_os = "macos"))]
fn controller_identities() -> Result<HashSet<LiveProcessIdentity>> {
    let durable = ProcessIdentity::capture(std::process::id())?;
    let identity = LiveProcessIdentity::capture(durable)?.context(CONTROLLER_MISSING_DETAIL)?;
    Ok(HashSet::from([identity]))
}
