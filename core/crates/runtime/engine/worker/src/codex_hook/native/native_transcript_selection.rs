use super::{CodexAdapter, NATIVE_EVIDENCE};
use crate::agent::{AgentAdapter, DetectionContext, ObservedProcess};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;
use std::path::Path;

const UNIQUE_TRANSCRIPT_COUNT: usize = 1;

pub(super) fn select<'a>(
    agent: &'a ObservedProcess,
    explicit: Option<&'a Path>,
    session_id: &str,
    context: &DetectionContext,
) -> Result<&'a Path> {
    let observed = agent.evidence_paths.as_ref().context(NATIVE_EVIDENCE)?;
    if let Some(path) = explicit {
        ensure!(
            observed.iter().any(|observed| observed == path),
            NATIVE_EVIDENCE
        );
        return Ok(path);
    }
    let candidates = observed
        .iter()
        .map(|path| path.as_path())
        .filter(|path| {
            CodexAdapter.is_evidence_candidate_path(path)
                && CodexAdapter.evidence_matches_session_hint(path, session_id)
                && CodexAdapter.owns_evidence_path(path, context)
        })
        .collect::<BTreeSet<_>>();
    ensure!(candidates.len() == UNIQUE_TRANSCRIPT_COUNT, NATIVE_EVIDENCE);
    candidates.first().copied().context(NATIVE_EVIDENCE)
}
