use super::store_support::{AUTHORITY_CHANGED, missing, read_with_identity, validate_progression};
use super::{CodexAuthorityStatus, CodexBindingAuthority};
use crate::namespace_lease::NamespaceLease;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

const BINDING_PARENT_MISSING: &str = "Codex authority binding has no session directory";
const AUTHORITY_READ_CONTEXT: &str = "read Codex binding authority";
const AUTHORITY_WRITE_CONTEXT: &str = "persist Codex binding authority";
#[derive(Clone, Debug)]
pub struct CodexBindingAuthorityStore {
    path: PathBuf,
}

impl CodexBindingAuthorityStore {
    pub fn from_binding_path(binding_path: &Path) -> Result<Self> {
        let parent = binding_path.parent().context(BINDING_PARENT_MISSING)?;
        Ok(Self {
            path: parent.join(super::AUTHORITY_FILE_NAME),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_in_namespace(
        &self,
        namespace: &NamespaceLease,
    ) -> Result<Option<CodexBindingAuthority>> {
        match namespace.read_json(&self.path) {
            Ok(authority) => {
                let authority: CodexBindingAuthority = authority;
                authority.validate()?;
                Ok(Some(authority))
            }
            Err(error) if missing(&error) => Ok(None),
            Err(error) => Err(error).context(AUTHORITY_READ_CONTEXT),
        }
    }

    pub fn publish_in_namespace_with_commit_fence(
        &self,
        namespace: &NamespaceLease,
        binding_path: &Path,
        authority: &CodexBindingAuthority,
        mut commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        authority.validate()?;
        if authority.status == CodexAuthorityStatus::Active {
            authority.verify_resume_evidence()?;
        }
        let current = read_with_identity::<CodexBindingAuthority>(namespace, &self.path)?;
        if current.as_ref().map(|(value, _)| value) != Some(authority) {
            validate_progression(current.as_ref().map(|(value, _)| value), authority)?;
            namespace
                .write_json_expected_with_commit_fence(
                    &self.path,
                    current.map(|(_, identity)| identity),
                    authority,
                    || {
                        if authority.status == CodexAuthorityStatus::Active {
                            authority.verify_resume_evidence()?;
                        }
                        commit_fence()
                    },
                )
                .context(AUTHORITY_WRITE_CONTEXT)?;
        }
        self.publish_binding(namespace, binding_path, authority, &mut commit_fence)?;
        self.require_exact(namespace, authority)
    }

    pub fn require_exact(
        &self,
        namespace: &NamespaceLease,
        expected: &CodexBindingAuthority,
    ) -> Result<()> {
        ensure!(
            self.load_in_namespace(namespace)?.as_ref() == Some(expected),
            AUTHORITY_CHANGED
        );
        Ok(())
    }

    pub fn invalidate_in_namespace_with_commit_fence(
        &self,
        namespace: &NamespaceLease,
        binding_path: &Path,
        runtime: yolo_agent_authority_contract::AuthorityRuntimeIdentity,
        commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<Option<CodexBindingAuthority>> {
        let Some((current, identity)) =
            read_with_identity::<CodexBindingAuthority>(namespace, &self.path)?
        else {
            return Ok(None);
        };
        current.validate()?;
        self.invalidate_loaded_in_namespace_with_commit_fence(
            namespace,
            binding_path,
            current,
            identity,
            runtime,
            commit_fence,
        )
    }

    pub(super) fn invalidate_loaded_in_namespace_with_commit_fence(
        &self,
        namespace: &NamespaceLease,
        binding_path: &Path,
        current: CodexBindingAuthority,
        identity: FileIdentity,
        runtime: yolo_agent_authority_contract::AuthorityRuntimeIdentity,
        mut commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<Option<CodexBindingAuthority>> {
        if !current.matches_runtime(runtime) {
            return Ok(None);
        }
        let terminal = match current.status {
            CodexAuthorityStatus::Active
                if current.binding.state == crate::agent::AgentState::Exited =>
            {
                current
            }
            CodexAuthorityStatus::Active => {
                let unavailable = current.unavailable()?;
                namespace
                    .write_json_expected_with_commit_fence(
                        &self.path,
                        Some(identity),
                        &unavailable,
                        &mut commit_fence,
                    )
                    .context(AUTHORITY_WRITE_CONTEXT)?;
                unavailable
            }
            CodexAuthorityStatus::Exhausted
            | CodexAuthorityStatus::Unavailable
            | CodexAuthorityStatus::Revoked => current,
        };
        self.publish_binding(namespace, binding_path, &terminal, &mut commit_fence)?;
        self.require_exact(namespace, &terminal)?;
        Ok(Some(terminal))
    }

    fn publish_binding(
        &self,
        namespace: &NamespaceLease,
        binding_path: &Path,
        authority: &CodexBindingAuthority,
        commit_fence: &mut dyn FnMut() -> Result<()>,
    ) -> Result<()> {
        let binding = read_with_identity::<crate::agent::AgentBinding>(namespace, binding_path)?;
        if binding.as_ref().map(|(value, _)| value) != Some(&authority.binding) {
            namespace
                .write_json_expected_with_commit_fence(
                    binding_path,
                    binding.map(|(_, identity)| identity),
                    &authority.binding,
                    || {
                        commit_fence()?;
                        if authority.status == CodexAuthorityStatus::Active {
                            authority.verify_resume_evidence()?;
                        }
                        self.require_exact(namespace, authority)
                    },
                )
                .context(crate::agent::constants::validation::PERSIST)?;
        }
        Ok(())
    }
}
