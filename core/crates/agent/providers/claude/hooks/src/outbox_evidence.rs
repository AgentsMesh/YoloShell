use super::*;
use anyhow::{Context, Result, ensure};

pub(super) fn transcript_identity(
    event: &ClaudeLifecycleEvent,
) -> Result<Option<crate::paths::identity::ObjectIdentity>> {
    let path = match event {
        ClaudeLifecycleEvent::SessionStart {
            transcript_path, ..
        }
        | ClaudeLifecycleEvent::CwdChanged {
            transcript_path, ..
        } => transcript_path,
        ClaudeLifecycleEvent::StateChanged { .. }
        | ClaudeLifecycleEvent::SessionEnd { .. }
        | ClaudeLifecycleEvent::InteractionRequested { .. } => {
            return Ok(None);
        }
    };
    let data_home = std::env::var_os(crate::agent::claude::constants::DATA_HOME_ENV)
        .map(PathBuf::from)
        .context(outbox_error::DATA_HOME_MISSING)?;
    let identity = crate::agent::claude::durable_transcript_identity(&data_home, path)?;
    ensure!(
        identity.is_some(),
        outbox_error::TRANSCRIPT_IDENTITY_MISSING
    );
    Ok(identity)
}
