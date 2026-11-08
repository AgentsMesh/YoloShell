use super::*;
use crate::claude_hooks::{ClaudeLifecycleEvent, ClaudeNativeRequest};
use anyhow::{Result, ensure};

pub(super) fn event(
    request: &ClaudeNativeRequest,
    evidence: &crate::claude::AuthoritativeClaudeEvidence,
) -> Result<()> {
    let info = evidence.evidence();
    ensure!(
        request.is_valid()
            && super::super::validation::canonical_session_id(request.event.provider_session_id())
            && info.provider_session_id == request.event.provider_session_id()
            && super::super::validation::valid_transcript(
                &info.data_home,
                &info.path,
                &info.provider_session_id
            )
            && evidence.verify()?,
        INVALID
    );
    match &request.event {
        ClaudeLifecycleEvent::SessionStart {
            cwd,
            transcript_path,
            ..
        }
        | ClaudeLifecycleEvent::CwdChanged {
            cwd,
            transcript_path,
            ..
        } => {
            ensure!(*cwd == info.cwd && *transcript_path == info.path, INVALID);
        }
        _ => {}
    }
    Ok(())
}

impl NativeGenerations {
    /// Records written before interaction support are still valid replay
    /// history; normalize their request schema before strict validation.
    pub(super) fn migrate_replay_schema(&mut self) {
        for generation in &mut self.invocations {
            for request in &mut generation.replay_events {
                if request.schema_version == crate::claude_hooks::PREVIOUS_CLAUDE_NATIVE_SCHEMA_VERSION
                {
                    request.schema_version = crate::claude_hooks::CLAUDE_NATIVE_SCHEMA_VERSION;
                }
            }
        }
    }
    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == SCHEMA_VERSION
                && !self.active_invocation.is_nil()
                && !self.invocations.is_empty()
                && self.invocations.len() <= super::super::constants::MAX_INVOCATION_AUTHORITIES
                && self
                    .invocations
                    .iter()
                    .any(|item| item.invocation_id == self.active_invocation),
            INVALID
        );
        let mut scopes = Vec::new();
        let mut ids = std::collections::HashSet::new();
        let session_id = self
            .invocations
            .first()
            .map(|item| item.scope.terminal_id);
        for record in &self.invocations {
            ensure!(
                record.schema_version == SCHEMA_VERSION
                    && record.scope.is_valid()
                    && Some(record.scope.terminal_id) == session_id
                    && record.worker.pid != u32::default()
                    && record.worker.start_time != u64::default()
                    && !record.boot_id.is_empty()
                    && !record.invocation_id.is_nil()
                    && ids.insert(record.invocation_id)
                    && !scopes.contains(&record.scope)
                    && record.revision >= INITIAL_REVISION
                    && record.last_event_sequence
                        >= crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE
                    && record.replay_events.len() <= super::super::constants::MAX_REPLAY_EVENT_IDS
                    && !record.replay_events.is_empty(),
                INVALID
            );
            let binding = &record.binding;
            ensure!(
                binding.terminal_id == record.scope.terminal_id
                    && binding.kind == crate::agent::AgentKind::ClaudeCode
                    && binding.executable == crate::claude::constants::EXECUTABLE
                    && binding.schema_version == crate::agent::constants::BINDING_SCHEMA_VERSION
                    && binding.state == record.activity
                    && binding.detected_at_ms >= i64::default()
                    && binding.updated_at_ms >= binding.detected_at_ms
                    && binding.process_pid == Some(record.scope.agent.pid)
                    && binding.process_start_time == Some(record.scope.agent.start_time)
                    && super::super::validation::canonical_session_id(&binding.provider_session_id)
                    && super::super::validation::valid_transcript(
                        std::path::Path::new(&binding.data_home),
                        std::path::Path::new(&binding.evidence_path),
                        &binding.provider_session_id
                    ),
                INVALID
            );
            let mut sequence = None;
            let mut event_ids = std::collections::HashSet::new();
            for request in &record.replay_events {
                ensure!(
                    request.is_valid()
                        && request.scope == record.scope
                        && sequence.is_none_or(|previous: u64| previous.checked_add(INCREMENT)
                            == Some(request.event_sequence))
                        && event_ids.insert(request.event.event_id()),
                    INVALID
                );
                sequence = Some(request.event_sequence);
            }
            ensure!(sequence == Some(record.last_event_sequence), INVALID);
            super::replay_validation::validate(record)?;
            scopes.push(record.scope);
        }
        Ok(())
    }
}
