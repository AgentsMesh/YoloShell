use crate::agent::AgentState;
use crate::claude::generation::native::{ClaudeNativeGeneration, ClaudeNativeGenerationStore};
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[path = "native_state.rs"]
mod state;

const INVALID: &str = "Claude native recovery fixture does not match retained authority";
const NAMESPACE_LABEL: &str = "Claude native recovery fixture";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NativeDocument {
    schema_version: u32,
    #[serde(with = "crate::serde_uuid")]
    active_invocation: uuid::Uuid,
    invocations: Vec<ClaudeNativeGeneration>,
}

pub fn persist_native_durable_state(binding_path: &Path, target: AgentState) -> Result<()> {
    let (namespace, store, current) = capture(binding_path)?;
    state::persist(&namespace, &store, &current, target)
}

pub fn persist_native_revoked(binding_path: &Path) -> Result<()> {
    let (namespace, store, current) = capture(binding_path)?;
    let revoked = store.revoke_in_namespace(&namespace, &current, || require_gone(&current))?;
    store.publish_binding_in_namespace(&namespace, &revoked, || require_gone(&current))
}

pub fn persist_native_previous_boot(
    binding_path: &Path,
    runtime_epoch: uuid::Uuid,
    current_boot: &str,
    previous_boot: &str,
) -> Result<()> {
    use crate::process::BootIdentityRelation::{Different, Same};
    ensure!(
        crate::process::boot_identity_relation(current_boot, current_boot) == Same
            && crate::process::boot_identity_relation(previous_boot, previous_boot) == Same
            && crate::process::boot_identity_relation(current_boot, previous_boot) == Different,
        INVALID
    );
    let (namespace, store, current) = capture(binding_path)?;
    ensure!(current.scope.runtime_epoch == runtime_epoch, INVALID);
    let generation = super::super::ClaudeGenerationStore::from_binding_path(binding_path)?;
    let mutation = super::super::mutation::GenerationMutation::acquire(&generation, &namespace)?;
    let (mut document, identity): (NativeDocument, _) =
        namespace.read_json_with_identity(store.path())?;
    ensure!(
        document.active_invocation == current.invocation_id
            && document.invocations.iter().any(|record| record == &current),
        INVALID
    );
    let mut matched = false;
    for record in document
        .invocations
        .iter_mut()
        .filter(|record| record.scope.runtime_epoch == runtime_epoch)
    {
        require_gone(record)?;
        ensure!(
            record.boot_id == current_boot || record.boot_id == previous_boot,
            INVALID
        );
        record.boot_id = previous_boot.to_owned();
        matched = true;
    }
    ensure!(matched, INVALID);
    let mut expected = current.clone();
    expected.boot_id = previous_boot.to_owned();
    mutation.write_path(&namespace, store.path(), Some(identity), &document, || {
        require_gone(&current)
    })?;
    let updated = store.load_in_namespace(&namespace)?.context(INVALID)?;
    ensure!(
        updated == expected && (updated.revoked || updated.verify_resume_evidence()?),
        INVALID
    );
    Ok(())
}

fn capture(
    path: &Path,
) -> Result<(
    NamespaceLease,
    ClaudeNativeGenerationStore,
    ClaudeNativeGeneration,
)> {
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, path.parent().context(INVALID)?)])?;
    let store = ClaudeNativeGenerationStore::from_binding_path(path)?;
    let current = store.load_in_namespace(&namespace)?.context(INVALID)?;
    require_gone(&current)?;
    ensure!(
        current.revoked || current.verify_resume_evidence()?,
        INVALID
    );
    Ok((namespace, store, current))
}

fn require_gone(record: &ClaudeNativeGeneration) -> Result<()> {
    ensure!(
        crate::process::process_identity_relation(
            record.scope.agent.pid,
            record.scope.agent.start_time
        ) == crate::process::ProcessIdentityRelation::Gone,
        INVALID
    );
    Ok(())
}
