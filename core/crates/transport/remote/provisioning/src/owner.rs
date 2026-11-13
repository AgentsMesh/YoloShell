use crate::{
    ProvisionGuard, RemoteProvisioningService,
    constants::*,
    journal::{self, Observation},
    record::Record,
};
use anyhow::{Result, ensure};
use yolo_distribution_publication_protocol::PublicationRequest;
use yolo_paths::coordination::CoordinationLock;
use yolo_remote_provisioning_protocol::{ProvisionPhase, ProvisionRequest};

/// Only live construction under a request lock can advance a durable record.
pub(crate) struct Owner {
    lock: CoordinationLock,
    pub observed: Observation,
}
impl RemoteProvisioningService {
    pub(crate) fn lock(&self, request: &ProvisionRequest) -> Result<CoordinationLock> {
        self.verify(request)?;
        CoordinationLock::try_acquire(
            &self
                .root
                .path()
                .join(format!("{}{LOCK_SUFFIX}", request.fields().request_id)),
            LOCK_BYTES,
        )?
        .ok_or_else(|| anyhow::anyhow!(BUSY))
    }
    pub(crate) fn prepare(
        &self,
        request: &ProvisionRequest,
        publication: PublicationRequest,
        lock: CoordinationLock,
        guard: &dyn ProvisionGuard,
    ) -> Result<Owner> {
        self.verify(request)?;
        guard.verify()?;
        lock.verify()?;
        ensure!(
            journal::inspect(&self.parent, &self.root, request)?.is_none(),
            CONFLICT
        );
        let entry = self.root.ensure_descendant(
            &self
                .root
                .path()
                .join(request.fields().request_id.to_string()),
        )?;
        let namespace = journal::namespace(&self.parent, &self.root, &entry)?;
        let path = entry.path().join(RECORD_FILE);
        let record = Record {
            schema: SCHEMA,
            parent: self.parent.object_identity()?,
            root: self.root.object_identity()?,
            entry: entry.object_identity()?,
            request: request.clone(),
            publication,
            phase: ProvisionPhase::Prepared,
            published: None,
            enrollment: None,
        };
        record.validate()?;
        let file = namespace.write_json_expected_with_commit_fence(&path, None, &record, || {
            self.verify(request)?;
            guard.verify()?;
            lock.verify()
        })?;
        entry.sync()?;
        self.root.sync()?;
        let owner = Owner {
            lock,
            observed: Observation {
                namespace,
                path,
                file,
                record,
            },
        };
        owner.verify(guard)?;
        Ok(owner)
    }
    pub(crate) fn advance_owner(
        &self,
        request: &ProvisionRequest,
        lock: CoordinationLock,
    ) -> Result<Owner> {
        let observed = journal::inspect(&self.parent, &self.root, request)?
            .ok_or_else(|| anyhow::anyhow!(UNKNOWN))?;
        ensure!(observed.record.phase == ProvisionPhase::Published, UNKNOWN);
        Ok(Owner { lock, observed })
    }
}
impl Owner {
    pub fn verify(&self, guard: &dyn ProvisionGuard) -> Result<()> {
        guard.verify()?;
        self.lock.verify()?;
        self.observed.verify()
    }
    pub fn update(
        &mut self,
        guard: &dyn ProvisionGuard,
        change: impl FnOnce(&mut Record),
    ) -> Result<()> {
        self.verify(guard)?;
        let mut record = self.observed.record.clone();
        change(&mut record);
        record.validate()?;
        let file = self
            .observed
            .namespace
            .write_json_expected_with_commit_fence(
                &self.observed.path,
                Some(self.observed.file),
                &record,
                || self.verify(guard),
            )?;
        self.observed.file = file;
        self.observed.record = record;
        self.verify(guard)
    }
}
