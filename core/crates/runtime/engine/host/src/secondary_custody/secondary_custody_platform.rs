use super::*;
use crate::pty_custody::{CustodyDescriptorSet, CustodyOffer, CustodyProfile};
use std::os::fd::OwnedFd;

const DESCRIPTOR_COUNT_DETAIL: &str = "PTY custody descriptor count changed during import";
const RELEASED_DESCRIPTOR_DETAIL: &str =
    "released PTY successor unexpectedly transferred platform descriptors";
const IMPORT_CONTEXT: &str = "validate transferred PTY custody descriptors";
const IMPORT_PROFILE_DETAIL: &str = "PTY custody import profile does not match its descriptors";
#[cfg(target_os = "linux")]
const LINUX_RECORD_CHANGED_DETAIL: &str =
    "Linux containment record changed during PTY custody import";

pub(super) const fn accepts_descriptor_set(descriptor_set: CustodyDescriptorSet) -> bool {
    #[cfg(target_os = "macos")]
    return matches!(
        descriptor_set,
        CustodyDescriptorSet::MacosProcessCapability { .. }
    );
    #[cfg(target_os = "linux")]
    return matches!(descriptor_set, CustodyDescriptorSet::LinuxContainment);
}

fn released_terminal(offer: &CustodyOffer) -> Result<bool, WorkerRpcError> {
    let valid = match offer.profile {
        CustodyProfile::ActiveRunning | CustodyProfile::ActiveTerminalReadOnly => {
            accepts_descriptor_set(offer.descriptor_set)
        }
        CustodyProfile::ReleasedTerminal => {
            offer.descriptor_set == CustodyDescriptorSet::ReleasedSuccessor
        }
    };
    if !valid {
        return Err(WorkerRpcError::identity_mismatch(IMPORT_PROFILE_DETAIL));
    }
    Ok(offer.profile == CustodyProfile::ReleasedTerminal)
}

#[cfg(target_os = "macos")]
impl Host {
    pub(super) fn import_secondary_custody(
        &self,
        record: &RuntimeRecord,
        offer: &CustodyOffer,
        descriptors: Vec<OwnedFd>,
    ) -> Result<PlatformSecondaryCustody, WorkerRpcError> {
        if released_terminal(offer)? {
            if !descriptors.is_empty() {
                return Err(WorkerRpcError::protocol(RELEASED_DESCRIPTOR_DETAIL));
            }
            return Ok(PlatformSecondaryCustody::ReleasedSuccessor);
        }
        let ordered: [OwnedFd; crate::process::MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT] =
            descriptors
                .try_into()
                .map_err(|_| WorkerRpcError::protocol(DESCRIPTOR_COUNT_DETAIL))?;
        crate::process::MacosSecondaryCustody::from_transferred(
            crate::process::MacosSecondaryCustodyDescriptorParts::from_ordered(ordered),
            &CustodyRuntimeIdentity::from_record(record),
            offer.recovery_identity_sha256,
            offer.descriptor_set,
            offer.profile,
        )
        .map(PlatformSecondaryCustody::Active)
        .map_err(|error| {
            let detail = super::containment::error_detail(error);
            WorkerRpcError::identity_mismatch(format!("{IMPORT_CONTEXT}: {detail}"))
        })
    }
}

#[cfg(target_os = "linux")]
impl Host {
    pub(super) fn import_secondary_custody(
        &self,
        record: &RuntimeRecord,
        offer: &CustodyOffer,
        descriptors: Vec<OwnedFd>,
    ) -> Result<PlatformSecondaryCustody, WorkerRpcError> {
        if released_terminal(offer)? {
            if !descriptors.is_empty() {
                return Err(WorkerRpcError::protocol(RELEASED_DESCRIPTOR_DETAIL));
            }
            return Ok(PlatformSecondaryCustody::ReleasedSuccessor);
        }
        let containment_path = crate::process::linux_containment::record_path(
            &self.paths.session_dir(record.terminal_id),
            record.runtime_epoch,
        );
        let persisted = self
            .read_namespace_json::<crate::process::linux_containment::LinuxContainmentRecord>(
                &containment_path,
            )
            .map_err(|error| WorkerRpcError::transport(super::containment::error_detail(error)))?;
        let ordered: [OwnedFd; crate::process::LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT] =
            descriptors
                .try_into()
                .map_err(|_| WorkerRpcError::protocol(DESCRIPTOR_COUNT_DETAIL))?;
        let custody = crate::process::LinuxSecondaryCustody::from_transferred(
            crate::process::LinuxSecondaryCustodyDescriptorParts::from_ordered(ordered),
            persisted.clone(),
            record.terminal_id,
            record.runtime_epoch,
            crate::process::ProcessIdentity {
                pid: record.child_pid,
                start_time: record.child_start_time,
            },
            offer.recovery_identity_sha256,
            offer.profile,
        )
        .map(PlatformSecondaryCustody::Active)
        .map_err(|error| {
            let detail = super::containment::error_detail(error);
            WorkerRpcError::identity_mismatch(format!("{IMPORT_CONTEXT}: {detail}"))
        })?;
        let current = self
            .read_namespace_json::<crate::process::linux_containment::LinuxContainmentRecord>(
                &containment_path,
            )
            .map_err(|error| WorkerRpcError::transport(super::containment::error_detail(error)))?;
        if current != persisted {
            return Err(WorkerRpcError::identity_mismatch(
                LINUX_RECORD_CHANGED_DETAIL,
            ));
        }
        Ok(custody)
    }
}
