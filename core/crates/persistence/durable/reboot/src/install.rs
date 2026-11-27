use super::constants::*;
use super::guard::SessionDurableMutationGuard;
use super::intent_io::LoadedIntent;
use crate::paths::Paths;
use anyhow::Result;

pub(super) fn install_prerequisites(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedIntent,
) -> Result<()> {
    super::intent_io::verify_exact(paths, guard, loaded)?;
    let intent = &loaded.value;
    let participant_paths =
        super::participant_paths::ParticipantPaths::new(paths, intent.terminal_id)?;
    super::install_optional::install(
        paths,
        guard,
        loaded,
        &participant_paths.startup_checkpoint,
        &intent.startup_checkpoint,
    )?;
    super::install_optional::install(
        paths,
        guard,
        loaded,
        &participant_paths.claude_custody,
        &intent.claude_custody,
    )?;
    super::install_optional::install(
        paths,
        guard,
        loaded,
        &participant_paths.claude_generation,
        &intent.claude_generation,
    )?;
    super::install_pty::install(
        paths,
        guard,
        loaded,
        &participant_paths.pty_custody,
        &intent.pty_custody,
    )
}

pub(super) fn install_runtime(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedIntent,
) -> Result<()> {
    super::intent_io::verify_exact(paths, guard, loaded)?;
    let intent = &loaded.value;
    let participant_paths =
        super::participant_paths::ParticipantPaths::new(paths, intent.terminal_id)?;
    super::install_participant::install(
        guard,
        &participant_paths.runtime,
        intent,
        RUNTIME_PARTICIPANT,
        &intent.runtime,
        || {
            super::intent_io::verify_exact(paths, guard, loaded)?;
            verify_prerequisites(paths, guard, loaded)
        },
    )
}

pub(super) fn verify_prerequisites(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedIntent,
) -> Result<()> {
    super::intent_io::verify_exact(paths, guard, loaded)?;
    let intent = &loaded.value;
    let participant_paths =
        super::participant_paths::ParticipantPaths::new(paths, intent.terminal_id)?;
    super::install_optional::verify(
        paths,
        guard,
        loaded,
        &participant_paths.startup_checkpoint,
        &intent.startup_checkpoint,
    )?;
    super::install_optional::verify(
        paths,
        guard,
        loaded,
        &participant_paths.claude_custody,
        &intent.claude_custody,
    )?;
    super::install_optional::verify(
        paths,
        guard,
        loaded,
        &participant_paths.claude_generation,
        &intent.claude_generation,
    )?;
    super::install_optional::verify(
        paths,
        guard,
        loaded,
        &participant_paths.pty_custody,
        &intent.pty_custody,
    )
}

pub(super) fn verify_runtime(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedIntent,
) -> Result<()> {
    super::intent_io::verify_exact(paths, guard, loaded)?;
    let intent = &loaded.value;
    let participant_paths =
        super::participant_paths::ParticipantPaths::new(paths, intent.terminal_id)?;
    super::install_participant::verify(
        guard,
        &participant_paths.runtime,
        intent,
        RUNTIME_PARTICIPANT,
        &intent.runtime,
    )
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/install_identity_tests.rs"]
mod identity_tests;
