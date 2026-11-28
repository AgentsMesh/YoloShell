use super::*;
use crate::pty_custody::{
    CustodyDurableSnapshot, CustodyDurableState, CustodyGuardianBinding,
    CustodyGuardianPreparation, CustodyHolderObservation,
};

const RUNTIME_CHANGED_DETAIL: &str = "PTY custody runtime changed before guardian preparation";
const GUARDIAN_REQUIRED_DETAIL: &str = "active PTY custody predecessor has no detached guardian";
const GUARDIAN_INDETERMINATE_DETAIL: &str =
    "active PTY custody predecessor guardian identity is indeterminate";
const PREPARED_ORPHAN_DETAIL: &str =
    "prepared PTY custody candidate is not an exact recoverable orphan";
const GUARDIAN_CONTEXT: &str = "prepare detached PTY custody guardian";
#[cfg(test)]
const TEST_RUNTIME_ENVIRONMENT: &str = "YOLO_TEST_RUNTIME";

impl Host {
    #[cfg(target_os = "macos")]
    pub(super) async fn prepare_secondary_custody_guardian(
        &self,
        record: &RuntimeRecord,
        request: &crate::pty_custody::CustodyRequest,
        offer: &CustodyOffer,
        predecessor: &CustodyDurableSnapshot,
        platform: &PlatformSecondaryCustody,
    ) -> Result<Option<CustodyGuardianPreparation>, WorkerRpcError> {
        if offer.profile == crate::pty_custody::CustodyProfile::ReleasedTerminal {
            return Ok(None);
        }
        let runtime_path = self.paths.runtime_record(record.terminal_id);
        let (current, identity): (RuntimeRecord, crate::paths::identity::FileIdentity) = self
            .namespace
            .read_json_with_identity(&runtime_path)
            .map_err(transport)?;
        if current != *record {
            return Err(WorkerRpcError::identity_mismatch(RUNTIME_CHANGED_DETAIL));
        }
        let binding = match &predecessor.record().state {
            CustodyDurableState::Retired { .. } => {
                return Err(WorkerRpcError::identity_mismatch(GUARDIAN_REQUIRED_DETAIL));
            }
            CustodyDurableState::PreparedGuardian { .. } => {
                let candidate = predecessor
                    .record()
                    .prepared_guardian_candidate()
                    .ok_or_else(|| WorkerRpcError::identity_mismatch(PREPARED_ORPHAN_DETAIL))?;
                let guardian = candidate
                    .guardian
                    .ok_or_else(|| WorkerRpcError::identity_mismatch(PREPARED_ORPHAN_DETAIL))?;
                if candidate.holder.observation() != CustodyHolderObservation::ProvenGone {
                    return Err(WorkerRpcError::identity_mismatch(PREPARED_ORPHAN_DETAIL));
                }
                match guardian_route(guardian)? {
                    GuardianRoute::Current => {
                        crate::custody_guardian::prepare_existing(
                            &self.paths,
                            guardian,
                            request,
                            offer,
                            identity,
                            predecessor,
                        )
                        .await
                    }
                    GuardianRoute::Replacement => {
                        let descriptors = platform.guardian_descriptors().ok_or_else(|| {
                            WorkerRpcError::identity_mismatch(GUARDIAN_REQUIRED_DETAIL)
                        })?;
                        crate::custody_guardian::spawn_successor(
                            &self.guardian_executable(),
                            &self.paths,
                            record.terminal_id,
                            request,
                            offer,
                            identity,
                            predecessor.clone(),
                            &descriptors,
                        )
                        .await
                    }
                }
            }
            CustodyDurableState::Pending {} => {
                let descriptors = platform
                    .guardian_descriptors()
                    .ok_or_else(|| WorkerRpcError::identity_mismatch(GUARDIAN_REQUIRED_DETAIL))?;
                crate::custody_guardian::spawn_initial(
                    &self.guardian_executable(),
                    &self.paths,
                    record.terminal_id,
                    request,
                    offer,
                    identity,
                    predecessor.clone(),
                    &descriptors,
                )
                .await
            }
            CustodyDurableState::Armed { guardian, .. } => {
                let Some(guardian) = *guardian else {
                    return Err(WorkerRpcError::identity_mismatch(GUARDIAN_REQUIRED_DETAIL));
                };
                match guardian_route(guardian)? {
                    GuardianRoute::Current => {
                        crate::custody_guardian::prepare_existing(
                            &self.paths,
                            guardian,
                            request,
                            offer,
                            identity,
                            predecessor,
                        )
                        .await
                    }
                    GuardianRoute::Replacement => {
                        let descriptors = platform.guardian_descriptors().ok_or_else(|| {
                            WorkerRpcError::identity_mismatch(GUARDIAN_REQUIRED_DETAIL)
                        })?;
                        crate::custody_guardian::spawn_successor(
                            &self.guardian_executable(),
                            &self.paths,
                            record.terminal_id,
                            request,
                            offer,
                            identity,
                            predecessor.clone(),
                            &descriptors,
                        )
                        .await
                    }
                }
            }
        }
        .map_err(transport)?;
        Ok(Some(binding))
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) async fn prepare_secondary_custody_guardian(
        &self,
        _record: &RuntimeRecord,
        _request: &crate::pty_custody::CustodyRequest,
        _offer: &CustodyOffer,
        _predecessor: &CustodyDurableSnapshot,
        _platform: &PlatformSecondaryCustody,
    ) -> Result<Option<CustodyGuardianPreparation>, WorkerRpcError> {
        Ok(None)
    }

    #[cfg(target_os = "macos")]
    fn guardian_executable(&self) -> std::path::PathBuf {
        #[cfg(test)]
        if let Some(executable) = std::env::var_os(TEST_RUNTIME_ENVIRONMENT) {
            return executable.into();
        }
        self.runtime_executable.clone()
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GuardianRoute {
    Current,
    Replacement,
}

#[cfg(target_os = "macos")]
fn guardian_route(binding: CustodyGuardianBinding) -> Result<GuardianRoute, WorkerRpcError> {
    match binding.process().observation() {
        CustodyHolderObservation::Current => Ok(GuardianRoute::Current),
        CustodyHolderObservation::ProvenGone => Ok(GuardianRoute::Replacement),
        CustodyHolderObservation::Indeterminate => Err(WorkerRpcError::identity_mismatch(
            GUARDIAN_INDETERMINATE_DETAIL,
        )),
    }
}

fn transport(error: anyhow::Error) -> WorkerRpcError {
    WorkerRpcError::transport(format!("{GUARDIAN_CONTEXT}: {error}"))
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/secondary_custody/secondary_custody_guardian_replacement_tests.rs"]
mod tests;
