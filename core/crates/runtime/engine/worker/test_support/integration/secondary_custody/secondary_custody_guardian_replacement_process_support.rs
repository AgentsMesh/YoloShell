use crate::journal::Journal;
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::process::{
    MacosProcessCapabilityIssuer, MacosSecondaryCustodyExport, PtyMasterAliasReleaseAuthority,
    PtySessionProof,
};
use crate::pty_custody::{
    CustodyDurableRecord, CustodyDurableSnapshot, CustodyHolderIdentity, CustodyOffer,
    CustodyRequest,
};
use crate::worker::namespace::WorkerNamespace;
use portable_pty::{PtySize, native_pty_system};
#[path = "secondary_custody_guardian_replacement_process_children.rs"]
mod children;
#[path = "secondary_custody_guardian_replacement_process_directory.rs"]
mod directory;
#[path = "secondary_custody_guardian_replacement_process_records.rs"]
mod records;
#[path = "secondary_custody_guardian_replacement_process_successor.rs"]
mod successor;
pub(super) use children::DetachedGuardian;
pub(super) use successor::{HostSuccessor, RuntimeFence};

const TEST_RUNTIME_ENVIRONMENT: &str = "YOLO_TEST_RUNTIME";
const TEST_RUNTIME_MISSING: &str = "YOLO_TEST_RUNTIME must name the adversarial yolo runtime";
const NAMESPACE_CAPABILITY: &str = "guardian-replacement-process";
const PTY_ROWS: u16 = 24;
const PTY_COLUMNS: u16 = 80;
const PTY_PIXEL_SIZE: u16 = 0;
pub(super) struct Fixture {
    live: LiveCustody,
    pub(super) paths: Paths,
    pub(super) namespace: WorkerNamespace,
    pub(super) runtime: RuntimeRecord,
    pub(super) runtime_identity: FileIdentity,
    pub(super) predecessor: CustodyDurableSnapshot,
    pub(super) request: CustodyRequest,
    pub(super) offer: CustodyOffer,
    _root: directory::FixtureDirectory,
}

struct LiveCustody {
    _pair: portable_pty::PtyPair,
    _proof: PtySessionProof,
    _alias: PtyMasterAliasReleaseAuthority,
    export: MacosSecondaryCustodyExport,
    host: children::ExactChild,
    worker: children::ExactChild,
    child: children::ExactChild,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::new(root.path());
        let (live, runtime) = LiveCustody::new(&paths);
        paths.ensure_session(runtime.terminal_id).unwrap();
        let namespace = WorkerNamespace::capture(&paths, runtime.terminal_id).unwrap();
        namespace
            .write_json(
                &paths.manifest(runtime.terminal_id),
                &records::manifest(&runtime),
            )
            .unwrap();
        namespace
            .write_json(&paths.runtime_record(runtime.terminal_id), &runtime)
            .unwrap();
        drop(Journal::open(paths.journal(runtime.terminal_id), runtime.runtime_epoch).unwrap());
        let runtime_identity = namespace
            .read_json_with_identity::<RuntimeRecord>(&paths.runtime_record(runtime.terminal_id))
            .unwrap()
            .1;
        let armed = records::armed(
            &runtime,
            live.export.descriptor_set(),
            live.export.recovery_identity_sha256(),
            children::gone_holder(),
            children::gone_holder(),
        );
        let custody_path = paths.pty_custody(runtime.terminal_id);
        namespace.write_json(&custody_path, &armed).unwrap();
        let durable_identity = namespace
            .read_json_with_identity::<CustodyDurableRecord>(&custody_path)
            .unwrap()
            .1;
        let predecessor = CustodyDurableSnapshot::new(armed, durable_identity);
        let (request, offer) = records::successor(
            &runtime,
            holder(live.host.identity()),
            live.export.descriptor_set(),
            live.export.recovery_identity_sha256(),
        );
        Self {
            _root: directory::FixtureDirectory::new(root, &paths),
            live,
            paths,
            namespace,
            runtime,
            runtime_identity,
            predecessor,
            request,
            offer,
        }
    }

    pub(super) fn executable(&self) -> std::path::PathBuf {
        std::env::var_os(TEST_RUNTIME_ENVIRONMENT)
            .expect(TEST_RUNTIME_MISSING)
            .into()
    }

    pub(super) fn descriptors(
        &self,
    ) -> [std::os::fd::BorrowedFd<'_>; crate::process::MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT]
    {
        self.live.export.as_descriptors()
    }

    pub(super) fn current(&self) -> (CustodyDurableRecord, FileIdentity) {
        self.namespace
            .read_json_with_identity(&self.paths.pty_custody(self.runtime.terminal_id))
            .unwrap()
    }

    pub(super) fn current_runtime(&self) -> RuntimeRecord {
        self.namespace
            .read_json(&self.paths.runtime_record(self.runtime.terminal_id))
            .unwrap()
    }

    pub(super) fn terminate_worker(&mut self) {
        self.live.worker.terminate();
    }

    pub(super) fn terminate_host(&mut self) {
        self.live.host.terminate();
    }

    pub(super) fn wait_for_guardian_to_terminate_child(&mut self) {
        self.live.child.wait_for_guardian_termination();
    }
}

impl LiveCustody {
    fn new(paths: &Paths) -> (Self, RuntimeRecord) {
        let pair = native_pty_system().openpty(pty_size()).unwrap();
        let mut proof = PtySessionProof::capture_bound_for_test(
            pair.master.as_ref(),
            NAMESPACE_CAPABILITY.to_owned(),
        )
        .unwrap();
        let (reader, writer) = MacosProcessCapabilityIssuer::create()
            .unwrap()
            .into_staged_for_test()
            .unwrap();
        proof.install_process_capability_for_test(reader).unwrap();
        let host = children::ExactChild::spawn(None);
        let worker = children::ExactChild::spawn(None);
        let child = children::ExactChild::spawn(Some(&writer));
        let runtime = records::runtime(paths, worker.identity(), child.identity());
        let custody_runtime = crate::pty_custody::CustodyRuntimeIdentity::from_record(&runtime);
        let alias = proof
            .bind_secondary_custody_alias_for_test(&custody_runtime)
            .unwrap();
        let export =
            MacosSecondaryCustodyExport::capture_for_test(&proof, &custody_runtime).unwrap();
        drop(writer);
        (
            Self {
                _pair: pair,
                _proof: proof,
                _alias: alias,
                export,
                host,
                worker,
                child,
            },
            runtime,
        )
    }
}

fn holder(identity: crate::process::ProcessIdentity) -> CustodyHolderIdentity {
    CustodyHolderIdentity {
        pid: identity.pid,
        start_time: identity.start_time,
    }
}

fn pty_size() -> PtySize {
    PtySize {
        rows: PTY_ROWS,
        cols: PTY_COLUMNS,
        pixel_width: PTY_PIXEL_SIZE,
        pixel_height: PTY_PIXEL_SIZE,
    }
}
