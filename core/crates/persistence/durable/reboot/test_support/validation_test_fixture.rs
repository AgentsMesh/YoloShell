use super::super::{RebootMigrationIntent, SessionDurableMutationGuard};
use super::super::test_boot::{SOURCE_BOOT, TARGET_BOOT, identity as boot_id};
use super::test_fixture_participants::{custody, generation, targets};
use super::test_fixture_records::{checkpoint, runtime, write_manifest};
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::pty_custody::{CustodyDurableRecord, CustodyRuntimeIdentity};

const NAMESPACE_LABEL: &str = "reboot typed validation";

pub(in crate::durable_reboot) struct Fixture {
    pub(in crate::durable_reboot) _root: tempfile::TempDir,
    pub(in crate::durable_reboot) intent: RebootMigrationIntent,
    pub(in crate::durable_reboot) source_pty: CustodyDurableRecord,
}

impl Fixture {
    pub(in crate::durable_reboot) fn new() -> Self {
        Self::with_pty_source(|runtime| {
            CustodyDurableRecord::pending(CustodyRuntimeIdentity::from_record(runtime))
        })
    }

    pub(in crate::durable_reboot) fn with_pty_source(
        build_pty: impl FnOnce(&crate::model::RuntimeRecord) -> CustodyDurableRecord,
    ) -> Self {
        Self::build(None, build_pty).unwrap()
    }

    pub(in crate::durable_reboot) fn try_with_pty_source_at_generation(
        controller_generation: u64,
        build_pty: impl FnOnce(&crate::model::RuntimeRecord) -> CustodyDurableRecord,
    ) -> anyhow::Result<Self> {
        Self::build(Some(controller_generation), build_pty)
    }

    fn build(
        controller_generation: Option<u64>,
        build_pty: impl FnOnce(&crate::model::RuntimeRecord) -> CustodyDurableRecord,
    ) -> anyhow::Result<Self> {
        let root = tempfile::tempdir().unwrap();
        let (intent, source_pty) = prepare_at(root.path(), controller_generation, build_pty)?;
        Ok(Self {
            _root: root,
            intent,
            source_pty,
        })
    }
}

pub(in crate::durable_reboot) fn prepare_at(
    root: &std::path::Path,
    controller_generation: Option<u64>,
    build_pty: impl FnOnce(&crate::model::RuntimeRecord) -> CustodyDurableRecord,
) -> anyhow::Result<(RebootMigrationIntent, CustodyDurableRecord)> {
    let paths = Paths::new(root);
    let source_boot = boot_id(SOURCE_BOOT);
    let target_boot = boot_id(TARGET_BOOT);
    let mut source = runtime(&source_boot, root);
    if let Some(generation) = controller_generation {
        source.controller_generation = generation;
    }
    paths.ensure_session(source.terminal_id).unwrap();
    write_manifest(&paths, &source);
    let session = paths.session_dir(source.terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
    let checkpoint = checkpoint(&source);
    let custody = custody(&source, &session.join(crate::claude_custody::LEDGER_FILE));
    let generation = generation(&source, &namespace, &paths);
    let pty = build_pty(&source);
    namespace
        .write_json(&paths.runtime_record(source.terminal_id), &source)
        .unwrap();
    namespace
        .write_json(
            &paths.worker_startup_checkpoint(source.terminal_id),
            &checkpoint,
        )
        .unwrap();
    namespace
        .write_json(&session.join(crate::claude_custody::LEDGER_FILE), &custody)
        .unwrap();
    namespace
        .write_json(&paths.pty_custody(source.terminal_id), &pty)
        .unwrap();
    let guard =
        SessionDurableMutationGuard::acquire(&paths, &namespace, source.terminal_id).unwrap();
    let targets = targets(
        source,
        checkpoint,
        custody,
        generation,
        pty.clone(),
        &target_boot,
    );
    let intent = super::super::prepare_intent(&paths, &guard, targets)?;
    drop(guard);
    Ok((intent, pty))
}

pub(in crate::durable_reboot) fn targets_from_intent(
    intent: &RebootMigrationIntent,
) -> super::super::ParticipantTargets {
    super::super::ParticipantTargets {
        startup_checkpoint: payload(&intent.startup_checkpoint).clone(),
        claude_custody: optional_payload(&intent.claude_custody),
        claude_generation: optional_payload(&intent.claude_generation),
        pty_custody: optional_payload(&intent.pty_custody),
        runtime: intent.runtime.target_payload.clone(),
    }
}

fn payload<T>(participant: &super::super::OptionalParticipant<T>) -> &T {
    let super::super::OptionalParticipant::Replace(required) = participant else {
        unreachable!();
    };
    &required.target_payload
}

fn optional_payload<T: Clone>(participant: &super::super::OptionalParticipant<T>) -> Option<T> {
    match participant {
        super::super::OptionalParticipant::Absent => None,
        super::super::OptionalParticipant::Replace(required) => {
            Some(required.target_payload.clone())
        }
    }
}
