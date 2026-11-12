use super::ClaudeGenerationStore;
use super::authority_set::ClaudeInvocationAuthorities;
use super::constants::*;
use crate::agent::AgentBinding;
use crate::namespace_lease::NamespaceLease;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};
use std::path::Path;

pub(super) fn write_binding(
    mutation: &super::mutation::GenerationMutation<'_>,
    namespace: &NamespaceLease,
    binding_path: &Path,
    expected: &ClaudeInvocationAuthorities,
    existing: Option<&AgentBinding>,
    binding_identity: Option<FileIdentity>,
    observed_admission: Option<&super::HookPublicationAdmission>,
    target: &AgentBinding,
) -> Result<()> {
    let store = ClaudeGenerationStore::from_binding_path(binding_path)?;
    let current = store.load_set(namespace)?.context(AUTHORITY_MISSING)?;
    ensure!(&current == expected, AUTHORITY_SNAPSHOT_MISMATCH);
    ensure!(
        accepts_existing(expected, existing)
            || observed_admission
                .map(|admission| admission.accepts(existing))
                .transpose()?
                .unwrap_or(false)
            || accepts_initial(expected, existing, binding_identity),
        DURABLE_BINDING_INVALID
    );
    mutation.write_path(namespace, binding_path, binding_identity, target, || {
        let set = store.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        ensure!(&set == expected, AUTHORITY_SNAPSHOT_MISMATCH);
        if let Some(admission) = observed_admission {
            admission.verify_captured()?;
        }
        Ok(())
    })
}

fn accepts_existing(
    expected: &ClaudeInvocationAuthorities,
    existing: Option<&AgentBinding>,
) -> bool {
    let Some(binding) = existing else {
        return false;
    };
    if binding.recovery_availability == crate::agent::AgentRecoveryAvailability::UnmanagedLaunch {
        return false;
    }
    if let Some(transition) = expected.pending_binding_transition.as_ref() {
        return transition.accepts_binding(binding);
    }
    expected.active().is_ok_and(|active| {
        active.binding_subject_matches(binding)
            || expected
                .authorities
                .iter()
                .any(|authority| authority.matches_binding(binding))
    })
}

fn accepts_initial(
    expected: &ClaudeInvocationAuthorities,
    existing: Option<&AgentBinding>,
    identity: Option<FileIdentity>,
) -> bool {
    existing.is_none()
        && identity.is_none()
        && expected.pending_binding_transition.is_none()
        && expected.binding_publication.is_none()
        && expected
            .active()
            .is_ok_and(|authority| authority.binding_revision.is_none())
}

pub(super) fn read_binding(
    namespace: &NamespaceLease,
    path: &Path,
) -> Result<Option<AgentBinding>> {
    yolo_agent_binding_io::AgentBindingFile::new(path).read_in_namespace(namespace)
}

pub(super) fn read_binding_with_identity(
    namespace: &NamespaceLease,
    path: &Path,
) -> Result<(Option<AgentBinding>, Option<FileIdentity>)> {
    if namespace.file_is_missing(path)? {
        return Ok((None, None));
    }
    namespace
        .read_json_with_identity(path)
        .map(|(binding, identity)| (Some(binding), Some(identity)))
}
