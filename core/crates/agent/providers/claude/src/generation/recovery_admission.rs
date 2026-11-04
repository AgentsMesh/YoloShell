use super::{ClaudeGenerationAuthority, ClaudeGenerationStore};
use crate::namespace_lease::NamespaceLease;
use anyhow::{Result, ensure};

const PUBLISHED_PREDECESSOR: &str =
    "parked Claude recovery retains the published binding predecessor";

pub(super) fn require_unreferenced(
    store: &ClaudeGenerationStore,
    namespace: &NamespaceLease,
    authority: &ClaudeGenerationAuthority,
) -> Result<()> {
    let binding = super::durable_binding_io::read_binding(namespace, &store.binding_path()?)?;
    ensure!(
        !binding
            .as_ref()
            .is_some_and(|binding| authority.matches_binding(binding)),
        PUBLISHED_PREDECESSOR
    );
    Ok(())
}

pub(super) fn process_gone(authority: &ClaudeGenerationAuthority) -> bool {
    if crate::process::boot_identity_relation(
        &authority.handshake_boot_id,
        &crate::process::boot_id(),
    ) == crate::process::BootIdentityRelation::Different
    {
        return true;
    }
    crate::process::process_identity_relation(authority.process.pid, authority.process.start_time)
        == crate::process::ProcessIdentityRelation::Gone
}
