use super::*;
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, ensure};

#[path = "native_fixture_boot.rs"]
mod boot;
pub use boot::persist_native_previous_boot;

const INVALID: &str = "Codex native recovery fixture does not match retained authority";
const NAMESPACE_LABEL: &str = "Codex native recovery fixture";

pub fn persist_native_durable_state(binding_path: &Path, state: AgentState) -> Result<()> {
    persist(binding_path, |current| {
        ensure!(
            current.resume_boot_id().is_some()
                && current.status != crate::codex::authority::CodexAuthorityStatus::Revoked,
            INVALID
        );
        if state == AgentState::Unknown {
            current.unavailable()
        } else {
            current.with_adversarial_active_state(state)
        }
    })
}

pub fn persist_native_revoked(binding_path: &Path) -> Result<()> {
    persist(binding_path, CodexBindingAuthority::revoked)
}

fn persist(
    binding_path: &Path,
    transform: impl FnOnce(&CodexBindingAuthority) -> Result<CodexBindingAuthority>,
) -> Result<()> {
    let (namespace, store, current, identity) = capture(binding_path)?;
    let target = transform(&current)?;
    namespace.write_json_expected_with_commit_fence(
        store.path(),
        Some(identity),
        &target,
        || current.verify_resume_evidence(),
    )?;
    let binding_identity = namespace
        .read_json_with_identity::<crate::agent::AgentBinding>(binding_path)?
        .1;
    namespace.write_json_expected_with_commit_fence(
        binding_path,
        Some(binding_identity),
        &target.binding,
        || {
            ensure!(
                store.load_in_namespace(&namespace)?.as_ref() == Some(&target),
                INVALID
            );
            target.verify_resume_evidence()
        },
    )?;
    Ok(())
}

fn capture(
    path: &Path,
) -> Result<(
    NamespaceLease,
    CodexBindingAuthorityStore,
    CodexBindingAuthority,
    crate::paths::identity::FileIdentity,
)> {
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, path.parent().context(INVALID)?)])?;
    let store = CodexBindingAuthorityStore::from_binding_path(path)?;
    let (current, identity): (CodexBindingAuthority, _) =
        namespace.read_json_with_identity(store.path())?;
    current.validate()?;
    current.verify_resume_evidence()?;
    Ok((namespace, store, current, identity))
}
