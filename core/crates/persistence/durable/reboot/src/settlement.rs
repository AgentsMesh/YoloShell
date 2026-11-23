use super::guard::SessionDurableMutationGuard;
use super::intent_io::LoadedIntent;
use super::schema::RebootMigrationPhase;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use anyhow::Result;
use uuid::Uuid;

pub fn settle_pending_session(
    paths: &Paths,
    namespace: &NamespaceLease,
    terminal_id: Uuid,
) -> Result<bool> {
    if !pending_work_present(paths, namespace, terminal_id)? {
        return Ok(false);
    }
    let guard = SessionDurableMutationGuard::acquire(paths, namespace, terminal_id)?;
    settle_pending_session_locked(paths, &guard)
}

fn pending_work_present(
    paths: &Paths,
    namespace: &NamespaceLease,
    terminal_id: Uuid,
) -> Result<bool> {
    let session = namespace.directory(&paths.session_dir(terminal_id))?;
    session.verify_path()?;
    let intent_present = !namespace.file_is_missing(&paths.reboot_migration_intent(terminal_id))?;
    session.verify_path()?;
    namespace.verify()?;
    if intent_present {
        return Ok(true);
    }
    super::preparation_authority::pending_without_lock(paths, namespace, terminal_id)
}

pub fn settle_pending_session_locked(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
) -> Result<bool> {
    let Some(mut loaded) = super::intent_io::load(paths, guard, guard.terminal_id())? else {
        return super::preparation_recovery::recover_orphan_if_present(paths, guard);
    };
    super::install::install_prerequisites(paths, guard, &loaded)?;
    super::install::verify_prerequisites(paths, guard, &loaded)?;
    if loaded.value.phase == RebootMigrationPhase::Prepared {
        loaded = super::intent_io::advance(
            paths,
            guard,
            loaded,
            RebootMigrationPhase::PrerequisitesInstalled,
        )?;
    }
    require_phase_at_least(&loaded, RebootMigrationPhase::PrerequisitesInstalled)?;
    super::install::install_runtime(paths, guard, &loaded)?;
    super::install::verify_runtime(paths, guard, &loaded)?;
    if loaded.value.phase == RebootMigrationPhase::PrerequisitesInstalled {
        loaded = super::intent_io::advance(
            paths,
            guard,
            loaded,
            RebootMigrationPhase::RuntimeInstalled,
        )?;
    }
    require_phase_at_least(&loaded, RebootMigrationPhase::RuntimeInstalled)?;
    verify_complete(paths, guard, &loaded)?;
    if loaded.value.phase == RebootMigrationPhase::RuntimeInstalled {
        loaded = super::intent_io::advance(paths, guard, loaded, RebootMigrationPhase::Verified)?;
    }
    require_phase(&loaded, RebootMigrationPhase::Verified)?;
    verify_complete(paths, guard, &loaded)?;
    super::intent_io::verify_exact(paths, guard, &loaded)?;
    super::install_cleanup::cleanup_installed(paths, guard, &loaded)?;
    verify_complete(paths, guard, &loaded)?;
    super::intent_io::verify_exact(paths, guard, &loaded)?;
    let authority = super::preparation_settled::mark(paths, guard, &loaded)?;
    super::intent_io::clear(paths, guard, &loaded)?;
    super::preparation_authority::retire(paths, guard, &authority)?;
    Ok(true)
}

pub fn settle_all_pending(paths: &Paths, namespace: &NamespaceLease) -> Result<usize> {
    let sessions = super::session_discovery::session_ids(paths, namespace)?;
    let mut settled = 0_usize;
    for terminal_id in sessions {
        if settle_pending_session(paths, namespace, terminal_id)? {
            settled = settled
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!(super::constants::SESSION_CAPACITY_EXCEEDED))?;
        }
    }
    Ok(settled)
}

fn verify_complete(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedIntent,
) -> Result<()> {
    super::intent_io::verify_exact(paths, guard, loaded)?;
    super::install::verify_prerequisites(paths, guard, loaded)?;
    super::install::verify_runtime(paths, guard, loaded)?;
    guard.verify_session_directory()
}

fn require_phase(loaded: &LoadedIntent, expected: RebootMigrationPhase) -> Result<()> {
    anyhow::ensure!(
        loaded.value.phase == expected,
        super::constants::INTENT_CHANGED
    );
    Ok(())
}

fn require_phase_at_least(loaded: &LoadedIntent, minimum: RebootMigrationPhase) -> Result<()> {
    anyhow::ensure!(
        phase_rank(loaded.value.phase) >= phase_rank(minimum),
        super::constants::INTENT_CHANGED
    );
    Ok(())
}

const fn phase_rank(phase: RebootMigrationPhase) -> u8 {
    match phase {
        RebootMigrationPhase::Prepared => 0,
        RebootMigrationPhase::PrerequisitesInstalled => 1,
        RebootMigrationPhase::RuntimeInstalled => 2,
        RebootMigrationPhase::Verified => 3,
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/settlement_tests.rs"]
mod tests;
