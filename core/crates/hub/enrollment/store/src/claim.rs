use crate::{
    Claim, EnrollmentRecord, Observation, PreparedEnrollment, constants::*, record::StoredRecord,
};
use anyhow::{Result, ensure};
use yolo_hub_delegation::PeerIdentity;
use yolo_remote_provisioning_protocol::{RemoteBootstrapReceipt, RemoteBootstrapRequest};
use yolo_paths::{directory::PrivateDirectory, namespace_lease::NamespaceLease};

pub enum Admission {
    Existing(Observation),
    Fresh(Claim),
}
pub struct SubmitPermit {
    observation: Observation,
    claim: Claim,
}

pub(crate) fn claim(
    parent: PrivateDirectory,
    peer: PeerIdentity,
    request: &RemoteBootstrapRequest,
) -> Result<Admission> {
    crate::admit_startup(parent, request)?
        .ok_or_else(|| anyhow::anyhow!(BUSY))?
        .claim(peer)
}

impl Claim {
    pub fn prepare(self, prepared: PreparedEnrollment) -> Result<SubmitPermit> {
        self.verify()?;
        prepared.validate()?;
        ensure!(
            prepared.request == self.request && prepared.peer == self.peer,
            CONFLICT
        );
        let entry = self.root.ensure_descendant(
            &self
                .root
                .path()
                .join(self.request.fields().request_id.to_string()),
        )?;
        let namespace = NamespaceLease::from_directories(vec![
            (PARENT_NAMESPACE, self.parent.try_clone()?),
            (ROOT_NAMESPACE, self.root.try_clone()?),
            (ENTRY_NAMESPACE, entry.try_clone()?),
        ])?;
        let path = entry.path().join(RECORD_FILE);
        let stored = StoredRecord {
            schema_version: RECORD_SCHEMA,
            parent: self.parent.durable_identity()?,
            root: self.root.durable_identity()?,
            entry: entry.durable_identity()?,
            record: EnrollmentRecord {
                prepared,
                completion: None,
            },
        };
        stored.validate(&self.parent, &self.root, &entry)?;
        let identity =
            namespace
                .write_json_expected_with_commit_fence(&path, None, &stored, || self.verify())?;
        entry.sync()?;
        self.root.sync()?;
        let observation = Observation {
            namespace,
            path,
            identity,
            stored,
        };
        observation.verify()?;
        self.verify()?;
        Ok(SubmitPermit {
            observation,
            claim: self,
        })
    }
    fn verify(&self) -> Result<()> {
        self.parent.verify_path()?;
        self.root.verify_path()?;
        self.lock.verify()
    }
}

impl SubmitPermit {
    pub fn prepared(&self) -> &PreparedEnrollment {
        &self.observation.record().prepared
    }
    pub fn verify(&self) -> Result<()> {
        self.claim.verify()?;
        self.observation.verify()
    }
    pub fn complete(mut self, receipt: RemoteBootstrapReceipt) -> Result<EnrollmentRecord> {
        self.verify()?;
        let mut stored = self.observation.stored.clone();
        stored.record.completion = Some(receipt);
        stored.record.validate()?;
        let identity = self
            .observation
            .namespace
            .write_json_expected_with_commit_fence(
                &self.observation.path,
                Some(self.observation.identity),
                &stored,
                || self.verify(),
            )?;
        self.observation.identity = identity;
        self.observation.stored = stored;
        self.verify()?;
        Ok(self.observation.record().clone())
    }
}
