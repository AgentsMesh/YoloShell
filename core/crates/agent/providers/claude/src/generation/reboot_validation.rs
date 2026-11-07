use super::authority_set::ClaudeInvocationAuthorities;
use super::{AuthorityProcessIdentity, ClaudeGenerationAuthority};
use anyhow::{Result, ensure};
use uuid::Uuid;

const BOOT_ID_INVALID: &str = "Claude authority reboot identity is invalid";
const RUNTIME_IDENTITY_INVALID: &str = "Claude authority reboot runtime identity is invalid";

pub fn source_before_reboot_migration(
    target: &ClaudeInvocationAuthorities,
    session_id: Uuid,
    runtime_epoch: Uuid,
    worker: AuthorityProcessIdentity,
    source_boot_id: &str,
    target_boot_id: &str,
) -> Result<ClaudeInvocationAuthorities> {
    target.validate()?;
    ensure!(
        target.terminal_id == session_id,
        RUNTIME_IDENTITY_INVALID
    );
    let active = target.active()?;
    ensure!(
        matches_runtime(active, runtime_epoch, worker),
        RUNTIME_IDENTITY_INVALID
    );
    let mut source = target.clone();
    let mut migrated = false;
    for authority in &mut source.authorities {
        if authority.runtime_epoch == runtime_epoch {
            ensure!(authority.worker == worker, RUNTIME_IDENTITY_INVALID);
            restore_boot(
                &mut authority.handshake_boot_id,
                source_boot_id,
                target_boot_id,
            )?;
            migrated = true;
        } else {
            require_unrelated_boot(&authority.handshake_boot_id, target_boot_id)?;
        }
    }
    if let Some(transition) = source.pending_binding_transition.as_mut() {
        let predecessor = &mut transition.predecessor_authority;
        ensure!(
            matches_runtime(predecessor, runtime_epoch, worker),
            RUNTIME_IDENTITY_INVALID
        );
        restore_boot(
            &mut predecessor.handshake_boot_id,
            source_boot_id,
            target_boot_id,
        )?;
    }
    ensure!(migrated, RUNTIME_IDENTITY_INVALID);
    source.validate()?;
    Ok(source)
}

fn matches_runtime(
    authority: &ClaudeGenerationAuthority,
    runtime_epoch: Uuid,
    worker: AuthorityProcessIdentity,
) -> bool {
    authority.runtime_epoch == runtime_epoch && authority.worker == worker
}

fn restore_boot(value: &mut String, source: &str, target: &str) -> Result<()> {
    use crate::process::BootIdentityRelation::Same;
    ensure!(
        crate::process::boot_identity_relation(value, target) == Same,
        BOOT_ID_INVALID
    );
    *value = source.to_owned();
    Ok(())
}

fn require_unrelated_boot(value: &str, target: &str) -> Result<()> {
    use crate::process::BootIdentityRelation::{Different, Same};
    ensure!(
        crate::process::boot_identity_relation(value, value) == Same
            && crate::process::boot_identity_relation(value, target) == Different,
        BOOT_ID_INVALID
    );
    Ok(())
}
