use super::{CodexHookPolicy, CodexNativeRequest, NATIVE_PROCESS};
use crate::agent::codex::{AuthoritativeCodexEvidence, CodexAdapter};
use crate::agent::hooks::{Generation, HookIdentity, ProcessIdentity};
use crate::agent::{AgentAdapter, AgentKind, DetectionContext, ObservedProcess};
use crate::codex_hooks::CodexAdmissionContext;
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(super) const NATIVE_EVIDENCE: &str =
    "native Codex hook session evidence does not match the exact agent";

#[path = "native_transcript_selection.rs"]
mod transcript;

pub(super) fn capture(
    agent: &ObservedProcess,
    request: &CodexNativeRequest,
    policy: &CodexHookPolicy,
) -> Result<(CodexAdmissionContext, Option<AuthoritativeCodexEvidence>)> {
    let environment = agent.environment.as_ref().context(NATIVE_PROCESS)?;
    ensure!(
        environment.get(crate::codex_hooks::TERMINAL_ENV)
            == Some(&policy.terminal_id().to_string())
            && environment.get(crate::codex_hooks::RUNTIME_ENV)
                == Some(&policy.runtime_epoch().to_string()),
        NATIVE_PROCESS
    );
    let configured = environment
        .get(crate::agent::codex::constants::DATA_HOME_ENV)
        .map(PathBuf::from)
        .or_else(|| {
            environment
                .get(crate::agent::constants::HOME_ENV)
                .map(|home| {
                    Path::new(home).join(crate::agent::codex::constants::DEFAULT_DATA_HOME_DIR)
                })
        })
        .context(NATIVE_EVIDENCE)?;
    let data_home = configured.canonicalize().context(NATIVE_EVIDENCE)?;
    ensure!(
        request
            .data_home_candidates
            .iter()
            .any(|path| path.canonicalize().is_ok_and(|path| path == data_home)),
        NATIVE_EVIDENCE
    );
    let context = DetectionContext::new(
        policy.terminal_id(),
        request.cwd().to_path_buf(),
        [(AgentKind::Codex, data_home.clone())],
    );
    let probe_identity = identity(policy, agent);
    let envelope = request.envelope(&probe_identity)?;
    let transcript_path = envelope.metadata.transcript_path.as_deref();
    let evidence = if request.is_session_start() {
        session_start_evidence(agent, request, transcript_path, &context, &data_home)?
    } else {
        let transcript = transcript::select(
            agent,
            transcript_path,
            request.provider_session_id(),
            &context,
        )?;
        Some(verified(agent, request, transcript, &data_home)?)
    };
    let executable = Path::new(&agent.executable)
        .canonicalize()
        .context(NATIVE_PROCESS)?;
    ensure!(
        executable.as_os_str() == Path::new(&agent.executable).as_os_str(),
        NATIVE_PROCESS
    );
    Ok((
        CodexAdmissionContext {
            identity: probe_identity,
            provider_version: crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            cwd: request.cwd().to_path_buf(),
            data_home,
            hook_executable: policy.hook_executable().to_path_buf(),
            real_executable: executable,
        },
        evidence,
    ))
}

fn verified(
    agent: &ObservedProcess,
    request: &CodexNativeRequest,
    transcript: &Path,
    data_home: &Path,
) -> Result<AuthoritativeCodexEvidence> {
    let proof = CodexAdapter
        .inspect_authoritative_evidence_with_hook(transcript, request.cwd(), data_home, || {})?
        .context(NATIVE_EVIDENCE)?;
    let evidence = CodexAdapter
        .contextualize_evidence(agent, proof.evidence().clone())?
        .context(NATIVE_EVIDENCE)?;
    ensure!(
        evidence.provider_session_id == request.provider_session_id()
            && evidence.cwd == request.cwd()
            && evidence.data_home == data_home,
        NATIVE_EVIDENCE
    );
    Ok(proof)
}

/// Codex writes `session_meta` after the SessionStart callback returns, so the
/// first callback can observe the rollout path before its contents are
/// authoritative. Keep the claim bound to the observed process and the
/// announced transcript path, and let the next callback supply evidence.
fn session_start_evidence(
    agent: &ObservedProcess,
    request: &CodexNativeRequest,
    transcript_path: Option<&Path>,
    context: &DetectionContext,
    data_home: &Path,
) -> Result<Option<AuthoritativeCodexEvidence>> {
    match transcript::select(
        agent,
        transcript_path,
        request.provider_session_id(),
        context,
    ) {
        Ok(transcript) => match verified(agent, request, transcript, data_home) {
            Ok(proof) => Ok(Some(proof)),
            Err(error) => {
                provisional(agent, request, Some(transcript), context, data_home).map_err(|_| error)
            }
        },
        Err(error) => {
            provisional(agent, request, transcript_path, context, data_home).map_err(|_| error)
        }
    }
}

/// Keep the claim bound to the observed process, the announced transcript path
/// and the agent working directory until authoritative evidence exists.
fn provisional(
    agent: &ObservedProcess,
    request: &CodexNativeRequest,
    transcript_path: Option<&Path>,
    context: &DetectionContext,
    data_home: &Path,
) -> Result<Option<AuthoritativeCodexEvidence>> {
    ensure!(agent.evidence_paths.is_some(), NATIVE_EVIDENCE);
    let path = transcript_path.context(NATIVE_EVIDENCE)?;
    ensure!(
        CodexAdapter.is_evidence_candidate_path(path)
            && CodexAdapter.owns_evidence_path(path, context)
            && CodexAdapter.evidence_matches_session_hint(path, request.provider_session_id()),
        NATIVE_EVIDENCE
    );
    if let Some(proof) = CodexAdapter.inspect_authoritative_evidence_with_hook(
        path,
        request.cwd(),
        data_home,
        || {},
    )? {
        return verified(agent, request, path, data_home).map(Some);
    }
    let agent_cwd = agent.cwd.as_deref().context(NATIVE_EVIDENCE)?;
    ensure!(
        agent_cwd.canonicalize().ok() == request.cwd().canonicalize().ok(),
        NATIVE_EVIDENCE
    );
    Ok(None)
}

fn identity(policy: &CodexHookPolicy, agent: &ObservedProcess) -> HookIdentity {
    let process_key = (u128::from(agent.pid) << u64::BITS) | u128::from(agent.start_time);
    HookIdentity {
        terminal_id: policy.terminal_id(),
        runtime_epoch: policy.runtime_epoch(),
        invocation_id: Uuid::from_u128(process_key),
        hook_token: Uuid::new_v4(),
        process: ProcessIdentity::new(agent.pid, agent.start_time),
        generation: Generation::INITIAL,
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../test_support/unit/codex_hook/native/native_admission_evidence.rs"]
mod test_support;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/native/native_admission_evidence_tests.rs"]
mod tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/native/native_admission_exec_tests.rs"]
mod exec_tests;
