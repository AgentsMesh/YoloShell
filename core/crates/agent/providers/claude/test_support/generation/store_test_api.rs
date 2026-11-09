use super::super::types::{
    AuthorityAuthentication, ClaudeGenerationAuthority, GenerationApplyDisposition, GenerationEvent,
};
use super::ClaudeGenerationStore;
use crate::namespace_lease::NamespaceLease;
use anyhow::Result;
use std::path::Path;

impl ClaudeGenerationStore {
    pub fn begin_in_namespace(
        &self,
        namespace: &NamespaceLease,
        begin: &super::super::GenerationBegin,
    ) -> Result<ClaudeGenerationAuthority> {
        self.begin_foreground_in_namespace(namespace, begin, false)
    }

    #[doc(hidden)]
    pub fn apply_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        event: &GenerationEvent,
    ) -> Result<(ClaudeGenerationAuthority, GenerationApplyDisposition)> {
        let current = self.authenticated(namespace, authentication)?;
        let sequence = current
            .replay_events
            .iter()
            .find(|applied| applied.event == *event)
            .map(|applied| applied.sequence)
            .unwrap_or_else(|| current.last_event_sequence.saturating_add(1));
        let identity = event_path(event)
            .map(|path| {
                super::super::super::authoritative_evidence::named_transcript_identity(
                    &current.data_home,
                    path,
                )
            })
            .transpose()?
            .flatten();
        let (next, disposition) = self.apply_expected_in_namespace(
            namespace,
            authentication,
            &current,
            sequence,
            event,
            identity,
        )?;
        let next = if disposition == GenerationApplyDisposition::Applied {
            self.commit_binding_publication_in_namespace(namespace, authentication, &next)?
        } else {
            next
        };
        Ok((next, disposition))
    }
}

fn event_path(event: &GenerationEvent) -> Option<&Path> {
    match event {
        GenerationEvent::SessionStart {
            transcript_path, ..
        }
        | GenerationEvent::CwdChanged {
            transcript_path, ..
        } => Some(transcript_path),
        GenerationEvent::StateChanged { .. } | GenerationEvent::SessionEnd { .. } => None,
    }
}
