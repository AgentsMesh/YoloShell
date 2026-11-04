use super::*;
use crate::pty_custody::{CustodyDurableLease, CustodyGuardianBinding};

const GUARDIAN_DETAIL: &str =
    "settled PTY secondary custody has no exact detached guardian binding";
const PUBLICATION_CONTEXT: &str = "publish settled PTY secondary-custody retirement";

pub(super) struct SecondaryCustodyDurableRetirement {
    namespace: Arc<crate::namespace_lease::NamespaceLease>,
    paths: crate::paths::Paths,
}

impl SecondaryCustodyDurableRetirement {
    pub(super) fn new(host: &Host) -> Self {
        Self {
            namespace: Arc::clone(&host.namespace),
            paths: host.paths.clone(),
        }
    }

    fn acquire(
        &self,
        runtime: &CustodyRuntimeIdentity,
    ) -> Result<yolo_paths::durable_mutation::SessionDurableMutationGuard<'_>, WorkerRpcError> {
        yolo_paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
            &self.paths,
            &self.namespace,
            runtime.terminal_id,
        )
        .map_err(publication_error)
    }

    fn publish_guardian(
        &self,
        guard: &yolo_paths::durable_mutation::SessionDurableMutationGuard<'_>,
        custody: &crate::process::MacosSecondaryCustody,
        runtime: &CustodyRuntimeIdentity,
        lease: CustodyDurableLease,
    ) -> Result<(), WorkerRpcError> {
        crate::custody_retirement::publish_exact_with_guard(
            guard,
            &self.paths,
            custody,
            runtime,
            lease,
            crate::claude_custody::settle_runtime_retirement,
        )
        .map_err(publication_error)
    }
}

impl SecondaryCustodyLease {
    pub(super) fn acquire_durable_retirement_participant(
        &self,
    ) -> Result<yolo_paths::durable_mutation::SessionDurableMutationGuard<'_>, WorkerRpcError> {
        self.durable_retirement.acquire(&self.runtime)
    }

    pub(super) fn publish_durable_retirement(
        &self,
        guard: &yolo_paths::durable_mutation::SessionDurableMutationGuard<'_>,
        platform: &PlatformSecondaryCustody,
    ) -> Result<(), WorkerRpcError> {
        let custody = match platform {
            PlatformSecondaryCustody::Active(custody) => custody,
            PlatformSecondaryCustody::ReleasedSuccessor => return Ok(()),
        };
        let binding = self.guardian.map(|preparation| preparation.binding());
        let lease = durable_lease(self, binding);
        let binding = binding.ok_or_else(|| WorkerRpcError::identity_mismatch(GUARDIAN_DETAIL))?;
        if !binding.matches_durable_lease(&self.runtime, &lease) {
            return Err(WorkerRpcError::identity_mismatch(GUARDIAN_DETAIL));
        }
        self.durable_retirement
            .publish_guardian(guard, custody, &self.runtime, lease)
    }
}

fn publication_error(error: anyhow::Error) -> WorkerRpcError {
    let detail = super::containment::error_detail(error);
    WorkerRpcError::transport(format!("{PUBLICATION_CONTEXT}: {detail}"))
}

fn durable_lease(
    lease: &SecondaryCustodyLease,
    guardian: Option<CustodyGuardianBinding>,
) -> CustodyDurableLease {
    CustodyDurableLease {
        lease_id: lease.offer.lease_id,
        offer_id: lease.offer.offer_id,
        holder: lease.holder,
        controller_generation: lease.controller_generation,
        descriptor_set: lease.offer.descriptor_set,
        profile: lease.offer.profile,
        recovery_identity_sha256: lease.offer.recovery_identity_sha256,
        guardian,
    }
}
