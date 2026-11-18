use super::event::{HookEnvelope, HookMetadata};
use super::snapshot::SnapshotViolation;
use super::state::NormalizedLifecycle;

pub(super) fn apply(state: &mut NormalizedLifecycle, envelope: &HookEnvelope) {
    if let Some(cwd) = envelope.metadata.cwd.as_ref() {
        state.current_cwd = Some(cwd.clone());
    }
    if let Some(transcript) = envelope.metadata.transcript_path.as_ref() {
        state.current_transcript_path = Some(transcript.clone());
    }
}

pub(super) fn validate(state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
    let metadata = HookMetadata {
        cwd: state.current_cwd.clone(),
        transcript_path: state.current_transcript_path.clone(),
    };
    if !metadata.is_valid()
        || !retained_value_matches(
            state,
            |metadata| metadata.cwd.as_ref(),
            state.current_cwd.as_ref(),
        )
        || !retained_value_matches(
            state,
            |metadata| metadata.transcript_path.as_ref(),
            state.current_transcript_path.as_ref(),
        )
    {
        return Err(SnapshotViolation::InvalidState);
    }
    Ok(())
}

fn retained_value_matches<'a, F>(
    state: &'a NormalizedLifecycle,
    select: F,
    current: Option<&std::path::PathBuf>,
) -> bool
where
    F: Fn(&'a HookMetadata) -> Option<&'a std::path::PathBuf>,
{
    state
        .history
        .values()
        .filter_map(|envelope| select(&envelope.metadata))
        .next_back()
        .is_none_or(|retained| Some(retained) == current)
}
