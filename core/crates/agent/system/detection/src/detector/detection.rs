use super::AgentDetector;
use super::candidate::{CandidateAuthority, ProcessOutcome, retain_strongest, unique_outcome};
use super::evidence::EvidencePathSource;
use crate::agent::adapter::SessionGenerationPolicy;
use crate::agent::{AgentEvidence, AgentKind, AgentState, DetectionResult, ObservedProcess};
use anyhow::Result;

impl AgentDetector {
    pub fn detect(&mut self, processes: &[ObservedProcess]) -> Result<DetectionResult> {
        let mut candidates = Vec::new();
        let mut invalidates_binding = false;
        for adapter in self.registry.adapters() {
            for process in processes {
                if !eligible_process(adapter, process) {
                    continue;
                }
                let outcome = self.inspect_process(adapter.kind(), process)?;
                candidates.extend(outcome.candidates);
                invalidates_binding |= outcome.invalidates_binding;
            }
        }
        retain_strongest(&mut candidates);
        if process_identities_conflict(processes) {
            self.consecutive_misses = 0;
            return Ok(self.mark_unknown());
        }
        if self.binding_pid_was_reused(processes) {
            self.consecutive_misses = 0;
            return Ok(self.mark_observed_exit());
        }
        if candidates.len() != 1 || invalidates_binding {
            return Ok(if invalidates_binding {
                self.consecutive_misses = 0;
                self.mark_unknown()
            } else {
                self.note_missing()
            });
        }
        let candidate = candidates
            .pop()
            .expect(crate::agent::constants::validation::SINGLE_CANDIDATE);
        self.consecutive_misses = 0;
        Ok(self.apply_candidate(candidate.kind, &candidate.process, candidate.evidence))
    }

    fn inspect_process(
        &self,
        kind: AgentKind,
        process: &ObservedProcess,
    ) -> Result<ProcessOutcome> {
        let adapter = self
            .registry
            .adapter(&kind)
            .expect(crate::agent::constants::validation::REGISTERED_ADAPTER);
        if !adapter.process_session_arguments_valid(process) {
            return Ok(ProcessOutcome::default());
        }
        let context = self.context.for_process(adapter, process);
        let Some(cwd) = process.cwd.as_deref() else {
            return Ok(ProcessOutcome::default());
        };
        let hint = adapter.process_session_id(process);
        let bound_process = self.binding_matches_process(&kind, process);
        let paths = self.sourced_paths_for_process(&kind, process, hint.as_deref())?;
        let mut associated = Vec::new();
        let mut discovered = Vec::new();
        let mut stable = None;
        for sourced in paths {
            let Some(mut evidence) =
                adapter.inspect_evidence_in_context(&sourced.path, cwd, &context)?
            else {
                continue;
            };
            let Some(contextualized) = adapter.contextualize_evidence(process, evidence)? else {
                continue;
            };
            evidence = contextualized;
            if adapter.session_generation_policy() == SessionGenerationPolicy::HookRequired {
                evidence.state = AgentState::Unknown;
            }
            match sourced.source {
                EvidencePathSource::Associated => associated.push(evidence),
                EvidencePathSource::Stable => stable = Some(evidence),
                EvidencePathSource::Discovered
                    if hint.as_deref() == Some(evidence.provider_session_id.as_str()) =>
                {
                    discovered.push(evidence)
                }
                EvidencePathSource::Discovered => {}
            }
        }
        if !bound_process
            && hint.as_deref().is_some_and(|expected| {
                associated
                    .iter()
                    .any(|evidence| evidence.provider_session_id != expected)
            })
        {
            return Ok(ProcessOutcome::default());
        }
        Ok(self.select_process_candidate(
            kind,
            process,
            associated,
            discovered,
            stable,
            bound_process,
        ))
    }

    fn select_process_candidate(
        &self,
        kind: AgentKind,
        process: &ObservedProcess,
        associated: Vec<AgentEvidence>,
        discovered: Vec<AgentEvidence>,
        stable: Option<AgentEvidence>,
        bound_process: bool,
    ) -> ProcessOutcome {
        if !associated.is_empty() {
            return unique_outcome(
                kind,
                process,
                associated,
                bound_process,
                CandidateAuthority::ProcessAssociated,
            );
        }
        if let Some(stable) = stable {
            let uncertain = self
                .binding
                .as_ref()
                .is_some_and(|binding| binding.state == AgentState::Unknown);
            if uncertain || !discovered.is_empty() {
                return ProcessOutcome {
                    candidates: Vec::new(),
                    invalidates_binding: bound_process,
                };
            }
            return unique_outcome(
                kind,
                process,
                vec![stable],
                false,
                CandidateAuthority::Contextual,
            );
        }
        unique_outcome(
            kind,
            process,
            discovered,
            false,
            CandidateAuthority::Contextual,
        )
    }

    fn binding_matches_process(&self, kind: &AgentKind, process: &ObservedProcess) -> bool {
        self.binding.as_ref().is_some_and(|binding| {
            binding.kind == *kind
                && binding
                    .process_pid
                    .zip(binding.process_start_time)
                    .is_some_and(|identity| process.identity_matches(identity.0, identity.1))
        })
    }
}

fn eligible_process(adapter: &dyn crate::agent::AgentAdapter, process: &ObservedProcess) -> bool {
    process.is_foreground()
        && process.start_time != 0
        && process.cwd.is_some()
        && adapter.matches_process(process)
}

fn process_identities_conflict(processes: &[ObservedProcess]) -> bool {
    let mut remaining = processes;
    while let Some((process, others)) = remaining.split_first() {
        if others
            .iter()
            .any(|other| process.pid == other.pid && process.start_time != other.start_time)
        {
            return true;
        }
        remaining = others;
    }
    false
}
