use super::native_authority::{self, INVALID, NativeAuthority};
use super::{AgentBinding, AgentBindingStore, DetectionContext};
use crate::agent::claude::generation::AuthorityRuntimeIdentity;
use crate::namespace_lease::NamespaceLease;
use crate::paths::identity::FileIdentity;
use anyhow::{Result, ensure};

pub(super) struct Snapshot {
    pub(super) authority: NativeAuthority,
    pub(super) binding: AgentBinding,
    pub(super) raw: Option<(AgentBinding, FileIdentity)>,
}

pub(super) fn read(
    store: &AgentBindingStore,
    namespace: &NamespaceLease,
    context: &DetectionContext,
    runtime: AuthorityRuntimeIdentity,
    after_snapshot: impl FnOnce() -> Result<()>,
) -> Result<Option<Snapshot>> {
    let raw = store.read_raw_with_identity_in_namespace(namespace)?;
    let Some(authority) = native_authority::select(
        store,
        namespace,
        raw.as_ref().map(|(binding, _)| binding),
        runtime,
    )?
    else {
        return Ok(None);
    };
    after_snapshot()?;
    let binding = authority.binding(runtime)?;
    if authority.transcript_absent() {
        store
            .registry()
            .validate_authority_binding(&binding, context)?;
    } else {
        store.registry().validate_binding(&binding, context)?;
    }
    let snapshot = Snapshot {
        authority,
        binding,
        raw,
    };
    snapshot.verify(store, namespace, runtime)?;
    Ok(Some(snapshot))
}

impl Snapshot {
    pub(super) fn verify(
        &self,
        store: &AgentBindingStore,
        namespace: &NamespaceLease,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<()> {
        self.verify_source(store, namespace, runtime)?;
        ensure!(
            store.read_raw_with_identity_in_namespace(namespace)? == self.raw,
            INVALID
        );
        Ok(())
    }

    pub(super) fn verify_source(
        &self,
        store: &AgentBindingStore,
        namespace: &NamespaceLease,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<()> {
        self.authority.verify(store, namespace)?;
        ensure!(self.authority.binding(runtime)? == self.binding, INVALID);
        Ok(())
    }
}

impl AgentBindingStore {
    pub fn repair_native_binding_in_namespace_with_commit_fence(
        &self,
        namespace: &NamespaceLease,
        context: &DetectionContext,
        runtime: AuthorityRuntimeIdentity,
        mut commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<Option<AgentBinding>> {
        let Some(snapshot) = read(self, namespace, context, runtime, || Ok(()))? else {
            return Ok(None);
        };
        if snapshot.raw.as_ref().map(|(binding, _)| binding) != Some(&snapshot.binding) {
            self.persist_reconciled_in_namespace(
                namespace,
                snapshot.raw.as_ref().map(|(_, identity)| *identity),
                &snapshot.binding,
                || {
                    commit_fence()?;
                    // The binding writer owns target CAS and its lock; verify only the source here.
                    snapshot.verify_source(self, namespace, runtime)
                },
            )?;
        }
        snapshot.authority.verify(self, namespace)?;
        ensure!(
            self.read_raw_in_namespace(namespace)?.as_ref() == Some(&snapshot.binding),
            INVALID
        );
        Ok(Some(snapshot.binding))
    }

    pub fn authorize_native_resume_in_namespace(
        &self,
        namespace: &NamespaceLease,
        context: &DetectionContext,
        runtime: AuthorityRuntimeIdentity,
        expected: &AgentBinding,
    ) -> Result<bool> {
        let Some(snapshot) = read(self, namespace, context, runtime, || Ok(()))? else {
            // A revoked Codex authority is a durable deny decision.  It is
            // deliberately excluded from recovery selection so repair cannot
            // recreate a cleared binding, but resume must still fail closed.
            if self.revoked_codex_authority_matches(namespace, runtime)? {
                anyhow::bail!(INVALID);
            }
            return Ok(false);
        };
        ensure!(
            snapshot.authority.matches_subject(expected)
                && snapshot.binding.state == expected.state
                && expected.recovery_availability
                    == crate::agent::AgentRecoveryAvailability::Available
                && snapshot.binding.recovery_availability
                    == crate::agent::AgentRecoveryAvailability::Available
                && snapshot.authority.process_relation()?
                    == crate::process::ProcessIdentityRelation::Gone,
            INVALID
        );
        snapshot.verify(self, namespace, runtime)?;
        Ok(true)
    }

    fn revoked_codex_authority_matches(
        &self,
        namespace: &NamespaceLease,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<bool> {
        let codex = self.codex_authority_store()?;
        if namespace.file_is_missing(codex.path())? {
            return Ok(false);
        }
        let (record, _) = namespace
            .read_json_with_identity::<crate::agent::codex::authority::CodexBindingAuthority>(
                codex.path(),
            )?;
        Ok(
            record.status == crate::agent::codex::authority::CodexAuthorityStatus::Revoked
                && record.matches_runtime(runtime),
        )
    }
}
