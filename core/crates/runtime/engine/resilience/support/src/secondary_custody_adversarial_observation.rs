use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::RetainedJsonFileObservation;
use crate::paths::identity::FileIdentity;
use crate::process::ProcessIdentity;
use crate::pty_custody::{
    CustodyDurableLease, CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderIdentity,
};
use crate::sha256_digest::Sha256Digest;
use anyhow::{Context, Result, ensure};
use uuid::Uuid;

const READ_CONTEXT: &str = "read durable PTY custody observation";
const INVALID_RECORD_DETAIL: &str = "durable PTY custody observation is invalid";
const SESSION_MISMATCH_DETAIL: &str = "durable PTY custody observation has another session";
const ACTIVE_LEASE_REQUIRED: &str = "durable PTY custody observation is not armed";
const GUARDIAN_REQUIRED: &str = "armed PTY custody observation has no guardian";
const OBSERVATION_CHANGED: &str = "durable PTY custody observation changed";
const OBSERVATION_NAMESPACE_LABEL: &str = "active custody adversarial observation";
const RUNTIME_MISMATCH_DETAIL: &str = "durable PTY custody runtime observation changed";

#[path = "secondary_custody_observation_read.rs"]
mod read;

#[doc(hidden)]
pub const GUARDIAN_NAMESPACE_LOSS_SETTLEMENT_TIMEOUT: std::time::Duration =
    crate::session_timing::GUARDIAN_NAMESPACE_LOSS_SETTLEMENT_TIMEOUT;

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CustodyRecoveryIdentity(Sha256Digest);

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct ActiveCustodyObservation {
    record: CustodyDurableRecord,
    record_identity: FileIdentity,
    runtime_identity: FileIdentity,
    probe: crate::custody_guardian::test_probe::ExactProbeObservation,
    _record_observation: RetainedJsonFileObservation,
}

impl PartialEq for ActiveCustodyObservation {
    fn eq(&self, other: &Self) -> bool {
        self.record == other.record
            && self.record_identity == other.record_identity
            && self.runtime_identity == other.runtime_identity
            && self.probe == other.probe
    }
}

impl Eq for ActiveCustodyObservation {}

impl ActiveCustodyObservation {
    pub fn runtime_epoch(&self) -> Uuid {
        self.record.runtime.runtime_epoch
    }

    pub fn worker_id(&self) -> Uuid {
        self.record.runtime.worker_id
    }

    pub fn controller_generation(&self) -> u64 {
        self.lease().controller_generation
    }

    pub fn host(&self) -> ProcessIdentity {
        process_identity(self.lease().holder)
    }

    pub fn guardian(&self) -> ProcessIdentity {
        process_identity(self.guardian_binding().process())
    }

    pub fn guardian_custody(&self) -> crate::process::PtyMasterFdCustodyProbe {
        self.probe.custody()
    }

    pub async fn recover_authority(
        &self,
        paths: &Paths,
        terminal_id: Uuid,
    ) -> Result<crate::process::PtyCustodyRecoveryAuthority> {
        ensure!(
            self.record.runtime.terminal_id == terminal_id,
            SESSION_MISMATCH_DETAIL
        );
        self.confirm_unchanged(paths, terminal_id)?;
        let authority = crate::custody_guardian::recover_authority(
            paths,
            self.runtime_identity,
            &self.record,
            self.record_identity,
            self.probe.custody(),
        )
        .await?;
        self.confirm_unchanged(paths, terminal_id)?;
        Ok(authority)
    }

    pub fn recovery_identity(&self) -> CustodyRecoveryIdentity {
        CustodyRecoveryIdentity(self.lease().recovery_identity_sha256)
    }

    pub fn matches_runtime(&self, runtime: &crate::model::RuntimeRecord) -> bool {
        self.record.runtime.matches_record(runtime)
            && self.controller_generation() == runtime.controller_generation
    }

    pub fn confirm_unchanged(&self, paths: &Paths, terminal_id: Uuid) -> Result<()> {
        self.confirm_retained_unchanged(paths, terminal_id)
    }

    pub fn confirm_retained_unchanged(&self, paths: &Paths, terminal_id: Uuid) -> Result<()> {
        ensure!(
            self.record.runtime.terminal_id == terminal_id,
            SESSION_MISMATCH_DETAIL
        );
        let session_dir = paths.session_dir(terminal_id);
        let namespace = NamespaceLease::capture(&[(OBSERVATION_NAMESPACE_LABEL, &session_dir)])
            .context(READ_CONTEXT)?;
        self._record_observation
            .verify_unchanged()
            .context(OBSERVATION_CHANGED)?;
        self.probe
            .confirm_retained_unchanged()
            .context(OBSERVATION_CHANGED)?;
        let custody: (CustodyDurableRecord, FileIdentity) = namespace
            .read_json_with_identity(&paths.pty_custody(terminal_id))
            .context(READ_CONTEXT)?;
        ensure!(
            custody == (self.record.clone(), self.record_identity),
            OBSERVATION_CHANGED
        );
        let (runtime, runtime_identity): (crate::model::RuntimeRecord, FileIdentity) = namespace
            .read_json_with_identity(&paths.runtime_record(terminal_id))
            .context(READ_CONTEXT)?;
        ensure!(
            runtime_identity == self.runtime_identity
                && self.record.runtime.matches_record(&runtime)
                && self.controller_generation() == runtime.controller_generation,
            RUNTIME_MISMATCH_DETAIL
        );
        crate::custody_guardian::test_probe::confirm_durable_unchanged(
            &namespace,
            paths,
            &self.record,
            self.record_identity,
        )
    }

    fn lease(&self) -> CustodyDurableLease {
        self.record.armed_lease().expect(ACTIVE_LEASE_REQUIRED)
    }

    fn guardian_binding(&self) -> CustodyGuardianBinding {
        self.lease().guardian.expect(GUARDIAN_REQUIRED)
    }
}

fn process_identity(identity: CustodyHolderIdentity) -> ProcessIdentity {
    ProcessIdentity {
        pid: identity.pid,
        start_time: identity.start_time,
    }
}
