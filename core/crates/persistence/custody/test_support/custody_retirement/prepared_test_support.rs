use super::{custody, records};
use crate::journal::Journal;
use crate::model::RuntimeRecord;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::process::MacosSecondaryCustody;
use crate::pty_custody::{CustodyDurableLease, CustodyDurableRecord, CustodyRetirementEvidence};

const TEST_NAMESPACE_LABEL: &str = "prepared retirement publisher test session";

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) paths: Paths,
    pub(super) namespace: NamespaceLease,
    pub(super) custody: MacosSecondaryCustody,
    pub(super) prepared: CustodyDurableRecord,
    pub(super) prepared_identity: FileIdentity,
    pub(super) candidate: CustodyDurableLease,
    pub(super) evidence: CustodyRetirementEvidence,
    pub(super) runtime_identity: FileIdentity,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let closed = custody::create();
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::new(root.path());
        paths.ensure_session(closed.runtime.terminal_id).unwrap();
        let session = paths.session_dir(closed.runtime.terminal_id);
        let namespace = NamespaceLease::capture(&[(TEST_NAMESPACE_LABEL, &session)]).unwrap();
        let manifest = records::manifest(&paths, &closed.runtime);
        namespace
            .write_json(&paths.manifest(closed.runtime.terminal_id), &manifest)
            .unwrap();
        let runtime = records::runtime_record(&paths, &closed.runtime);
        let runtime_path = paths.runtime_record(closed.runtime.terminal_id);
        namespace.write_json(&runtime_path, &runtime).unwrap();
        let runtime_identity = namespace
            .read_json_with_identity::<RuntimeRecord>(&runtime_path)
            .unwrap()
            .1;
        let journal_path = paths.journal(closed.runtime.terminal_id);
        drop(Journal::open(&journal_path, closed.runtime.runtime_epoch).unwrap());
        let prepared = records::prepared(&closed.runtime, closed.descriptor_set, closed.digest);
        let candidate = prepared.prepared_guardian_candidate().unwrap();
        let custody_path = paths.pty_custody(closed.runtime.terminal_id);
        namespace.write_json(&custody_path, &prepared).unwrap();
        let prepared_identity = namespace
            .read_json_with_identity::<CustodyDurableRecord>(&custody_path)
            .unwrap()
            .1;
        let journal = Journal::inspect_retained_evidence(
            namespace.file(&journal_path).unwrap(),
            closed.runtime.runtime_epoch,
        )
        .unwrap();
        let evidence =
            journal.custody_retirement_evidence(records::CONTROLLER_GENERATION, runtime_identity);
        Self {
            _root: root,
            paths,
            namespace,
            custody: closed.custody,
            prepared,
            prepared_identity,
            candidate,
            evidence,
            runtime_identity,
        }
    }

    pub(super) fn current(&self) -> (CustodyDurableRecord, FileIdentity) {
        self.namespace
            .read_json_with_identity(&self.paths.pty_custody(self.prepared.runtime.terminal_id))
            .unwrap()
    }

    pub(super) fn replace_same_value(&self) -> (CustodyDurableRecord, FileIdentity) {
        crate::paths::atomic_write_json(
            &self.paths.pty_custody(self.prepared.runtime.terminal_id),
            &self.prepared,
        )
        .unwrap();
        self.current()
    }
}
