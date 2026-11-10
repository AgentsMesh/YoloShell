use super::store_support::read_with_identity;
use super::{CodexAuthorityStatus, CodexBindingAuthority, CodexBindingAuthorityStore};
use yolo_agent_authority_contract::AuthorityRuntimeIdentity;
use crate::agent::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use crate::namespace_lease::NamespaceLease;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use uuid::Uuid;

const INVALID_REPAIR_SESSION: &str = "Codex binding repair session is invalid";
const REPAIR_CHANGED: &str = "Codex binding publication repair changed";
const REVOCATION_CHANGED: &str = "Codex binding authority revocation changed";

impl CodexBindingAuthorityStore {
    pub fn repair_active_binding_in_namespace_with_commit_fence(
        &self,
        namespace: &NamespaceLease,
        binding_path: &Path,
        expected_session_id: Uuid,
        expected_runtime: AuthorityRuntimeIdentity,
        validate_binding: impl FnOnce(&AgentBinding) -> Result<()>,
        mut commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        ensure!(!expected_session_id.is_nil(), INVALID_REPAIR_SESSION);
        let Some(authority) = self.load_in_namespace(namespace)? else {
            return Ok(());
        };
        if !repairable_authority(&authority, expected_session_id, expected_runtime) {
            return Ok(());
        }
        authority.verify_resume_evidence()?;
        validate_binding(&authority.binding)?;
        let current = read_with_identity::<AgentBinding>(namespace, binding_path)?;
        if current.as_ref().map(|(value, _)| value) != Some(&authority.binding) {
            if current
                .as_ref()
                .is_some_and(|(value, _)| !repairable_predecessor(value, &authority))
            {
                return Ok(());
            }
            namespace
                .write_json_expected_with_commit_fence(
                    binding_path,
                    current.map(|(_, identity)| identity),
                    &authority.binding,
                    || {
                        commit_fence()?;
                        authority.verify_resume_evidence()?;
                        self.require_exact(namespace, &authority)
                    },
                )
                .context(crate::agent::constants::validation::PERSIST)?;
        }
        self.require_exact(namespace, &authority)?;
        ensure!(
            read_with_identity::<AgentBinding>(namespace, binding_path)?
                .as_ref()
                .map(|(value, _)| value)
                == Some(&authority.binding),
            REPAIR_CHANGED
        );
        Ok(())
    }

    pub fn revoke_exact_in_namespace_with_commit_fence(
        &self,
        namespace: &NamespaceLease,
        expected: &CodexBindingAuthority,
        expected_identity: FileIdentity,
        mut commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        expected.validate()?;
        let revoked = expected.revoked()?;
        let Some((current, identity)) =
            read_with_identity::<CodexBindingAuthority>(namespace, self.path())?
        else {
            namespace.write_json_expected_with_commit_fence(self.path(), None, &revoked, || {
                commit_fence()?;
                ensure!(
                    read_with_identity::<CodexBindingAuthority>(namespace, self.path())?.is_none(),
                    REVOCATION_CHANGED
                );
                Ok(())
            })?;
            return self.require_exact(namespace, &revoked);
        };
        if current == revoked {
            return Ok(());
        }
        ensure!(
            current == *expected && identity.unchanged(expected_identity),
            REVOCATION_CHANGED
        );
        namespace.write_json_expected_with_commit_fence(
            self.path(),
            Some(expected_identity),
            &revoked,
            || {
                commit_fence()?;
                ensure!(
                    self.load_in_namespace(namespace)?.as_ref() == Some(expected),
                    REVOCATION_CHANGED
                );
                Ok(())
            },
        )?;
        self.require_exact(namespace, &revoked)
    }
}

fn repairable_authority(
    authority: &CodexBindingAuthority,
    session_id: Uuid,
    runtime: AuthorityRuntimeIdentity,
) -> bool {
    authority.status == CodexAuthorityStatus::Active
        && authority.binding.terminal_id == session_id
        && authority.matches_runtime(runtime)
}

fn repairable_predecessor(current: &AgentBinding, authority: &CodexBindingAuthority) -> bool {
    let target = &authority.binding;
    let recoverable = current.state.is_active()
        && current.recovery_availability == AgentRecoveryAvailability::Available;
    let unavailable = current.state == AgentState::Unknown
        && current.recovery_availability == AgentRecoveryAvailability::Unavailable;
    (recoverable || unavailable)
        && authority.binding_matches_provider_process(current)
        && current.schema_version == target.schema_version
        && current.terminal_id == target.terminal_id
        && current.kind == AgentKind::Codex
        && target.kind == AgentKind::Codex
        && current.provider_session_id == target.provider_session_id
        && current.executable == target.executable
        && current.cwd == target.cwd
        && current.evidence_path == target.evidence_path
        && current.data_home == target.data_home
        && current.detected_at_ms == target.detected_at_ms
        && current.updated_at_ms <= target.updated_at_ms
}
