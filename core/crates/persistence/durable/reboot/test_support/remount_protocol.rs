use super::guard::SessionDurableMutationGuard;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

pub(super) const ROOT_ENV: &str = "YOLO_TEST_DURABLE_REBOOT_ROOT";
pub(super) const PHASE_ENV: &str = "YOLO_TEST_DURABLE_REBOOT_PHASE";
const NAMESPACE_LABEL: &str = "durable reboot remount acceptance";
const HANDOFF_FILE: &str = "reboot-test-handoff.json";
const INVALID_HANDOFF: &str = "invalid durable reboot remount handoff";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Prepare,
    Prerequisites,
    Exchange,
    Runtime,
    Settled,
    Finish,
    Verify,
}

pub(super) const PHASES: [Phase; 7] = [
    Phase::Prepare,
    Phase::Prerequisites,
    Phase::Exchange,
    Phase::Runtime,
    Phase::Settled,
    Phase::Finish,
    Phase::Verify,
];

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Handoff {
    terminal_id: Uuid,
    expected: crate::model::RuntimeRecord,
    completed: Phase,
}

pub(super) fn run(root: &Path, phase: Phase) -> Result<()> {
    ensure!(root.is_absolute(), INVALID_HANDOFF);
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, root)])?;
    let path = root.join(HANDOFF_FILE);
    let paths = Paths::new(root);
    if phase == Phase::Prepare {
        ensure!(namespace.file_is_missing(&path)?, INVALID_HANDOFF);
        let (intent, _) = super::validation::test_fixture::prepare_at(root, None, |runtime| {
            crate::pty_custody::CustodyDurableRecord::pending(
                crate::pty_custody::CustodyRuntimeIdentity::from_record(runtime),
            )
        })?;
        let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, intent.terminal_id)?;
        super::publish_intent(&paths, &guard, &intent)?;
        namespace.write_json(
            &path,
            &Handoff {
                terminal_id: intent.terminal_id,
                expected: intent.runtime.target_payload,
                completed: phase,
            },
        )?;
        return Ok(());
    }
    let mut handoff: Handoff = namespace.read_json(&path)?;
    ensure!(
        PHASES
            .windows(2)
            .any(|pair| pair == [handoff.completed, phase]),
        INVALID_HANDOFF
    );
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, handoff.terminal_id)?;
    super::remount_steps::run(&paths, &guard, phase)?;
    if matches!(phase, Phase::Finish | Phase::Verify) {
        let actual: crate::model::RuntimeRecord =
            namespace.read_json(&paths.runtime_record(handoff.terminal_id))?;
        ensure!(actual == handoff.expected, INVALID_HANDOFF);
        ensure!(
            namespace.file_is_missing(&paths.reboot_migration_intent(handoff.terminal_id))?,
            INVALID_HANDOFF
        );
    }
    handoff.completed = phase;
    namespace.write_json(&path, &handoff)
}
