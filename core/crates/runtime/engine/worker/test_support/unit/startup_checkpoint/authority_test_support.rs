use super::retirement_test_support::*;
use super::*;
use crate::model::{BootstrapDelivery, RuntimeRecord};

pub(super) struct AuthorityFixture {
    pub(super) fixture: CheckpointFixture,
    pub(super) predecessor: RuntimeRecord,
    pub(super) successor: WorkerLaunchSpec,
    pub(super) checkpoint: ActiveStartupCheckpoint,
    pub(super) replacement: RuntimeRecord,
}

impl AuthorityFixture {
    pub(super) fn new() -> Self {
        let fixture = CheckpointFixture::new();
        let worker = fixture.worker;
        let child = fixture.child();
        Self::build(fixture, worker, child, None)
    }

    pub(super) fn new_with_predecessor_custody(
        worker: ProcessIdentity,
        child: ProcessIdentity,
        custody: fn(&RuntimeRecord) -> crate::pty_custody::CustodyDurableRecord,
    ) -> Self {
        Self::build(CheckpointFixture::new(), worker, child, Some(custody))
    }

    fn build(
        fixture: CheckpointFixture,
        worker: ProcessIdentity,
        child: ProcessIdentity,
        custody: Option<fn(&RuntimeRecord) -> crate::pty_custody::CustodyDurableRecord>,
    ) -> Self {
        let predecessor = activated_runtime(&fixture);
        persist_manifest(&fixture, &predecessor);
        if let Some(custody) = custody {
            crate::paths::atomic_write_json(
                &fixture.paths.pty_custody(predecessor.terminal_id),
                &custody(&predecessor),
            )
            .unwrap();
        }
        let successor = successor_for(&fixture);
        prepare(&fixture, &successor, Some(&predecessor)).unwrap();
        crate::paths::atomic_write_json(
            &fixture.paths.worker_launch(successor.terminal_id),
            &successor,
        )
        .unwrap();
        let mut checkpoint =
            ActiveStartupCheckpoint::begin(&fixture.namespace, &fixture.paths, &successor, worker)
                .unwrap();
        advance_to_runtime_record(&fixture, &mut checkpoint, child);
        let mut replacement = runtime_for(&fixture, worker, child);
        replacement.runtime_epoch = successor.runtime_epoch;
        replacement.worker_id = successor.worker_id;
        replacement.bootstrap_delivery = BootstrapDelivery::Pending;
        Self {
            fixture,
            predecessor,
            successor,
            checkpoint,
            replacement,
        }
    }

    pub(super) fn publish(&mut self) -> anyhow::Result<()> {
        crate::worker::runtime_record_publication::publish_initial_with_test_hook(
            &self.fixture.namespace,
            &self.fixture.paths,
            &self.replacement,
            &mut self.checkpoint,
            || Ok(()),
        )
    }

    pub(super) fn runtime_bytes(&self) -> Vec<u8> {
        std::fs::read(
            self.fixture
                .paths
                .runtime_record(self.fixture.launch.terminal_id),
        )
        .unwrap()
    }

    pub(super) fn persist_launch(&self, launch: &WorkerLaunchSpec) {
        crate::paths::atomic_write_json(
            &self
                .fixture
                .paths
                .worker_launch(self.fixture.launch.terminal_id),
            launch,
        )
        .unwrap();
    }

    pub(super) fn manifest(&self) -> crate::model::TerminalManifest {
        crate::paths::read_json(&self.fixture.paths.manifest(self.fixture.launch.terminal_id))
            .unwrap()
    }
}

pub(super) fn persist_manifest(fixture: &CheckpointFixture, runtime: &RuntimeRecord) {
    let manifest = crate::model::TerminalManifest {
        schema_version: crate::constants::SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        name: runtime.command.clone(),
        command: runtime.command.clone(),
        args: runtime.args.clone(),
        cwd: runtime.cwd.clone(),
        cols: runtime.cols,
        rows: runtime.rows,
        created_at_ms: runtime.started_at_ms,
        updated_at_ms: runtime.started_at_ms,
        archived: false,
        recovery_state: crate::model::RecoveryState::Created,
    };
    crate::paths::atomic_write_json(&fixture.paths.manifest(runtime.terminal_id), &manifest)
        .unwrap();
}

pub(super) fn advance_to_runtime_record(
    fixture: &CheckpointFixture,
    checkpoint: &mut ActiveStartupCheckpoint,
    child: ProcessIdentity,
) {
    for stage in STARTUP_STAGES {
        checkpoint
            .enter(&fixture.namespace, &fixture.paths, stage)
            .unwrap();
        if stage == StartupStage::RuntimeRecord {
            break;
        }
        if stage == StartupStage::ChildProcess {
            checkpoint
                .complete_child(&fixture.namespace, &fixture.paths, child)
                .unwrap();
        } else {
            checkpoint
                .complete(&fixture.namespace, &fixture.paths, stage)
                .unwrap();
        }
    }
}
