use crate::custody_guardian::protocol::GuardianBootstrap;
use crate::model::RuntimeRecord;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::process::{ProcessIdentityRelation, ProcessIdentityRelationGuard};
#[cfg(target_os = "macos")]
use crate::pty_custody::CustodyGuardianPreparation;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyDurableSnapshot, CustodyGuardianBinding, CustodyLeaseId,
    CustodyOffer, CustodyRequest,
};
use uuid::Uuid;

#[path = "replacement_test_records.rs"]
mod records;
use records::*;

const NAMESPACE_LABEL: &str = "guardian replacement test session";

pub(in crate::custody_guardian) struct Fixture {
    _root: tempfile::TempDir,
    _old_host_relation: ProcessIdentityRelationGuard,
    _old_guardian_relation: ProcessIdentityRelationGuard,
    _child_relation: Option<ProcessIdentityRelationGuard>,
    pub(in crate::custody_guardian) paths: Paths,
    pub(in crate::custody_guardian) namespace: NamespaceLease,
    pub(in crate::custody_guardian) runtime: RuntimeRecord,
    pub(in crate::custody_guardian) runtime_identity: FileIdentity,
    pub(in crate::custody_guardian) snapshot: CustodyDurableSnapshot,
    pub(super) request: CustodyRequest,
    pub(super) offer: CustodyOffer,
    pub(super) bootstrap: GuardianBootstrap,
}

impl Fixture {
    pub(super) fn new(guardian_relation: ProcessIdentityRelation) -> Self {
        Self::build(guardian_relation, None)
    }

    pub(in crate::custody_guardian) fn new_with_distinct_child(
        guardian_relation: ProcessIdentityRelation,
    ) -> Self {
        Self::build(
            guardian_relation,
            Some(crate::process::ProcessIdentity {
                pid: CHECKPOINT_CHILD_PID,
                start_time: CHECKPOINT_CHILD_START,
            }),
        )
    }

    fn build(
        guardian_relation: ProcessIdentityRelation,
        child: Option<crate::process::ProcessIdentity>,
    ) -> Self {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::new(root.path());
        let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
        let child = child.unwrap_or(process);
        let runtime = runtime(&paths, process, child);
        let child_relation = (child != process).then(|| {
            crate::process::install_test_process_identity_relation(
                child.pid,
                child.start_time,
                ProcessIdentityRelation::Matching,
            )
        });
        paths.ensure_session(runtime.terminal_id).unwrap();
        let session = paths.session_dir(runtime.terminal_id);
        let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
        namespace
            .write_json(&paths.manifest(runtime.terminal_id), &manifest(&runtime))
            .unwrap();
        namespace
            .write_json(&paths.runtime_record(runtime.terminal_id), &runtime)
            .unwrap();
        let runtime_identity = namespace
            .read_json_with_identity::<RuntimeRecord>(&paths.runtime_record(runtime.terminal_id))
            .unwrap()
            .1;
        let old_host = holder(OLD_HOST_PID, OLD_HOST_START);
        let old_guardian = holder(OLD_GUARDIAN_PID, OLD_GUARDIAN_START);
        let armed = armed(&runtime, old_host, old_guardian);
        let custody_path = paths.pty_custody(runtime.terminal_id);
        namespace.write_json(&custody_path, &armed).unwrap();
        let durable_identity = namespace
            .read_json_with_identity::<CustodyDurableRecord>(&custody_path)
            .unwrap()
            .1;
        let snapshot = CustodyDurableSnapshot::new(armed, durable_identity);
        let request = successor_request(&runtime, process);
        let offer = successor_offer(&request);
        let bootstrap = GuardianBootstrap::new(
            Uuid::new_v4(),
            request.clone(),
            offer.clone(),
            runtime_identity,
            snapshot.clone(),
        );
        Self {
            _root: root,
            _old_host_relation: relation(old_host, ProcessIdentityRelation::Gone),
            _old_guardian_relation: relation(old_guardian, guardian_relation),
            _child_relation: child_relation,
            paths,
            namespace,
            runtime,
            runtime_identity,
            snapshot,
            request,
            offer,
            bootstrap,
        }
    }

    pub(in crate::custody_guardian) fn current(&self) -> (CustodyDurableRecord, FileIdentity) {
        self.namespace
            .read_json_with_identity(&self.paths.pty_custody(self.runtime.terminal_id))
            .unwrap()
    }

    #[cfg(target_os = "macos")]
    pub(in crate::custody_guardian) fn retained_runtime_file(
        &self,
    ) -> crate::custody_guardian::settlement_runtime_file::RetainedRuntimeFile {
        crate::custody_guardian::settlement_runtime_file::RetainedRuntimeFile::capture(
            &self.namespace,
            &self.paths,
            &self.runtime,
            self.runtime_identity,
        )
        .unwrap()
    }

    pub(super) fn validate(&self, bootstrap: &GuardianBootstrap) -> anyhow::Result<()> {
        let (durable, identity) = self.current();
        crate::custody_guardian::bootstrap_validation::validate(
            bootstrap,
            &self.runtime,
            self.runtime_identity,
            &durable,
            identity,
        )
    }

    pub(super) fn binding(&self) -> CustodyGuardianBinding {
        CustodyGuardianBinding::new(
            self.bootstrap.guardian_id,
            self.request.holder(),
            &self.request,
            &self.offer,
        )
        .unwrap()
    }

    #[cfg(target_os = "macos")]
    pub(in crate::custody_guardian) fn install(
        &self,
    ) -> anyhow::Result<CustodyGuardianPreparation> {
        crate::custody_guardian::preparation::install(
            &self.namespace,
            &self.paths,
            self.snapshot.record(),
            self.snapshot.file_identity(),
            self.runtime_identity,
            &self.request,
            &self.offer,
            self.binding(),
        )
    }

    pub(super) fn replace_same_value(&self) -> (CustodyDurableRecord, FileIdentity) {
        let path = self.paths.pty_custody(self.runtime.terminal_id);
        crate::paths::atomic_write_json(&path, self.snapshot.record()).unwrap();
        self.current()
    }

    pub(super) fn mixed_bootstrap(&self) -> GuardianBootstrap {
        let other = CustodyRequest::new(
            CustodyLeaseId::new(),
            self.request.runtime.clone(),
            self.request.holder(),
            AUTH_TOKEN.to_owned(),
            NEW_GENERATION,
        );
        let mut mixed = self.bootstrap.clone();
        mixed.offer = successor_offer(&other);
        mixed
    }
}
