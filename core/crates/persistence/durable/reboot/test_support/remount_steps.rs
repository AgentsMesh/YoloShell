use super::guard::SessionDurableMutationGuard;
use super::remount_protocol::Phase;
use super::schema::RebootMigrationPhase;
use crate::paths::Paths;
use anyhow::{Result, ensure};

const MISSING_INTENT: &str = "remount handoff lost its migration intent";
const INVALID_PHASE: &str = "remount handoff has an unexpected durable migration phase";

pub(super) fn run(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    phase: Phase,
) -> Result<()> {
    match phase {
        Phase::Prepare => anyhow::bail!(INVALID_PHASE),
        Phase::Finish => {
            ensure!(
                super::settle_pending_session_locked(paths, guard)?,
                INVALID_PHASE
            );
            return Ok(());
        }
        Phase::Verify => {
            ensure!(
                !super::settle_pending_session_locked(paths, guard)?,
                INVALID_PHASE
            );
            return Ok(());
        }
        _ => {}
    }
    let loaded = super::intent_io::load(paths, guard, guard.terminal_id())?
        .ok_or_else(|| anyhow::anyhow!(MISSING_INTENT))?;
    match phase {
        Phase::Prerequisites => {
            ensure!(
                loaded.value.phase == RebootMigrationPhase::Prepared,
                INVALID_PHASE
            );
            super::install::install_prerequisites(paths, guard, &loaded)?;
            super::intent_io::advance(
                paths,
                guard,
                loaded,
                RebootMigrationPhase::PrerequisitesInstalled,
            )?;
        }
        Phase::Exchange => {
            ensure!(
                loaded.value.phase == RebootMigrationPhase::PrerequisitesInstalled,
                INVALID_PHASE
            );
            let required = &loaded.value.runtime;
            super::staging::install(
                guard,
                &paths.runtime_record(guard.terminal_id()),
                loaded.value.transaction_id,
                super::constants::RUNTIME_PARTICIPANT,
                required.source_version,
                required.target_version,
                &required.target_payload,
                || super::intent_io::verify_exact(paths, guard, &loaded),
            )?;
        }
        Phase::Runtime => {
            ensure!(
                loaded.value.phase == RebootMigrationPhase::PrerequisitesInstalled,
                INVALID_PHASE
            );
            super::install::install_runtime(paths, guard, &loaded)?;
            super::intent_io::advance(
                paths,
                guard,
                loaded,
                RebootMigrationPhase::RuntimeInstalled,
            )?;
        }
        Phase::Settled => {
            ensure!(
                loaded.value.phase == RebootMigrationPhase::RuntimeInstalled,
                INVALID_PHASE
            );
            super::install::verify_prerequisites(paths, guard, &loaded)?;
            super::install::verify_runtime(paths, guard, &loaded)?;
            let loaded =
                super::intent_io::advance(paths, guard, loaded, RebootMigrationPhase::Verified)?;
            super::install_cleanup::cleanup_installed(paths, guard, &loaded)?;
            super::preparation_settled::mark(paths, guard, &loaded)?;
        }
        _ => anyhow::bail!(INVALID_PHASE),
    }
    Ok(())
}
