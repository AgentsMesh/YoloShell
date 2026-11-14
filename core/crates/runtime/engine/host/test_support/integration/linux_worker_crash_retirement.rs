use super::*;
use crate::pty_custody::{CustodyDescriptorSet, CustodyDurableRecord, CustodyRequest};

const LEASE_DIGEST_BYTE: u8 = 0x51;
const JOURNAL_OUTPUT: &[u8] = b"retained Linux crash output";
const REPLACEMENT_EXTENSION: &str = "retirement-replacement";

pub(super) struct Fixture {
    pub(super) host: Host,
    pub(super) manifest: TerminalManifest,
    pub(super) runtime: RuntimeRecord,
    pub(super) armed: CustodyDurableRecord,
    _containment: crate::host::linux_retirement_evidence::LiveFixtureContainment,
    directory: tempfile::TempDir,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let (fixture, mut child) = Self::with_live_child();
        child.stop();
        fixture
    }

    pub(super) fn with_live_child() -> (Self, ArchiveTestProcess) {
        let directory = tempdir().unwrap();
        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        let manifest = fixture_manifest(directory.path());
        let child = ArchiveTestProcess::spawn();
        let worker = crate::host::reaped_host_test_process_identity();
        let mut runtime = host_runtime_record(
            &host,
            manifest.terminal_id,
            host.paths.worker_socket(manifest.terminal_id).unwrap(),
        );
        (runtime.worker_pid, runtime.worker_start_time) = (worker.pid, worker.start_time);
        (runtime.child_pid, runtime.child_start_time) = child.identity();
        persist_host_session(&host, &manifest, &runtime);
        let containment = crate::host::linux_retirement_evidence::capture_live(&host, &runtime);
        let request = CustodyRequest::new(
            crate::pty_custody::CustodyLeaseId::new(),
            crate::pty_custody::CustodyRuntimeIdentity::from_record(&runtime),
            host.secondary_custody_holder,
            runtime.auth_token.clone(),
            runtime.controller_generation,
        );
        let offer = crate::host::test_custody_offer(
            &request,
            CustodyDescriptorSet::LinuxContainment,
            crate::sha256_digest::Sha256Digest::repeated(LEASE_DIGEST_BYTE),
        );
        let armed = CustodyDurableRecord::pending(request.runtime.clone())
            .arm(&request, &offer)
            .unwrap();
        host.namespace
            .write_json(&host.paths.pty_custody(runtime.terminal_id), &armed)
            .unwrap();
        let mut journal = crate::journal::Journal::open(
            host.paths.journal(runtime.terminal_id),
            runtime.runtime_epoch,
        )
        .unwrap();
        journal.append(JOURNAL_OUTPUT).unwrap();
        drop(journal);
        (
            Self {
                host,
                manifest,
                runtime,
                armed,
                _containment: containment,
                directory,
            },
            child,
        )
    }

    pub(super) async fn settle(&self) {
        let _lifecycle = self
            .host
            .acquire_resume_lifecycle_lock(self.runtime.terminal_id, None)
            .await
            .unwrap();
        crate::host::linux_retirement::settle_after_lifecycle_lock(&self.host, &self.runtime)
            .await
            .unwrap();
    }

    pub(super) fn containment_path(&self) -> PathBuf {
        process::linux_containment::record_path(
            &self.host.paths.session_dir(self.runtime.terminal_id),
            self.runtime.runtime_epoch,
        )
    }

    pub(super) fn receipt(&self) -> CustodyDurableRecord {
        self.host
            .namespace
            .read_json(&self.host.paths.pty_custody(self.runtime.terminal_id))
            .unwrap()
    }

    pub(super) fn reopen(self) -> Self {
        let Self {
            host,
            manifest,
            runtime,
            armed,
            _containment,
            directory,
        } = self;
        drop(host);
        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        Self {
            host,
            manifest,
            runtime,
            armed,
            _containment,
            directory,
        }
    }

    pub(super) async fn publish_exit(&self) -> Result<()> {
        let _lifecycle = self
            .host
            .acquire_resume_lifecycle_lock(self.runtime.terminal_id, None)
            .await
            .unwrap();
        self.host
            .ensure_worker_crash_exit_inner(&self.manifest)
            .await
    }

    pub(super) fn exit(&self) -> ExitRecord {
        self.host
            .namespace
            .read_json(&self.host.paths.exit_record(self.runtime.terminal_id))
            .unwrap()
    }
}

pub(super) fn replace_same_value(path: &Path) -> Result<()> {
    let replacement = path.with_extension(REPLACEMENT_EXTENSION);
    std::fs::copy(path, &replacement)?;
    std::fs::rename(replacement, path)?;
    Ok(())
}
