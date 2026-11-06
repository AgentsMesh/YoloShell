use super::AgentDetector;
use crate::agent::adapter::EvidenceDiscoveryPolicy;
use crate::agent::files::fingerprint;
use crate::agent::{
    AgentKind, AgentRecoveryAvailability, AgentState, ObservedProcess, SessionGenerationPolicy,
};
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum EvidencePathSource {
    Discovered,
    Stable,
    Associated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourcedEvidencePath {
    pub path: PathBuf,
    pub source: EvidencePathSource,
}

enum AssociatedSelection {
    Paths(Vec<PathBuf>),
    Rejected,
}

impl AgentDetector {
    pub(super) fn sourced_paths_for_process(
        &self,
        kind: &AgentKind,
        process: &ObservedProcess,
        session_hint: Option<&str>,
    ) -> Result<Vec<SourcedEvidencePath>> {
        let adapter = self
            .registry
            .adapter(kind)
            .expect(crate::agent::constants::validation::REGISTERED_ADAPTER);
        let context = self.context.for_process(adapter, process);
        let Some(process_cwd) = process.cwd.as_deref() else {
            return Ok(Vec::new());
        };
        let stable = self.binding_path_for_process(adapter, process, session_hint);
        let associated =
            match self.associated_paths(adapter, &context, process.evidence_paths.as_deref()) {
                AssociatedSelection::Paths(paths) => paths,
                AssociatedSelection::Rejected => {
                    return Ok(stable
                        .map(|path| SourcedEvidencePath {
                            path,
                            source: EvidencePathSource::Stable,
                        })
                        .into_iter()
                        .collect());
                }
            };
        let can_discover = associated.is_empty()
            && adapter.evidence_discovery_policy() == EvidenceDiscoveryPolicy::ScopedStrongHint
            && session_hint.is_some();
        let discovered = if can_discover {
            adapter
                .discover_evidence(&context, process_cwd)?
                .into_iter()
                .filter(|path| {
                    session_hint
                        .is_some_and(|hint| adapter.evidence_matches_session_hint(path, hint))
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut paths = Vec::new();
        for path in discovered {
            self.push_sourced_path(&mut paths, path, EvidencePathSource::Discovered, process);
        }
        if let Some(path) = stable {
            self.push_sourced_path(&mut paths, path, EvidencePathSource::Stable, process);
        }
        for path in associated {
            self.push_sourced_path(&mut paths, path, EvidencePathSource::Associated, process);
        }
        paths.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(paths)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    #[doc(hidden)]
    pub fn paths_for_process(
        &self,
        kind: &AgentKind,
        process: &ObservedProcess,
    ) -> Result<Vec<PathBuf>> {
        let adapter = self
            .registry
            .adapter(kind)
            .expect(crate::agent::constants::validation::REGISTERED_ADAPTER);
        let session_hint = adapter.process_session_id(process);
        Ok(self
            .sourced_paths_for_process(kind, process, session_hint.as_deref())?
            .into_iter()
            .map(|candidate| candidate.path)
            .collect())
    }

    fn associated_paths(
        &self,
        adapter: &dyn crate::agent::AgentAdapter,
        context: &crate::agent::DetectionContext,
        associated: Option<&[PathBuf]>,
    ) -> AssociatedSelection {
        let paths = associated.unwrap_or_default();
        if paths
            .iter()
            .any(|path| !adapter.owns_evidence_path(path, context))
        {
            return AssociatedSelection::Rejected;
        }
        AssociatedSelection::Paths(
            paths
                .iter()
                .filter(|path| {
                    adapter.is_evidence_candidate_path(path) && fingerprint(path).is_some()
                })
                .cloned()
                .collect(),
        )
    }

    fn push_sourced_path(
        &self,
        paths: &mut Vec<SourcedEvidencePath>,
        path: PathBuf,
        source: EvidencePathSource,
        process: &ObservedProcess,
    ) {
        if source == EvidencePathSource::Discovered
            && self.binding_process_was_reused_by(process)
            && self
                .binding
                .as_ref()
                .is_some_and(|binding| Path::new(&binding.evidence_path) == path)
        {
            return;
        }
        if let Some(existing) = paths.iter_mut().find(|candidate| candidate.path == path) {
            if source > existing.source {
                existing.source = source;
            }
        } else {
            paths.push(SourcedEvidencePath { path, source });
        }
    }

    pub(super) fn binding_path_for_process(
        &self,
        adapter: &dyn crate::agent::AgentAdapter,
        process: &ObservedProcess,
        session_hint: Option<&str>,
    ) -> Option<PathBuf> {
        let binding = self.binding.as_ref()?;
        if binding.kind != adapter.kind() {
            return None;
        }
        let same_process = binding.state != AgentState::Exited
            && binding
                .process_pid
                .zip(binding.process_start_time)
                .is_some_and(|identity| process.identity_matches(identity.0, identity.1));
        let resumes = !self.binding_process_was_reused_by(process)
            && session_hint == Some(binding.provider_session_id.as_str());
        (same_process || resumes).then(|| PathBuf::from(&binding.evidence_path))
    }

    pub(super) fn binding_pid_was_reused(&self, processes: &[ObservedProcess]) -> bool {
        processes
            .iter()
            .any(|process| self.binding_live_process_was_reused_by(process))
    }

    pub(super) fn binding_process_was_reused_by(&self, process: &ObservedProcess) -> bool {
        self.binding.as_ref().is_some_and(|binding| {
            binding.state != AgentState::Exited
                && binding.process_pid == Some(process.pid)
                && binding.process_start_time != Some(process.start_time)
        })
    }

    fn binding_live_process_was_reused_by(&self, process: &ObservedProcess) -> bool {
        let policy = super::transitions::binding_policy(self);
        self.binding.as_ref().is_some_and(|binding| {
            binding.state != AgentState::Exited
                && (policy == SessionGenerationPolicy::AuthoritativeOnly
                    || binding.recovery_availability != AgentRecoveryAvailability::Unavailable)
                && binding.process_pid == Some(process.pid)
                && binding.process_start_time != Some(process.start_time)
        })
    }
}
