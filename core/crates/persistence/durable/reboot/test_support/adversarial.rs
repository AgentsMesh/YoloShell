use super::SessionDurableMutationGuard;
use crate::model::RuntimeRecord;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use anyhow::{Context, Result, ensure};

const BOOT_ID_INVALID: &str = "reboot test boot identities are invalid";
const LIFECYCLE_BUSY: &str = "reboot test session lifecycle lock is busy";
const RUNTIME_CHANGED: &str = "reboot test runtime record changed";
const SESSION_NAMESPACE_LABEL: &str = "durable reboot test";

pub fn persist_previous_boot(
    paths: &Paths,
    runtime: &RuntimeRecord,
    target_boot_id: &str,
) -> Result<()> {
    validate_boots(&runtime.boot_id, target_boot_id)?;
    let _lifecycle = paths
        .try_acquire_session_lifecycle_lock(runtime.terminal_id)?
        .context(LIFECYCLE_BUSY)?;
    let session = paths.session_dir(runtime.terminal_id);
    let namespace = NamespaceLease::capture(&[(SESSION_NAMESPACE_LABEL, &session)])?;
    let guard = SessionDurableMutationGuard::acquire_after_lifecycle_lock(
        paths,
        &namespace,
        runtime.terminal_id,
    )?;
    if super::settle_pending_session_locked(paths, &guard)? {
        return verify_completed(paths, &guard, runtime, target_boot_id);
    }
    if runtime_is_target(paths, &guard, runtime, target_boot_id)? {
        return verify_completed(paths, &guard, runtime, target_boot_id);
    }
    let targets = super::adversarial_targets::capture(paths, &guard, runtime, target_boot_id)?;
    super::prepare_and_publish_intent(paths, &guard, targets)?;
    super::settle_pending_session_locked(paths, &guard)?;
    verify_completed(paths, &guard, runtime, target_boot_id)
}

fn runtime_is_target(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    source: &RuntimeRecord,
    target: &str,
) -> Result<bool> {
    let current: RuntimeRecord = guard
        .namespace()
        .read_json(&paths.runtime_record(source.terminal_id))?;
    let mut expected = source.clone();
    expected.boot_id = target.to_owned();
    ensure!(current == *source || current == expected, RUNTIME_CHANGED);
    Ok(current == expected)
}

fn verify_completed(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    source: &RuntimeRecord,
    target: &str,
) -> Result<()> {
    ensure!(
        runtime_is_target(paths, guard, source, target)?,
        RUNTIME_CHANGED
    );
    let expected = completed_targets(paths, guard, source, target)?;
    verify_typed(
        guard,
        &paths.worker_startup_checkpoint(source.terminal_id),
        Some(expected.startup_checkpoint),
    )?;
    let participant_paths =
        super::participant_paths::ParticipantPaths::new(paths, source.terminal_id)?;
    verify_typed(
        guard,
        &participant_paths.claude_custody,
        expected.claude_custody,
    )?;
    verify_typed(
        guard,
        &participant_paths.claude_generation,
        expected.claude_generation,
    )?;
    verify_typed(guard, &participant_paths.pty_custody, expected.pty_custody)
}

fn completed_targets(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    source: &RuntimeRecord,
    target: &str,
) -> Result<super::ParticipantTargets> {
    let mut rewrite_source = source.clone();
    rewrite_source.boot_id = target.to_owned();
    super::adversarial_targets::capture(paths, guard, &rewrite_source, target)
}

fn verify_typed<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &std::path::Path,
    expected: Option<T>,
) -> Result<()>
where
    T: serde::de::DeserializeOwned + PartialEq,
{
    match expected {
        Some(expected) => ensure!(
            guard.namespace().read_json::<T>(path)? == expected,
            RUNTIME_CHANGED
        ),
        None => ensure!(guard.namespace().file_is_missing(path)?, RUNTIME_CHANGED),
    }
    Ok(())
}

fn validate_boots(current: &str, target: &str) -> Result<()> {
    use crate::process::BootIdentityRelation::{Different, Same};
    ensure!(
        crate::process::boot_identity_relation(current, current) == Same
            && crate::process::boot_identity_relation(target, target) == Same
            && crate::process::boot_identity_relation(current, target) == Different,
        BOOT_ID_INVALID
    );
    Ok(())
}
