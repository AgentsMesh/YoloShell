use crate::agent::claude::generation::native::ClaudeNativeGeneration;
use crate::agent::claude::{AuthoritativeClaudeEvidence, ClaudeAdapter};
use crate::claude_hooks::{ClaudeLifecycleEvent, ClaudeNativeRequest};
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

const INVALID: &str = "Claude native callback evidence does not match its agent process";

pub(super) fn inspect(
    admitted: &super::native_admission::Admission,
    request: &ClaudeNativeRequest,
    previous: Option<&ClaudeNativeGeneration>,
) -> Result<AuthoritativeClaudeEvidence> {
    let environment = admitted.agent.environment.as_ref().context(INVALID)?;
    let data_home = environment
        .get(crate::agent::claude::constants::DATA_HOME_ENV)
        .map(PathBuf::from)
        .or_else(|| {
            environment
                .get(crate::agent::constants::HOME_ENV)
                .map(|home| {
                    Path::new(home).join(crate::agent::claude::constants::DEFAULT_DATA_HOME_DIR)
                })
        })
        .context(INVALID)?
        .canonicalize()?;
    let (cwd, transcript) = match &request.event {
        ClaudeLifecycleEvent::SessionStart {
            cwd,
            transcript_path,
            ..
        }
        | ClaudeLifecycleEvent::CwdChanged {
            cwd,
            transcript_path,
            ..
        } => (cwd.clone(), transcript_path.clone()),
        _ => {
            let record = previous.context(INVALID)?;
            (
                PathBuf::from(&record.binding.cwd),
                PathBuf::from(&record.binding.evidence_path),
            )
        }
    };
    ensure!(
        cwd.is_absolute()
            && cwd.canonicalize()? == cwd
            && transcript.is_absolute()
            && transcript.canonicalize()? == transcript
            && admitted.agent.cwd.as_ref().is_some_and(|current| current
                .canonicalize()
                .ok()
                .as_ref()
                == Some(&cwd)),
        INVALID
    );
    if let Some(previous) = previous {
        ensure!(
            !previous.revoked && Path::new(&previous.binding.data_home) == data_home,
            INVALID
        );
    }
    let evidence = ClaudeAdapter
        .inspect_authoritative_evidence_with_hook(&transcript, &cwd, &data_home, || {})?
        .context(INVALID)?;
    ensure!(
        evidence.evidence().provider_session_id == request.event.provider_session_id()
            && evidence.evidence().cwd == cwd
            && evidence.evidence().path == transcript
            && evidence.evidence().data_home == data_home,
        INVALID
    );
    Ok(evidence)
}
