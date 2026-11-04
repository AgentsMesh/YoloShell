use super::constants::*;
use super::guard::SessionDurableMutationGuard;
use super::preparation_schema::*;
use super::schema::{OptionalParticipant, RebootMigrationIntent, RequiredParticipant};
use crate::paths::identity::DurableFileVersion;
use anyhow::{Result, ensure};
use serde::Serialize;
use uuid::Uuid;

pub(super) fn validate_for_guard(
    guard: &SessionDurableMutationGuard<'_>,
    value: &PreparationAuthority,
) -> Result<()> {
    validate_header(value, guard.terminal_id())?;
    ensure!(
        value.session_directory() == guard.session_durable_identity()?,
        SESSION_DIRECTORY_MISMATCH
    );
    guard.verify_session_directory()
}

pub(super) fn validate_header(value: &PreparationAuthority, terminal_id: Uuid) -> Result<()> {
    let schema = match value {
        PreparationAuthority::Active(active) => active.schema_version,
        PreparationAuthority::Settled(settled) => settled.schema_version,
        PreparationAuthority::Retired(retired) => retired.schema_version,
    };
    ensure!(
        schema == REBOOT_PREPARATION_SCHEMA_VERSION,
        PREPARATION_INVALID
    );
    ensure!(
        value.terminal_id() == terminal_id && !terminal_id.is_nil(),
        PREPARATION_INVALID
    );
    if let PreparationAuthority::Active(active) = value {
        ensure!(!active.transaction_id.is_nil(), PREPARATION_INVALID);
        ensure!(
            matches!(
                active.startup_checkpoint,
                ParticipantPreparation::Replace { .. }
            ),
            PREPARATION_INVALID
        );
        ensure!(
            matches!(active.runtime, ParticipantPreparation::Replace { .. }),
            PREPARATION_INVALID
        );
    }
    if let PreparationAuthority::Settled(settled) = value {
        ensure!(!settled.transaction_id.is_nil(), PREPARATION_INVALID);
        ensure!(
            matches!(
                settled.startup_checkpoint,
                SettledParticipant::Installed { .. }
            ) && matches!(settled.runtime, SettledParticipant::Installed { .. }),
            PREPARATION_INVALID
        );
    }
    Ok(())
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(super) fn validate_active(
    guard: &SessionDurableMutationGuard<'_>,
    active: &ActivePreparationAuthority,
) -> Result<()> {
    validate_for_guard(guard, &PreparationAuthority::Active(active.clone()))
}

pub(super) fn require_plan(
    active: &ActivePreparationAuthority,
    intent: &RebootMigrationIntent,
) -> Result<()> {
    plan_matches(&active.startup_checkpoint, &intent.startup_checkpoint)?;
    plan_matches(&active.claude_custody, &intent.claude_custody)?;
    plan_matches(&active.claude_generation, &intent.claude_generation)?;
    plan_matches(&active.pty_custody, &intent.pty_custody)?;
    plan_matches_required(&active.runtime, &intent.runtime)
}

pub(super) fn require_intent_link(
    value: &PreparationAuthority,
    identity: DurableFileVersion,
    intent: &RebootMigrationIntent,
) -> Result<()> {
    match value {
        PreparationAuthority::Active(active) => {
            ensure!(
                identity == intent.preparation_authority_version,
                PREPARATION_CHANGED
            );
            ensure!(
                active.transaction_id == intent.transaction_id,
                PREPARATION_CHANGED
            );
            require_plan(active, intent)
        }
        PreparationAuthority::Settled(settled) => {
            ensure!(
                settled.active_version == intent.preparation_authority_version,
                PREPARATION_CHANGED
            );
            ensure!(
                settled.transaction_id == intent.transaction_id,
                PREPARATION_CHANGED
            );
            settled_matches(&settled.startup_checkpoint, &intent.startup_checkpoint)?;
            settled_matches(&settled.claude_custody, &intent.claude_custody)?;
            settled_matches(&settled.claude_generation, &intent.claude_generation)?;
            settled_matches(&settled.pty_custody, &intent.pty_custody)?;
            settled_matches_required(&settled.runtime, &intent.runtime)
        }
        PreparationAuthority::Retired(_) => Err(anyhow::anyhow!(PREPARATION_CHANGED)),
    }
}

fn settled_matches<T: Serialize>(
    settled: &SettledParticipant,
    value: &OptionalParticipant<T>,
) -> Result<()> {
    match value {
        OptionalParticipant::Absent => ensure!(
            matches!(settled, SettledParticipant::Absent),
            PREPARATION_CHANGED
        ),
        OptionalParticipant::Replace(required) => settled_matches_required(settled, required)?,
    }
    Ok(())
}

fn settled_matches_required<T: Serialize>(
    settled: &SettledParticipant,
    value: &RequiredParticipant<T>,
) -> Result<()> {
    let SettledParticipant::Installed { version, sha256 } = settled else {
        anyhow::bail!(PREPARATION_CHANGED);
    };
    ensure!(
        *version == value.target_version && *sha256 == value.target_sha256,
        PREPARATION_CHANGED
    );
    Ok(())
}

fn plan_matches<T: Serialize>(
    plan: &ParticipantPreparation,
    value: &OptionalParticipant<T>,
) -> Result<()> {
    match value {
        OptionalParticipant::Absent => ensure!(
            matches!(plan, ParticipantPreparation::Absent),
            PREPARATION_CHANGED
        ),
        OptionalParticipant::Replace(required) => plan_matches_required(plan, required)?,
    }
    Ok(())
}

fn plan_matches_required<T: Serialize>(
    plan: &ParticipantPreparation,
    value: &RequiredParticipant<T>,
) -> Result<()> {
    let ParticipantPreparation::Replace {
        source_version,
        source_sha256,
        target_sha256,
    } = plan
    else {
        anyhow::bail!(PREPARATION_CHANGED);
    };
    ensure!(
        *source_version == value.source_version
            && *source_sha256 == value.source_sha256
            && *target_sha256 == value.target_sha256,
        PREPARATION_CHANGED
    );
    Ok(())
}
