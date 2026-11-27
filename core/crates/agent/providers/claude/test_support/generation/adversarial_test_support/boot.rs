use super::super::ClaudeGenerationStore;
use anyhow::{Context, Result, ensure};
use std::path::Path;

const AUTHORITY_MISSING: &str = "Claude authority is missing from reboot test state";
const BOOT_ID_INVALID: &str = "Claude reboot test boot identities are invalid";
const BOOT_ID_MISMATCH: &str = "Claude authority belongs to an unexpected boot";
const NAMESPACE_LABEL: &str = "Claude reboot test";

pub(crate) fn persist_previous_boot(
    binding_path: &Path,
    runtime_epoch: uuid::Uuid,
    current_boot_id: &str,
    previous_boot_id: &str,
) -> Result<()> {
    validate_boots(current_boot_id, previous_boot_id)?;
    let store = ClaudeGenerationStore::from_binding_path(binding_path)?;
    if !store.path.try_exists()? {
        return Ok(());
    }
    let session_directory = binding_path
        .parent()
        .context(super::super::constants::BINDING_PARENT_MISSING)?;
    let lease =
        crate::namespace_lease::NamespaceLease::capture(&[(NAMESPACE_LABEL, session_directory)])?;
    let mut set = store.load_set(&lease)?.context(AUTHORITY_MISSING)?;
    let before = set.clone();
    rewrite_set_boot(&mut set, runtime_epoch, current_boot_id, previous_boot_id)?;
    if set != before {
        store.exchange_set(&lease, &before, &set)?;
    }
    Ok(())
}

pub(crate) fn rewrite_set_boot(
    set: &mut super::super::authority_set::ClaudeInvocationAuthorities,
    runtime_epoch: uuid::Uuid,
    current_boot_id: &str,
    previous_boot_id: &str,
) -> Result<()> {
    let mut matched = false;
    for authority in set
        .authorities
        .iter_mut()
        .filter(|authority| authority.runtime_epoch == runtime_epoch)
    {
        matched = true;
        rewrite_boot(
            &mut authority.handshake_boot_id,
            current_boot_id,
            previous_boot_id,
        )?;
    }
    ensure!(matched, AUTHORITY_MISSING);
    if let Some(transition) = set.pending_binding_transition.as_mut() {
        transition.rewrite_boot_for_test(current_boot_id, previous_boot_id)?;
    }
    set.validate()
}

pub(crate) fn rewrite_boot(value: &mut String, current: &str, previous: &str) -> Result<()> {
    use crate::process::BootIdentityRelation::{Different, Same};
    let current_relation = crate::process::boot_identity_relation(value, current);
    let previous_relation = crate::process::boot_identity_relation(value, previous);
    ensure!(
        current_relation == Same || previous_relation == Same,
        BOOT_ID_MISMATCH
    );
    if current_relation == Same {
        *value = previous.to_owned();
    }
    ensure!(
        previous_relation != Different || current_relation == Same,
        BOOT_ID_MISMATCH
    );
    Ok(())
}

fn validate_boots(current_boot_id: &str, previous_boot_id: &str) -> Result<()> {
    use crate::process::BootIdentityRelation::{Different, Same};
    ensure!(
        crate::process::boot_identity_relation(current_boot_id, current_boot_id) == Same
            && crate::process::boot_identity_relation(previous_boot_id, previous_boot_id) == Same
            && crate::process::boot_identity_relation(current_boot_id, previous_boot_id)
                == Different,
        BOOT_ID_INVALID
    );
    Ok(())
}
