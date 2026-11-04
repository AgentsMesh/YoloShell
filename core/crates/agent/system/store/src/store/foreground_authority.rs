use super::*;
use crate::agent::ObservedProcess;
use crate::agent::claude::generation::{AuthorityRuntimeIdentity, ClaudeGenerationStore};
use anyhow::{Context, ensure};

const FOREGROUND_TRANSCRIPT_MISSING: &str = "foreground Claude authority has no transcript path";
const FOREGROUND_EVIDENCE_MISSING: &str = "foreground Claude authoritative evidence is unavailable";
const FOREGROUND_EVIDENCE_CHANGED: &str = "foreground Claude authoritative evidence changed";

impl AgentBindingStore {
    pub fn reactivate_claude_foreground_in_namespace<F>(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        context: &DetectionContext,
        runtime: AuthorityRuntimeIdentity,
        processes: &[ObservedProcess],
        mut observe: F,
    ) -> Result<Option<AgentBinding>>
    where
        F: FnMut() -> Result<Vec<ObservedProcess>>,
    {
        if super::native_authority::claude_record_exists(self, namespace)? {
            return Ok(None);
        }
        let generations = ClaudeGenerationStore::from_binding_path(&self.path)?;
        let Some((selected, active)) =
            generations.foreground_invocation_in_namespace(namespace, runtime, processes)?
        else {
            return Ok(None);
        };
        let previous = self.read_raw_in_namespace(namespace)?;
        if active
            && previous
                .as_ref()
                .is_some_and(|binding| selected.matches_binding(binding))
        {
            return Ok(None);
        }
        let transcript = selected
            .transcript_path
            .as_deref()
            .context(FOREGROUND_TRANSCRIPT_MISSING)?;
        let fence = crate::agent::claude::ClaudeAdapter
            .inspect_authoritative_evidence_with_hook(
                transcript,
                &selected.cwd,
                &selected.data_home,
                || {},
            )?
            .context(FOREGROUND_EVIDENCE_MISSING)?;
        verify_fence(&selected, &fence)?;
        verify_foreground(&generations, namespace, runtime, &selected, &observe()?)?;
        let authority =
            generations.activate_observed_in_namespace(namespace, &selected, selected.state)?;
        let authentication = authority.authentication();
        verify_fence(&authority, &fence)?;
        verify_foreground(&generations, namespace, runtime, &authority, &observe()?)?;
        let registry = self.registry();
        let (committed, binding) = generations.publish_binding_state_in_namespace(
            namespace,
            self.path(),
            authentication,
            &authority,
            true,
            |binding| registry.validate_binding(binding, context),
        )?;
        verify_fence(&committed, &fence)?;
        verify_foreground(&generations, namespace, runtime, &committed, &observe()?)?;
        ensure!(
            committed.matches_binding(&binding),
            FOREGROUND_EVIDENCE_CHANGED
        );
        Ok(Some(binding))
    }
}

fn verify_foreground(
    generations: &ClaudeGenerationStore,
    namespace: &crate::namespace_lease::NamespaceLease,
    runtime: AuthorityRuntimeIdentity,
    expected: &crate::agent::claude::generation::ClaudeGenerationAuthority,
    processes: &[ObservedProcess],
) -> Result<()> {
    let selected = generations
        .foreground_invocation_in_namespace(namespace, runtime, processes)?
        .context(FOREGROUND_EVIDENCE_CHANGED)?
        .0;
    ensure!(
        same_invocation(&selected, expected),
        FOREGROUND_EVIDENCE_CHANGED
    );
    Ok(())
}

fn same_invocation(
    left: &crate::agent::claude::generation::ClaudeGenerationAuthority,
    right: &crate::agent::claude::generation::ClaudeGenerationAuthority,
) -> bool {
    left.terminal_id == right.terminal_id
        && left.runtime_epoch == right.runtime_epoch
        && left.invocation_id == right.invocation_id
        && left.worker == right.worker
        && left.process == right.process
        && left.provider_session_id == right.provider_session_id
        && left.cwd == right.cwd
        && left.data_home == right.data_home
        && left.transcript_path == right.transcript_path
        && left.transcript_identity == right.transcript_identity
}

fn verify_fence(
    authority: &crate::agent::claude::generation::ClaudeGenerationAuthority,
    fence: &crate::agent::claude::AuthoritativeClaudeEvidence,
) -> Result<()> {
    let evidence = fence.evidence();
    ensure!(
        fence.verify()?
            && authority.transcript_identity == Some(fence.identity())
            && authority.transcript_path.as_deref() == Some(evidence.path.as_path())
            && authority.provider_session_id == evidence.provider_session_id
            && authority.cwd == evidence.cwd
            && authority.data_home == evidence.data_home,
        FOREGROUND_EVIDENCE_CHANGED
    );
    Ok(())
}
